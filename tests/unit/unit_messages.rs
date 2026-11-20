use super::*;
use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    // Use the per-thread database URL rather than a process-global env var.
    // Each test runs on its own thread, so this keeps tests isolated and idempotent.
    crate::db::set_db_url(db_path.to_str().expect("db path"));
    crate::db::init_database().expect("init db");
    f();
    crate::db::release_db_lock();
    crate::db::reset_db_url();
}

#[test]
#[serial(db)]
fn save_and_mark_message_sent() {
    with_test_db(|| {
        let msg = save_message("hello", None, "topic-a", false, None).expect("save");
        let loaded = load_messages("topic-a", 100).expect("load");
        assert!(loaded.iter().any(|m| m.id == msg.id));
        mark_message_sent(msg.id).expect("mark sent");
        let after = load_messages("topic-a", 100).expect("load after");
        assert!(after.iter().any(|m| m.id == msg.id && m.sent == 1));
    });
}

#[test]
#[serial(db)]
fn direct_message_roundtrip() {
    with_test_db(|| {
        let saved =
            save_message("dm", Some("peer-a"), "topic-a", true, Some("peer-b")).expect("save");
        let dms = load_direct_messages("peer-b", 20).expect("load direct");
        let found = dms.iter().find(|m| m.id == saved.id).expect("found");
        assert_eq!(found.sent, 0); // saved unsent
    });
}

#[test]
#[serial(db)]
fn save_message_with_meta_persists_fields() {
    with_test_db(|| {
        let meta = MessageMeta {
            sender_nickname: Some("alice".to_string()),
            msg_id: Some("msg-1".to_string()),
            sent_at: Some(123.5),
            attachment: None,
            reply_to: None,
            encrypted: false,
        };
        let saved = save_message_with_meta("payload", Some("peer-x"), "topic-x", false, None, meta)
            .expect("save with meta");
        assert_eq!(saved.sender_nickname.as_deref(), Some("alice"));
        assert_eq!(saved.msg_id.as_deref(), Some("msg-1"));
        assert_eq!(saved.sent_at, Some(123.5));
    });
}

#[test]
#[serial(db)]
fn save_receipt_upserts_same_key() {
    with_test_db(|| {
        save_receipt("msg-2", "peer-z", 1, 10.0).expect("save receipt");
        save_receipt("msg-2", "peer-z", 1, 20.0).expect("upsert receipt");
        let receipts = load_receipts().expect("load receipts");
        let matching: Vec<_> = receipts
            .into_iter()
            .filter(|r| r.msg_id == "msg-2" && r.peer_id == "peer-z" && r.kind == 1)
            .collect();
        assert_eq!(matching.len(), 1);
        assert!((matching[0].confirmed_at - 20.0).abs() < f64::EPSILON);
    });
}

#[test]
#[serial(db)]
fn load_messages_filters_broadcast_and_applies_limit() {
    with_test_db(|| {
        let b1 = save_message("b1", Some("peer-a"), "topic-l", false, None).expect("save b1");
        let _b2 = save_message("b2", Some("peer-a"), "topic-l", false, None).expect("save b2");
        let _dm =
            save_message("dm", Some("peer-a"), "topic-l", true, Some("peer-b")).expect("save dm");

        let loaded = load_messages("topic-l", 1).expect("load messages");
        assert_eq!(loaded.len(), 1);
        assert_ne!(loaded[0].id, b1.id);
        assert_eq!(loaded[0].is_direct, 0);
    });
}

#[test]
#[serial(db)]
fn save_message_reports_context_on_insert_failure() {
    with_test_db(|| {
        let conn = &mut crate::sqlite_connect().expect("connect");
        diesel::sql_query("DROP TABLE messages")
            .execute(conn)
            .expect("drop messages");

        let err = save_message("boom", None, "topic-e", false, None).expect_err("must fail");
        let rendered = format!("{err:?}");
        assert!(rendered.contains("Failed to save message"));
        assert!(rendered.contains("topic-e"));
    });
}

#[test]
#[serial(db)]
fn mark_message_sent_nonexistent() {
    with_test_db(|| {
        let result = mark_message_sent(99999);
        assert!(
            result.is_ok(),
            "Should handle non-existent message gracefully"
        );
    });
}

