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
fn test_build_direct_message_all_some() {
    use p2p_app::swarm_handler::build_direct_message;
    let msg = build_direct_message(
        "full message".to_string(),
        Some("alice".to_string()),
        Some("dm-123".to_string()),
        Some(1.5),
        Some("original-msg".to_string()),
        Some(2.0),
    );
    assert_eq!(msg.content, "full message");
    assert_eq!(msg.nickname, Some("alice".to_string()));
    assert_eq!(msg.msg_id, Some("dm-123".to_string()));
    assert_eq!(msg.sent_at, Some(1.5));
    assert_eq!(msg.ack_for, Some("original-msg".to_string()));
    assert_eq!(msg.received_at, Some(2.0));
}

#[test]
fn test_serialize_broadcast_message_full() {
    use p2p_app::behavior::BroadcastMessage;
    use p2p_app::swarm_handler::serialize_broadcast_message;

    let msg = BroadcastMessage {
        content: "test content".to_string(),
        sent_at: Some(1.234),
        nickname: Some("sender".to_string()),
        msg_id: Some("bc-456".to_string()),
    };

    let json = serialize_broadcast_message(&msg);
    assert!(json.is_some());
    let json_str = json.unwrap();
    assert!(json_str.contains("test content"));
}

#[test]
fn test_serialize_broadcast_message_minimal() {
    use p2p_app::behavior::BroadcastMessage;
    use p2p_app::swarm_handler::serialize_broadcast_message;

    let msg = BroadcastMessage {
        content: "msg".to_string(),
        sent_at: None,
        nickname: None,
        msg_id: None,
    };

    let json = serialize_broadcast_message(&msg);
    assert!(json.is_some());
}

#[test]
fn test_serialize_direct_message_full() {
    use p2p_app::behavior::DirectMessage;
    use p2p_app::swarm_handler::serialize_direct_message;

    let msg = DirectMessage {
        content: "direct msg".to_string(),
        timestamp: 12345,
        sent_at: Some(1.5),
        nickname: Some("alice".to_string()),
        msg_id: Some("dm-789".to_string()),
        ack_for: Some("prev-msg".to_string()),
        received_at: Some(2.5),
    };

    let json = serialize_direct_message(&msg);
    assert!(json.is_some());
    let json_str = json.unwrap();
    assert!(json_str.contains("direct msg"));
    assert!(json_str.contains("12345"));
}

#[test]
fn test_serialize_direct_message_minimal() {
    use p2p_app::behavior::DirectMessage;
    use p2p_app::swarm_handler::serialize_direct_message;

    let msg = DirectMessage {
        content: "msg".to_string(),
        timestamp: 0,
        sent_at: None,
        nickname: None,
        msg_id: None,
        ack_for: None,
        received_at: None,
    };

    let json = serialize_direct_message(&msg);
    assert!(json.is_some());
}

#[test]
fn test_build_broadcast_message_empty_content() {
    use p2p_app::swarm_handler::build_broadcast_message;
    let msg = build_broadcast_message("".to_string(), None, None);
    assert_eq!(msg.content, "");
}

#[test]
fn test_build_direct_message_with_partial_metadata() {
    use p2p_app::swarm_handler::build_direct_message;
    let msg = build_direct_message(
        "msg".to_string(),
        Some("name".to_string()),
        None,
        Some(1.0),
        None,
        None,
    );
    assert_eq!(msg.nickname, Some("name".to_string()));
    assert!(msg.msg_id.is_none());
    assert_eq!(msg.sent_at, Some(1.0));
}
