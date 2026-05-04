//! Tests for messages.rs module

use p2p_app::messages::MessageMeta;
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup_test_db() -> TempDir {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().unwrap()) };
    dir
}

// ── MessageMeta struct ───────────────────────────────────────────────────────

#[test]
fn test_message_meta_default() {
    let meta = MessageMeta::default();
    assert!(meta.sender_nickname.is_none());
    assert!(meta.msg_id.is_none());
    assert!(meta.sent_at.is_none());
}

#[test]
fn test_message_meta_with_all_fields() {
    let meta = MessageMeta {
        sender_nickname: Some("alice".into()),
        msg_id: Some("msg-001".into()),
        sent_at: Some(1_700_000_000.0),
    };
    assert_eq!(meta.sender_nickname.as_deref(), Some("alice"));
    assert_eq!(meta.msg_id.as_deref(), Some("msg-001"));
    assert_eq!(meta.sent_at, Some(1_700_000_000.0));
}

// ── save_message ─────────────────────────────────────────────────────────────

#[test]
fn test_save_broadcast_message() {
    let _db = setup_test_db();
    let msg = p2p_app::save_message("hello world", None, "test-topic", false, None).unwrap();
    assert_eq!(msg.content, "hello world");
    assert_eq!(msg.topic, "test-topic");
    assert_eq!(msg.is_direct, 0);
    assert!(msg.peer_id.is_none());
    assert_eq!(msg.sent, 0); // unsent initially
}

#[test]
fn test_save_incoming_broadcast_message() {
    let _db = setup_test_db();
    let msg = p2p_app::save_message("hi", Some("peer-abc"), "chat", false, None).unwrap();
    assert_eq!(msg.content, "hi");
    assert_eq!(msg.peer_id.as_deref(), Some("peer-abc"));
    assert_eq!(msg.is_direct, 0);
}

#[test]
fn test_save_direct_message() {
    let _db = setup_test_db();
    let msg = p2p_app::save_message("dm content", None, "chat", true, Some("peer-xyz")).unwrap();
    assert_eq!(msg.content, "dm content");
    assert_eq!(msg.is_direct, 1);
    assert_eq!(msg.target_peer.as_deref(), Some("peer-xyz"));
}

#[test]
fn test_save_message_with_meta() {
    let _db = setup_test_db();
    let meta = MessageMeta {
        sender_nickname: Some("bob".into()),
        msg_id: Some("msg-42".into()),
        sent_at: Some(9999.0),
    };
    let msg = p2p_app::save_message_with_meta("meta msg", None, "topic", false, None, meta).unwrap();
    assert_eq!(msg.content, "meta msg");
    assert_eq!(msg.sender_nickname.as_deref(), Some("bob"));
    assert_eq!(msg.msg_id.as_deref(), Some("msg-42"));
    assert_eq!(msg.sent_at, Some(9999.0));
}

#[test]
fn test_save_message_with_empty_content() {
    let _db = setup_test_db();
    let msg = p2p_app::save_message("", None, "topic", false, None).unwrap();
    assert_eq!(msg.content, "");
}

// ── load_messages ────────────────────────────────────────────────────────────

#[test]
fn test_load_messages_empty() {
    let _db = setup_test_db();
    let msgs = p2p_app::load_messages("no-such-topic", 10).unwrap();
    assert!(msgs.is_empty());
}

#[test]
fn test_load_messages_returns_broadcast_only() {
    let _db = setup_test_db();
    p2p_app::save_message("broadcast", None, "topic", false, None).unwrap();
    p2p_app::save_message("direct", None, "topic", true, Some("peer-1")).unwrap();

    let msgs = p2p_app::load_messages("topic", 10).unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0].content, "broadcast");
}

#[test]
fn test_load_messages_limit() {
    let _db = setup_test_db();
    for i in 0..5 {
        p2p_app::save_message(&format!("msg {}", i), None, "limited", false, None).unwrap();
    }
    let msgs = p2p_app::load_messages("limited", 3).unwrap();
    assert_eq!(msgs.len(), 3);
}

