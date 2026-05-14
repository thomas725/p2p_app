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

// ── Additional swarm handler tests ────────────────────────────────────────────────

#[test]
fn test_build_broadcast_message_all_none() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("content only".to_string(), None, None);
    assert_eq!(msg.content, "content only");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_build_broadcast_message_empty_content() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("".to_string(), None, None);
    assert_eq!(msg.content, "");
}

#[test]
fn test_build_broadcast_message_only_nickname() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("msg".to_string(), Some("Alice".to_string()), None);
    assert_eq!(msg.content, "msg");
    assert_eq!(msg.nickname, Some("Alice".to_string()));
    assert!(msg.msg_id.is_none());
    assert!(msg.sent_at.is_some());
}

#[test]
fn test_build_broadcast_message_only_msg_id() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("msg".to_string(), None, Some("id-1".to_string()));
    assert!(msg.nickname.is_none());
    assert_eq!(msg.msg_id, Some("id-1".to_string()));
}

#[test]
fn test_build_broadcast_message_long_content() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let long = "a".repeat(1000);
    let msg = build_broadcast_message(long.clone(), None, None);
    assert_eq!(msg.content.len(), 1000);
    assert_eq!(msg.content, long);
}

#[test]
fn test_build_broadcast_message_special_chars() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("Hello! @#$%^&*() 你好 🚀".to_string(), None, None);
    assert_eq!(msg.content, "Hello! @#$%^&*() 你好 🚀");
    assert!(msg.sent_at.is_some());
}
