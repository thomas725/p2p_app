//! Tests for messages.rs module

#[test]
fn test_message_meta_default() {
    let meta = p2p_app::messages::MessageMeta::default();
    assert!(meta.sender_nickname.is_none());
    assert!(meta.msg_id.is_none());
    assert!(meta.sent_at.is_none());
}