#[test]
fn test_load_messages_newest_first() {
    let _db = setup_test_db();
    p2p_app::save_message("first", None, "ordered", false, None).unwrap();
    p2p_app::save_message("second", None, "ordered", false, None).unwrap();
    p2p_app::save_message("third", None, "ordered", false, None).unwrap();

    let msgs = p2p_app::load_messages("ordered", 10).unwrap();
    assert_eq!(msgs[0].content, "third");
    assert_eq!(msgs[2].content, "first");
}

#[test]
fn test_load_messages_topic_isolation() {
    let _db = setup_test_db();
    p2p_app::save_message("for-a", None, "topic-a", false, None).unwrap();
    p2p_app::save_message("for-b", None, "topic-b", false, None).unwrap();

    let msgs_a = p2p_app::load_messages("topic-a", 10).unwrap();
    let msgs_b = p2p_app::load_messages("topic-b", 10).unwrap();
    assert_eq!(msgs_a.len(), 1);
    assert_eq!(msgs_a[0].content, "for-a");
    assert_eq!(msgs_b.len(), 1);
    assert_eq!(msgs_b[0].content, "for-b");
}

// ── load_direct_messages ─────────────────────────────────────────────────────

#[test]
fn test_load_direct_messages_empty() {
    let _db = setup_test_db();
    let msgs = p2p_app::load_direct_messages("unknown-peer", 10).unwrap();
    assert!(msgs.is_empty());
}

#[test]
fn test_load_direct_messages_oldest_first() {
    let _db = setup_test_db();
    p2p_app::save_message("dm-a", None, "t", true, Some("peer-dm")).unwrap();
    p2p_app::save_message("dm-b", None, "t", true, Some("peer-dm")).unwrap();
    p2p_app::save_message("dm-c", None, "t", true, Some("peer-dm")).unwrap();

    let msgs = p2p_app::load_direct_messages("peer-dm", 10).unwrap();
    assert_eq!(msgs.len(), 3);
    assert_eq!(msgs[0].content, "dm-a");
    assert_eq!(msgs[2].content, "dm-c");
}

#[test]
fn test_load_direct_messages_peer_isolation() {
    let _db = setup_test_db();
    p2p_app::save_message("to-alice", None, "t", true, Some("alice")).unwrap();
    p2p_app::save_message("to-bob", None, "t", true, Some("bob")).unwrap();

    let alice_msgs = p2p_app::load_direct_messages("alice", 10).unwrap();
    let bob_msgs = p2p_app::load_direct_messages("bob", 10).unwrap();
    assert_eq!(alice_msgs.len(), 1);
    assert_eq!(alice_msgs[0].content, "to-alice");
    assert_eq!(bob_msgs.len(), 1);
    assert_eq!(bob_msgs[0].content, "to-bob");
}

#[test]
fn test_load_direct_messages_limit() {
    let _db = setup_test_db();
    for i in 0..6 {
        p2p_app::save_message(&format!("dm {}", i), None, "t", true, Some("peer-lim")).unwrap();
    }
    let msgs = p2p_app::load_direct_messages("peer-lim", 4).unwrap();
    assert_eq!(msgs.len(), 4);
}

// ── get_unsent_messages ───────────────────────────────────────────────────────

#[test]
fn test_get_unsent_messages_empty() {
    let _db = setup_test_db();
    let msgs = p2p_app::get_unsent_messages("no-topic").unwrap();
    assert!(msgs.is_empty());
}

#[test]
fn test_get_unsent_messages_returns_only_unsent() {
    let _db = setup_test_db();
    let saved = p2p_app::save_message("unsent", None, "unsent-topic", false, None).unwrap();
    p2p_app::save_message("also-unsent", None, "unsent-topic", false, None).unwrap();

    // Mark one as sent
    p2p_app::mark_message_sent(saved.id).unwrap();

    let unsent = p2p_app::get_unsent_messages("unsent-topic").unwrap();
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].content, "also-unsent");
}

