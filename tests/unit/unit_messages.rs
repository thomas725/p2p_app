use super::*;
use serial_test::serial;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestDb {
    _dir: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for TestDb {
    fn drop(&mut self) {
        crate::db::release_db_lock();
        crate::db::reset_db_url_cache();
        unsafe { std::env::remove_var("DATABASE_URL") };
    }
}

fn setup_test_db() -> TestDb {
    let guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().expect("db path")) };
    crate::db::init_database().expect("init db");
    TestDb {
        _dir: dir,
        _guard: guard,
    }
}

#[test]
#[serial(db)]
fn save_and_mark_message_sent() {
    let _db = setup_test_db();
    let msg = save_message("hello", None, "topic-a", false, None).expect("save");
    let unsent = get_unsent_messages("topic-a").expect("load unsent");
    assert!(unsent.iter().any(|m| m.id == msg.id));
    mark_message_sent(msg.id).expect("mark sent");
    let unsent_after = get_unsent_messages("topic-a").expect("load unsent after");
    assert!(!unsent_after.iter().any(|m| m.id == msg.id));
}

#[test]
#[serial(db)]
fn direct_message_roundtrip_and_unsent_filter() {
    let _db = setup_test_db();
    let saved = save_message("dm", Some("peer-a"), "topic-a", true, Some("peer-b")).expect("save");
    let dms = load_direct_messages("peer-b", 20).expect("load direct");
    assert!(dms.iter().any(|m| m.id == saved.id));
    let unsent = get_unsent_direct_messages("peer-b").expect("load unsent direct");
    assert!(unsent.iter().any(|m| m.id == saved.id));
}

#[test]
#[serial(db)]
fn save_message_with_meta_persists_fields() {
    let _db = setup_test_db();
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
}

#[test]
#[serial(db)]
fn save_receipt_upserts_same_key() {
    let _db = setup_test_db();
    save_receipt("msg-2", "peer-z", 1, 10.0).expect("save receipt");
    save_receipt("msg-2", "peer-z", 1, 20.0).expect("upsert receipt");
    let receipts = load_receipts().expect("load receipts");
    let matching: Vec<_> = receipts
        .into_iter()
        .filter(|r| r.msg_id == "msg-2" && r.peer_id == "peer-z" && r.kind == 1)
        .collect();
    assert_eq!(matching.len(), 1);
    assert_eq!(matching[0].confirmed_at, 20.0);
}

#[test]
#[serial(db)]
fn load_messages_filters_broadcast_and_applies_limit() {
    let _db = setup_test_db();
    let b1 = save_message("b1", Some("peer-a"), "topic-l", false, None).expect("save b1");
    let _b2 = save_message("b2", Some("peer-a"), "topic-l", false, None).expect("save b2");
    let _dm = save_message("dm", Some("peer-a"), "topic-l", true, Some("peer-b")).expect("save dm");

    let loaded = load_messages("topic-l", 1).expect("load messages");
    assert_eq!(loaded.len(), 1);
    assert_ne!(loaded[0].id, b1.id);
    assert_eq!(loaded[0].is_direct, 0);
}

#[test]
#[serial(db)]
fn save_message_reports_context_on_insert_failure() {
    let _db = setup_test_db();
    let conn = &mut crate::sqlite_connect().expect("connect");
    diesel::sql_query("DROP TABLE messages")
        .execute(conn)
        .expect("drop messages");

    let err = save_message("boom", None, "topic-e", false, None).expect_err("must fail");
    let rendered = format!("{err:?}");
    assert!(rendered.contains("Failed to save message"));
    assert!(rendered.contains("topic-e"));
}
