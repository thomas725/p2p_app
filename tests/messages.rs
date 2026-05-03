//! Tests for messages.rs module

#[test]
fn test_message_meta_default() {
    let meta = p2p_app::messages::MessageMeta::default();
    assert!(meta.sender_nickname.is_none());
    assert!(meta.msg_id.is_none());
    assert!(meta.sent_at.is_none());
}

#[test]
fn test_message_meta_with_values() {
    let meta = p2p_app::messages::MessageMeta {
        sender_nickname: Some("tester".to_string()),
        msg_id: Some("msg-123".to_string()),
        sent_at: Some(1234567890.0),
    };
    assert_eq!(meta.sender_nickname, Some("tester".to_string()));
    assert_eq!(meta.msg_id, Some("msg-123".to_string()));
    assert_eq!(meta.sent_at, Some(1234567890.0));
}

#[test]
fn test_message_meta_implements_default() {
    let meta = p2p_app::messages::MessageMeta::default();
    assert!(meta.sender_nickname.is_none());
    assert!(meta.msg_id.is_none());
    assert!(meta.sent_at.is_none());
}

#[test]
fn test_message_meta_clone() {
    let meta = p2p_app::messages::MessageMeta {
        sender_nickname: Some("cloned".to_string()),
        msg_id: None,
        sent_at: None,
    };
    let cloned = p2p_app::messages::MessageMeta {
        sender_nickname: meta.sender_nickname.clone(),
        ..Default::default()
    };
    assert_eq!(cloned.sender_nickname, meta.sender_nickname);
}

#[test]
fn test_message_meta_all_fields() {
    let meta = p2p_app::messages::MessageMeta {
        sender_nickname: Some("alice".to_string()),
        msg_id: Some("abc123".to_string()),
        sent_at: Some(1000.0),
    };
    assert!(meta.sender_nickname.is_some());
    assert!(meta.msg_id.is_some());
    assert!(meta.sent_at.is_some());
}