#[test]
#[serial(db)]
fn load_direct_messages_filters_correctly() {
    with_test_db(|| {
        let _dm1 =
            save_message("dm1", Some("peer-x"), "topic-1", true, Some("peer-x")).expect("save dm1");
        let _dm2 =
            save_message("dm2", Some("peer-y"), "topic-1", true, Some("peer-y")).expect("save dm2");
        let _broadcast =
            save_message("bcast", Some("peer-x"), "topic-1", false, None).expect("save broadcast");

        let dms_with_x = load_direct_messages("peer-x", 100).expect("load dms");
        assert_eq!(dms_with_x.len(), 1, "Should load only DMs with peer-x");
        assert_eq!(dms_with_x[0].content, "dm1");
    });
}

#[test]
#[serial(db)]
fn load_messages_respects_limit() {
    with_test_db(|| {
        for i in 0..10 {
            let _ = save_message(
                &format!("msg-{i}"),
                Some("peer-a"),
                "topic-limit",
                false,
                None,
            );
        }

        let limited = load_messages("topic-limit", 3).expect("load with limit");
        assert!(limited.len() <= 3, "Should respect limit parameter");
    });
}

#[test]
#[serial(db)]
fn get_all_peer_stats_aggregates_across_peers_in_one_pass() {
    with_test_db(|| {
        // Real storage convention: outbound DMs have `peer_id = NULL` and
        // `target_peer = recipient`; inbound DMs set *both* columns to the
        // sender. peer-a sends us 2 DMs and receives 1 from us (3 total), while
        // peer-b sends 1 and receives 1 (2 total). We sent broadcasts to
        // peer-b (×1) and peer-c (×2), each recorded in `broadcast_recipients`.
        crate::save_peer("peer-a", &[]).expect("save a");
        crate::save_peer("peer-b", &[]).expect("save b");
        crate::save_peer("peer-c", &[]).expect("save c");

        // Inbound from peer-a (both columns set to the sender) — ×2.
        let _ = save_message("a-in-1", Some("peer-a"), "t", true, Some("peer-a")).expect("ai1");
        let _ = save_message("a-in-2", Some("peer-a"), "t", true, Some("peer-a")).expect("ai2");
        // Outbound to peer-a (peer_id NULL, target = recipient).
        let _ = save_message("to-a", None, "t", true, Some("peer-a")).expect("ta");
        // Inbound from peer-b + outbound to peer-b.
        let _ = save_message("b-in", Some("peer-b"), "t", true, Some("peer-b")).expect("bi");
        let _ = save_message("to-b", None, "t", true, Some("peer-b")).expect("tb");
        // Inbound broadcast from peer-a: never counted as a DM or a sent-to.
        let _ =
            save_message("bcast-from-a", Some("peer-a"), "t", false, None).expect("bcast from a");
        crate::peers::record_broadcast_recipients("m1", &["peer-b".to_string()]).expect("rec b");
        crate::peers::record_broadcast_recipients("m2", &["peer-c".to_string()]).expect("rec c");
        crate::peers::record_broadcast_recipients("m3", &["peer-c".to_string()]).expect("rec c2");

        let all = get_all_peer_stats().expect("all stats");

        // peer-a: 2 received + 1 sent = 3; no broadcasts *sent to* a.
        let a = all.get("peer-a").expect("peer-a present");
        assert_eq!(a.dm_count, 3);
        assert_eq!(a.broadcast_sent_to_peer, 0);

        // peer-b: 1 received + 1 sent = 2; 1 broadcast we sent b.
        let b = all.get("peer-b").expect("peer-b present");
        assert_eq!(b.dm_count, 2);
        assert_eq!(b.broadcast_sent_to_peer, 1);

        // peer-c: no DMs at all; 2 broadcasts we sent c.
        let c = all.get("peer-c").expect("peer-c present");
        assert_eq!(c.dm_count, 0);
        assert_eq!(c.broadcast_sent_to_peer, 2);

        // The inbound broadcast from peer-a never attributes a *sent-to* count.
        assert!(!all.contains_key("peer-a") || all["peer-a"].broadcast_sent_to_peer == 0);
    });
}