#[test]
fn test_get_unsent_messages_excludes_direct() {
    let _db = setup_test_db();
    p2p_app::save_message("broadcast", None, "topic", false, None).unwrap();
    p2p_app::save_message("direct", None, "topic", true, Some("peer-x")).unwrap();

    let unsent = p2p_app::get_unsent_messages("topic").unwrap();
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].content, "broadcast");
}

// ── get_unsent_direct_messages ────────────────────────────────────────────────

#[test]
fn test_get_unsent_direct_messages_empty() {
    let _db = setup_test_db();
    let msgs = p2p_app::get_unsent_direct_messages("nobody").unwrap();
    assert!(msgs.is_empty());
}

#[test]
fn test_get_unsent_direct_messages_after_sent() {
    let _db = setup_test_db();
    let sent = p2p_app::save_message("dm-sent", None, "t", true, Some("peer-s")).unwrap();
    p2p_app::save_message("dm-unsent", None, "t", true, Some("peer-s")).unwrap();
    p2p_app::mark_message_sent(sent.id).unwrap();

    let unsent = p2p_app::get_unsent_direct_messages("peer-s").unwrap();
    assert_eq!(unsent.len(), 1);
    assert_eq!(unsent[0].content, "dm-unsent");
}

// ── mark_message_sent ─────────────────────────────────────────────────────────

#[test]
fn test_mark_message_sent() {
    let _db = setup_test_db();
    let msg = p2p_app::save_message("to-send", None, "t", false, None).unwrap();
    assert_eq!(msg.sent, 0);

    p2p_app::mark_message_sent(msg.id).unwrap();

    // Should no longer appear in unsent list
    let unsent = p2p_app::get_unsent_messages("t").unwrap();
    assert!(unsent.is_empty());
}

// ── save_receipt / load_receipts ──────────────────────────────────────────────

#[test]
fn test_save_and_load_receipt() {
    let _db = setup_test_db();
    p2p_app::save_receipt("msg-r1", "peer-r1", 0, 1234.5).unwrap();

    let receipts = p2p_app::load_receipts().unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].msg_id, "msg-r1");
    assert_eq!(receipts[0].peer_id, "peer-r1");
    assert_eq!(receipts[0].kind, 0);
    assert_eq!(receipts[0].confirmed_at, 1234.5);
}

#[test]
fn test_save_receipt_upsert() {
    let _db = setup_test_db();
    p2p_app::save_receipt("msg-up", "peer-up", 1, 100.0).unwrap();
    // Same (msg_id, peer_id, kind) — should update confirmed_at
    p2p_app::save_receipt("msg-up", "peer-up", 1, 200.0).unwrap();

    let receipts = p2p_app::load_receipts().unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].confirmed_at, 200.0);
}

#[test]
fn test_save_receipt_multiple_peers() {
    let _db = setup_test_db();
    p2p_app::save_receipt("msg-m", "peer-1", 0, 1.0).unwrap();
    p2p_app::save_receipt("msg-m", "peer-2", 0, 2.0).unwrap();

    let receipts = p2p_app::load_receipts().unwrap();
    assert_eq!(receipts.len(), 2);
}

#[test]
fn test_save_receipt_different_kinds() {
    let _db = setup_test_db();
    p2p_app::save_receipt("msg-k", "peer-k", 0, 1.0).unwrap(); // sent
    p2p_app::save_receipt("msg-k", "peer-k", 1, 2.0).unwrap(); // read

    let receipts = p2p_app::load_receipts().unwrap();
    assert_eq!(receipts.len(), 2);
}

#[test]
fn test_load_receipts_empty() {
    let _db = setup_test_db();
    let receipts = p2p_app::load_receipts().unwrap();
    assert!(receipts.is_empty());
}
