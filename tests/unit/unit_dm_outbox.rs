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
    crate::db::set_db_url(db_path.to_str().expect("db path"));
    crate::db::init_database().expect("init db");
    f();
    crate::db::release_db_lock();
    crate::db::reset_db_url();
}

/// A sealed-envelope-shaped JSON blob (routing fields only, no readable body).
fn sealed_envelope(msg_id: &str) -> String {
    serde_json::json!({
        "content": "",
        "timestamp": 1_i64,
        "sent_at": 1.0_f64,
        "nickname": null,
        "msg_id": msg_id,
        "ack_for": null,
        "received_at": null,
        "group_id": null,
        "attachment": null,
        "reply_to": null,
        "encrypted": { "ciphertext": "deadbeef", "nonce": "00", "ephemeral_pubkey": "ff" },
    })
    .to_string()
}

#[test]
#[serial]
fn queue_is_idempotent_per_recipient_and_msg_id() {
    with_test_db(|| {
        let json = sealed_envelope("msg-1");
        assert!(queue("peer-a", "msg-1", &json, DEFAULT_TTL_SECS).expect("queue first"));
        assert!(
            !queue("peer-a", "msg-1", &json, DEFAULT_TTL_SECS).expect("queue duplicate"),
            "a resend must not create a second row"
        );
        assert_eq!(count().expect("count"), 1);
        // The same msg_id to a different recipient is a distinct envelope.
        assert!(queue("peer-b", "msg-1", &json, DEFAULT_TTL_SECS).expect("queue other peer"));
        assert_eq!(count().expect("count"), 2);
    });
}

#[test]
#[serial]
fn pending_and_pending_for_filter_by_expiry_and_recipient() {
    with_test_db(|| {
        let json = sealed_envelope("fresh");
        assert!(queue("peer-a", "fresh", &json, 3600).expect("queue fresh"));
        assert!(queue("peer-a", "stale", &json, -10).expect("queue stale"));
        assert!(queue("peer-b", "other", &json, 3600).expect("queue other"));

        let all = pending().expect("pending");
        assert_eq!(all.len(), 2, "expired row must be excluded");
        assert!(all.iter().all(|row| row.msg_id != "stale"));

        let for_a = pending_for("peer-a").expect("pending_for");
        assert_eq!(for_a.len(), 1);
        assert_eq!(for_a[0].msg_id, "fresh");
    });
}

#[test]
#[serial]
fn prune_expired_removes_only_expired_rows() {
    with_test_db(|| {
        let json = sealed_envelope("x");
        assert!(queue("peer-a", "fresh", &json, 3600).expect("queue fresh"));
        assert!(queue("peer-a", "stale", &json, -10).expect("queue stale"));
        assert_eq!(prune_expired().expect("prune"), 1);
        assert_eq!(count().expect("count"), 1);
        assert_eq!(pending().expect("pending")[0].msg_id, "fresh");
    });
}

#[test]
#[serial]
fn remove_deletes_only_the_matching_row() {
    with_test_db(|| {
        let json = sealed_envelope("x");
        assert!(queue("peer-a", "msg-1", &json, DEFAULT_TTL_SECS).expect("queue 1"));
        assert!(queue("peer-a", "msg-2", &json, DEFAULT_TTL_SECS).expect("queue 2"));
        assert_eq!(remove("peer-a", "msg-1").expect("remove"), 1);
        // A different recipient or an unknown id is a harmless no-op.
        assert_eq!(remove("peer-b", "msg-2").expect("remove other"), 0);
        assert_eq!(remove("peer-a", "missing").expect("remove missing"), 0);
        assert_eq!(count().expect("count"), 1);
        assert_eq!(pending().expect("pending")[0].msg_id, "msg-2");
    });
}

#[test]
#[serial]
fn record_attempt_increments_the_counter() {
    with_test_db(|| {
        assert!(
            queue(
                "peer-a",
                "msg-1",
                &sealed_envelope("msg-1"),
                DEFAULT_TTL_SECS
            )
            .expect("queue")
        );
        let id = pending().expect("pending")[0].id;
        record_attempt(id).expect("attempt 1");
        record_attempt(id).expect("attempt 2");
        assert_eq!(pending().expect("pending")[0].attempts, 2);
    });
}

#[test]
#[serial]
fn queued_row_stores_only_the_sealed_envelope() {
    with_test_db(|| {
        let json = sealed_envelope("sealed-only");
        assert!(queue("peer-a", "sealed-only", &json, DEFAULT_TTL_SECS).expect("queue"));
        let row = &pending().expect("pending")[0];
        assert!(row.envelope_json.contains("\"encrypted\""));
        let parsed: serde_json::Value =
            serde_json::from_str(&row.envelope_json).expect("envelope json");
        assert_eq!(parsed["content"], "");
        assert!(parsed["encrypted"].is_object());
    });
}

#[test]
#[serial]
fn incoming_dm_exists_detects_persisted_replies() {
    with_test_db(|| {
        assert!(
            !crate::messages::incoming_dm_exists("sender-1", "dup-1"),
            "nothing persisted yet"
        );
        crate::messages::save_message_with_meta(
            "hello",
            Some("sender-1"),
            crate::CHAT_TOPIC,
            true,
            Some("me"),
            crate::messages::MessageMeta {
                msg_id: Some("dup-1".to_string()),
                ..Default::default()
            },
        )
        .expect("save incoming");
        assert!(crate::messages::incoming_dm_exists("sender-1", "dup-1"));
        assert!(
            !crate::messages::incoming_dm_exists("sender-2", "dup-1"),
            "sender is part of the key"
        );
        assert!(!crate::messages::incoming_dm_exists("sender-1", "other"));
    });
}
