//! Tests for swarm_handler.rs module

use p2p_app::behavior::{BroadcastMessage, DirectMessage};

#[test]
fn test_build_broadcast_message() {
    let msg = p2p_app::swarm_handler::build_broadcast_message(
        "hello world",
        Some("alice"),
        Some("msg-123"),
    );
    assert_eq!(msg.content, "hello world");
    assert_eq!(msg.nickname, Some("alice".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
}

#[test]
fn test_build_broadcast_message_no_metadata() {
    let msg = p2p_app::swarm_handler::build_broadcast_message("content", None, None);
    assert_eq!(msg.content, "content");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_build_direct_message() {
    let msg = p2p_app::swarm_handler::build_direct_message(
        "hello peer",
        Some("bob"),
        Some("dm-456"),
        Some(1.5),
        Some("ack-for-123"),
        Some(2.0),
    );
    assert_eq!(msg.content, "hello peer");
    assert_eq!(msg.nickname, Some("bob".to_string()));
    assert_eq!(msg.msg_id, Some("dm-456".to_string()));
    assert_eq!(msg.sent_at, Some(1.5));
    assert_eq!(msg.ack_for, Some("ack-for-123".to_string()));
    assert_eq!(msg.received_at, Some(2.0));
}

#[test]
fn test_build_direct_message_minimal() {
    let msg = p2p_app::swarm_handler::build_direct_message("text", None, None, None, None, None);
    assert_eq!(msg.content, "text");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
    assert!(msg.sent_at.is_none());
}

#[test]
fn test_serialize_broadcast_message_some() {
    let msg = BroadcastMessage {
        content: "test".to_string(),
        sent_at: Some(1.0),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
    };
    let json = p2p_app::swarm_handler::serialize_broadcast_message(&msg);
    assert!(json.is_some());
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
    let json = p2p_app::swarm_handler::serialize_broadcast_message(&msg);
    assert!(json.is_some());
}

#[test]
fn test_serialize_direct_message() {
    let msg = DirectMessage {
        content: "dm content".to_string(),
        timestamp: 12345,
        sent_at: Some(1.0),
        nickname: Some("sender".to_string()),
        msg_id: Some("dm-id".to_string()),
        ack_for: Some("orig-id".to_string()),
        received_at: Some(2.0),
    };
    let json = p2p_app::swarm_handler::serialize_direct_message(&msg);
    assert!(json.is_some());
    let json_str = json.unwrap();
    assert!(json_str.contains("\"content\":\"dm content\""));
    assert!(json_str.contains("\"timestamp\":12345"));
}