#[test]
#[serial(db)]
fn dm_stats_count_inbound_dm_once_even_with_both_columns_set() {
    with_test_db(|| {
        // Regression test: inbound DMs store `peer_id = target_peer = sender`,
        // which used to double-count under a UNION of the two columns.
        crate::save_peer("peer-a", &[]).expect("save a");
        let _ = save_message("in", Some("peer-a"), "t", true, Some("peer-a")).expect("inbound");
        let _ = save_message("from-me", None, "t", true, Some("peer-a")).expect("outbound");

        let all = get_all_peer_stats().expect("all stats");
        let a = all.get("peer-a").expect("peer-a present");
        assert_eq!(
            a.dm_count, 2,
            "one inbound + one outbound DM each count once"
        );
    });
}

#[test]
#[serial(db)]
fn confirmed_message_ids_matches_receipts() {
    with_test_db(|| {
        // No ids at all short-circuits without touching the database.
        assert!(confirmed_message_ids(&[]).expect("empty input").is_empty());

        save_message_with_meta(
            "hi",
            None,
            crate::CHAT_TOPIC,
            false,
            None,
            MessageMeta {
                msg_id: Some("msg-a".to_string()),
                sent_at: Some(1.0),
                ..MessageMeta::default()
            },
        )
        .expect("save");

        // An outgoing message with no receipt is not yet confirmed.
        assert!(
            confirmed_message_ids(&["msg-a".to_string(), "msg-none".to_string()])
                .expect("no receipts")
                .is_empty()
        );

        save_receipt("msg-a", "peer-x", 0, 10.0).expect("receipt");
        let confirmed =
            confirmed_message_ids(&["msg-a".to_string(), "msg-none".to_string()]).expect("ids");
        assert_eq!(confirmed.len(), 1);
        assert!(confirmed.contains("msg-a"));
        assert!(!confirmed.contains("msg-none"));

        // A second peer acknowledging the same message stays one entry.
        save_receipt("msg-a", "peer-y", 0, 11.0).expect("second receipt");
        assert_eq!(
            confirmed_message_ids(&["msg-a".to_string()])
                .expect("deduped")
                .len(),
            1
        );
    });
}

#[test]
#[serial(db)]
fn message_is_dm_resolves_kind_and_unknown_ids() {
    with_test_db(|| {
        let dm = save_message_with_meta(
            "dm",
            None,
            "t",
            true,
            Some("peer-b"),
            MessageMeta {
                msg_id: Some("dm-id".to_string()),
                ..MessageMeta::default()
            },
        )
        .expect("save dm");
        mark_message_sent(dm.id).expect("mark dm sent");
        let bc = save_message_with_meta(
            "bc",
            None,
            "t",
            false,
            None,
            MessageMeta {
                msg_id: Some("bc-id".to_string()),
                ..MessageMeta::default()
            },
        )
        .expect("save broadcast");
        mark_message_sent(bc.id).expect("mark broadcast sent");
        // Inbound rows now persist wire ids too. An echoed id must not flip a
        // receipt's kind, so only sent rows participate in the lookup.
        save_message_with_meta(
            "echo",
            Some("peer-b"),
            "t",
            false,
            None,
            MessageMeta {
                msg_id: Some("dm-id".to_string()),
                ..MessageMeta::default()
            },
        )
        .expect("save inbound echo");

        assert_eq!(message_is_dm("dm-id").expect("dm"), Some(true));
        assert_eq!(message_is_dm("bc-id").expect("broadcast"), Some(false));
        assert_eq!(message_is_dm("missing-id").expect("missing"), None);
    });
}

