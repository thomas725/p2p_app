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
        save_message_with_meta(
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
        save_message_with_meta(
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

        assert_eq!(message_is_dm("dm-id").expect("dm"), Some(true));
        assert_eq!(message_is_dm("bc-id").expect("broadcast"), Some(false));
        assert_eq!(message_is_dm("missing-id").expect("missing"), None);
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
