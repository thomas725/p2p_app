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
        };
        let saved = save_message_with_meta("payload", Some("peer-x"), "topic-x", false, None, meta)
            .expect("save with meta");
        assert_eq!(saved.sender_nickname.as_deref(), Some("alice"));
        assert_eq!(saved.msg_id.as_deref(), Some("msg-1"));
        assert_eq!(saved.sent_at, Some(123.5));
    });
}

#[test]
#[allow(clippy::float_cmp)]
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
        assert_eq!(matching[0].confirmed_at, 20.0);
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
        // peer-a: sends 2 DMs (to peer-b and to peer-c), receives 1 DM from peer-b,
        // receives 1 broadcast, and has a persisted broadcast_sent counter.
        crate::save_peer("peer-a", &[]).expect("save a");
        crate::save_peer("peer-b", &[]).expect("save b");
        crate::save_peer("peer-c", &[]).expect("save c");

        let _ = save_message("dm-a-to-b", Some("peer-a"), "t", true, Some("peer-b")).expect("ab");
        let _ = save_message("dm-a-to-c", Some("peer-a"), "t", true, Some("peer-c")).expect("ac");
        let _ = save_message("dm-b-to-a", Some("peer-b"), "t", true, Some("peer-a")).expect("ba");
        let _ =
            save_message("bcast-from-a", Some("peer-a"), "t", false, None).expect("bcast from a");
        crate::peers::record_broadcasts_sent(&["peer-a".to_string()]).expect("bump counter");

        let all = get_all_peer_stats().expect("all stats");

        // peer-a: 2 sent DMs + 1 received DM = 3; 1 broadcast received; 1 broadcast sent.
        let a = all.get("peer-a").expect("peer-a present");
        assert_eq!(a.dm_count, 3);
        assert_eq!(a.broadcast_received, 1);
        assert_eq!(a.broadcast_sent, 1);

        // peer-b: 1 DM sent to a + 1 DM received from a = 2; no broadcasts.
        let b = all.get("peer-b").expect("peer-b present");
        assert_eq!(b.dm_count, 2);
        assert_eq!(b.broadcast_received, 0);

        // peer-c: 1 received DM from a = 1.
        let c = all.get("peer-c").expect("peer-c present");
        assert_eq!(c.dm_count, 1);
        assert_eq!(c.broadcast_received, 0);
    });
}