#[test]
#[serial(db)]
fn mark_message_deleted_hides_only_sent_rows_from_broadcast_and_dm_history() {
    with_test_db(|| {
        let topic = "delete-topic";
        let outgoing =
            save_message("outgoing", None, topic, false, None).expect("save outgoing broadcast");
        assert_eq!(outgoing.deleted_at, None, "new rows start visible");
        assert!(
            !mark_message_deleted(outgoing.id).expect("unsent row is not deletable"),
            "only rows we actually sent can be hidden"
        );
        mark_message_sent(outgoing.id).expect("mark broadcast sent");

        let incoming =
            save_message("incoming", Some("peer-a"), topic, false, None).expect("save incoming");
        let dm_out =
            save_message("dm-out", None, topic, true, Some("peer-b")).expect("save outgoing dm");
        mark_message_sent(dm_out.id).expect("mark dm sent");
        let dm_in = save_message("dm-in", Some("peer-a"), topic, true, Some("peer-a"))
            .expect("save incoming dm");

        assert!(mark_message_deleted(outgoing.id).expect("delete broadcast"));
        assert!(mark_message_deleted(dm_out.id).expect("delete outgoing DM"));
        assert!(
            !mark_message_deleted(outgoing.id).expect("second delete is a no-op"),
            "tombstones are idempotent"
        );

        let conn = &mut crate::sqlite_connect().expect("connect");
        let stamped = crate::generated::schema::messages::table
            .filter(crate::generated::schema::messages::id.eq(outgoing.id))
            .select(crate::generated::schema::messages::deleted_at)
            .first::<Option<f64>>(conn)
            .expect("read tombstone");
        assert!(
            stamped.is_some_and(|at| at > 0.0),
            "deletion stamps a Unix timestamp: {stamped:?}"
        );
        let untouched = crate::generated::schema::messages::table
            .filter(crate::generated::schema::messages::id.eq(incoming.id))
            .select(crate::generated::schema::messages::deleted_at)
            .first::<Option<f64>>(conn)
            .expect("read incoming row");
        assert_eq!(untouched, None, "incoming rows are never tombstoned");

        let broadcasts = load_messages(topic, 100).expect("broadcast history");
        let broadcast_ids: Vec<i32> = broadcasts.iter().map(|m| m.id).collect();
        assert!(
            !broadcast_ids.contains(&outgoing.id),
            "hidden broadcast gone"
        );
        assert!(broadcast_ids.contains(&incoming.id), "other rows remain");

        let dms = load_direct_messages("peer-b", 100).expect("dm history");
        assert!(dms.iter().all(|m| m.id != dm_out.id), "hidden DM gone");
        let incoming_dms = load_direct_messages("peer-a", 100).expect("incoming DMs");
        assert!(
            incoming_dms.iter().any(|m| m.id == dm_in.id),
            "incoming DM stays visible"
        );
    });
}

#[test]
#[serial(db)]
fn edit_message_replaces_only_own_live_rows_and_marks_them_edited() {
    with_test_db(|| {
        let topic = "edit-topic";
        let unsent =
            save_message("unsent", None, topic, false, None).expect("save unsent broadcast");
        assert!(
            !edit_message(unsent.id, "new body").expect("unsent row is not editable"),
            "only rows we actually sent can be edited"
        );

        let outgoing =
            save_message("before", None, topic, false, None).expect("save outgoing broadcast");
        mark_message_sent(outgoing.id).expect("mark broadcast sent");
        let incoming =
            save_message("theirs", Some("peer-a"), topic, false, None).expect("save incoming");
        assert!(
            !edit_message(incoming.id, "new body").expect("incoming row is not editable"),
            "incoming rows are never edited"
        );

        assert!(edit_message(outgoing.id, " after ").expect("edit broadcast"));
        let conn = &mut crate::sqlite_connect().expect("connect");
        let (body, edited_at, deleted_at) = crate::generated::schema::messages::table
            .filter(crate::generated::schema::messages::id.eq(outgoing.id))
            .select((
                crate::generated::schema::messages::content,
                crate::generated::schema::messages::edited_at,
                crate::generated::schema::messages::deleted_at,
            ))
            .first::<(String, Option<f64>, Option<f64>)>(conn)
            .expect("read edited row");
        assert_eq!(body, " after ", "the new body is stored verbatim");
        assert!(edited_at.is_some_and(|at| at > 0.0));
        assert_eq!(deleted_at, None, "editing does not tombstone the row");

        assert!(edit_message(outgoing.id, "second").expect("repeat edit"));
        let loaded = load_messages(topic, 100).expect("history");
        let row = loaded
            .iter()
            .find(|m| m.id == outgoing.id)
            .expect("edited row");
        assert_eq!(row.content, "second", "repeat edits replace the body");

        mark_message_deleted(outgoing.id).expect("hide edited row");
        assert!(
            !edit_message(outgoing.id, "after delete").expect("deleted row is not editable"),
            "tombstoned rows cannot be edited"
        );
    });
}

