//! Tests for swarm_handler.rs module

use p2p_app::behavior::BroadcastMessage;

#[test]
fn test_build_broadcast_message() {
    let msg = p2p_app::swarm_handler::build_broadcast_message(
        "hello world".to_string(),
        Some("alice".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "hello world");
    assert_eq!(msg.nickname, Some("alice".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
}

#[test]
fn test_build_broadcast_message_no_metadata() {
    let msg = p2p_app::swarm_handler::build_broadcast_message("content".to_string(), None, None);
    assert_eq!(msg.content, "content");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_serialize_broadcast_message_some() {
    let msg = BroadcastMessage {
        content: "test".to_string(),
        sent_at: Some(1.0),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
    };
    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());
    let json_str = json.unwrap();
    assert!(json_str.contains("\"content\":\"test\""));
}

#[test]
fn test_serialize_broadcast_message_none() {
    let msg = BroadcastMessage {
        content: "test".to_string(),
        sent_at: None,
        nickname: None,
        msg_id: None,
    };
    let json = serde_json::to_string(&msg);
    assert!(json.is_ok());
}
