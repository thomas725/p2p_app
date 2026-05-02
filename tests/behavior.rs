//! Tests for behavior.rs module via libp2p behavior

#[test]
fn test_behavior_constants() {
    assert_eq!(p2p_app::behavior::DM_PROTOCOL_NAME, "/p2p-chat/dm/1.0.0");
    assert_eq!(p2p_app::behavior::CHAT_TOPIC, "test-net");
}

#[test]
fn test_direct_message_default() {
    let dm = p2p_app::behavior::DirectMessage::default();
    assert!(dm.content.is_empty());
    assert_eq!(dm.timestamp, 0);
}

#[test]
fn test_broadcast_message_default() {
    let bm = p2p_app::behavior::BroadcastMessage::default();
    assert!(bm.content.is_empty());
}

#[test]
fn test_direct_message_serialization() {
    use serde_json;
    let dm = p2p_app::behavior::DirectMessage {
        content: "hello".to_string(),
        timestamp: 12345,
        sent_at: Some(1234567890.0),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
        ack_for: None,
        received_at: None,
    };
    let json = serde_json::to_string(&dm).unwrap();
    assert!(json.contains("hello"));
}

#[test]
fn test_broadcast_message_serialization() {
    use serde_json;
    let bm = p2p_app::behavior::BroadcastMessage {
        content: "broadcast test".to_string(),
        sent_at: Some(1234567890.0),
        nickname: Some("Bob".to_string()),
        msg_id: Some("bcast-1".to_string()),
    };
    let json = serde_json::to_string(&bm).unwrap();
    assert!(json.contains("broadcast test"));
}