#[test]
#[serial(db)]
fn deleted_dms_leave_peer_stats_but_not_broadcast_recipients() {
    with_test_db(|| {
        let dm = save_message("dm", None, "stats-delete", true, Some("peer-x")).expect("save dm");
        mark_message_sent(dm.id).expect("mark sent");
        let _incoming = save_message("in", Some("peer-y"), "stats-delete", true, Some("peer-y"))
            .expect("save incoming");
        crate::peers::record_broadcast_recipients("m-x", &["peer-x".to_string()])
            .expect("record recipient");

        assert_eq!(
            get_all_peer_stats()
                .expect("before")
                .get("peer-x")
                .expect("peer-x before")
                .dm_count,
            1
        );
        mark_message_deleted(dm.id).expect("hide dm");

        let after = get_all_peer_stats().expect("after");
        let x = after.get("peer-x").expect("peer-x still has broadcasts");
        assert_eq!(x.dm_count, 0, "tombstoned DM no longer counts");
        assert_eq!(
            x.broadcast_sent_to_peer, 1,
            "sent broadcasts are unaffected"
        );
        assert_eq!(
            after.get("peer-y").expect("peer-y").dm_count,
            1,
            "other peers are unaffected"
        );
    });
}

#[test]
#[serial(db)]
fn receipts_for_hidden_rows_remain_consistent() {
    with_test_db(|| {
        let msg = save_message_with_meta(
            "hidden",
            None,
            "receipt-delete",
            false,
            None,
            MessageMeta {
                msg_id: Some("hidden-msg".to_string()),
                ..MessageMeta::default()
            },
        )
        .expect("save");
        mark_message_sent(msg.id).expect("mark sent");
        mark_message_deleted(msg.id).expect("hide");

        // Receipts are independent of visibility: a hidden row can still be
        // acknowledged and still produces a delivery tick on load.
        save_receipt("hidden-msg", "peer-z", 0, 12.0).expect("receipt");
        assert!(
            load_messages("receipt-delete", 100)
                .expect("history")
                .is_empty()
        );
        assert!(
            confirmed_message_ids(&["hidden-msg".to_string()])
                .expect("confirmed")
                .contains("hidden-msg")
        );
        assert_eq!(load_receipts().expect("receipts").len(), 1);
        assert!(
            !message_is_dm("hidden-msg")
                .expect("kind")
                .expect("still in DB")
        );
    });
}

#[test]
#[serial(db)]
fn save_receipt_backfills_broadcast_recipient_confirmation() {
    use crate::generated::schema::broadcast_recipients;
    use diesel::OptionalExtension as _;

    with_test_db(|| {
        crate::peers::record_broadcast_recipients("bc-1", &["peer-a".to_string()])
            .expect("record recipient");

        let conn = &mut crate::sqlite_connect().expect("connect");
        let before: Option<Option<f64>> = broadcast_recipients::table
            .filter(broadcast_recipients::msg_id.eq("bc-1"))
            .select(broadcast_recipients::confirmed_at)
            .first(conn)
            .optional()
            .expect("query before");
        assert_eq!(before, Some(None), "unacked broadcasts start unconfirmed");

        // A kind-0 receipt labels the *broadcast* ack, so it back-fills the row.
        save_receipt("bc-1", "peer-a", 0, 42.0).expect("ack");

        let after: Option<Option<f64>> = broadcast_recipients::table
            .filter(broadcast_recipients::msg_id.eq("bc-1"))
            .select(broadcast_recipients::confirmed_at)
            .first(conn)
            .optional()
            .expect("query after");
        assert_eq!(after, Some(Some(42.0)));

        // A DM-kind (1) receipt never touches the broadcast recipients table.
        crate::peers::record_broadcast_recipients("bc-2", &["peer-a".to_string()])
            .expect("record recipient 2");
        save_receipt("bc-2", "peer-a", 1, 99.0).expect("dm-kind ack");
        let untouched: Option<Option<f64>> = broadcast_recipients::table
            .filter(broadcast_recipients::msg_id.eq("bc-2"))
            .select(broadcast_recipients::confirmed_at)
            .first(conn)
            .optional()
            .expect("query dm-kind");
        assert_eq!(untouched, Some(None));
    });
}
