use super::*;

#[test]
fn test_build_broadcast_message() {
    let msg = build_broadcast_message(
        "Hello world".to_string(),
        Some("Alice".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "Hello world");
    assert_eq!(msg.nickname, Some("Alice".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
    assert!(msg.sent_at.is_some());
}

#[test]
fn test_build_broadcast_message_empty_content() {
    let msg = build_broadcast_message(String::new(), None, None);
    assert!(msg.content.is_empty());
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_build_broadcast_message_with_all_fields() {
    let msg = build_broadcast_message(
        "test content".to_string(),
        Some("Tester".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "test content");
    assert_eq!(msg.nickname, Some("Tester".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
}

#[test]
fn test_swarm_command_variants_all() {
    // Test all SwarmCommand variants are constructible
    let _publish = SwarmCommand::Publish {
        content: "msg".to_string(),
        nickname: None,
        msg_id: None,
    };
    
    let _send_dm = SwarmCommand::SendDm {
        peer_id: "peer".to_string(),
        content: "msg".to_string(),
        nickname: None,
        msg_id: None,
        ack_for: None,
    };
    
    let _request_ack = SwarmCommand::RequestAck {
        peer_id: "peer".to_string(),
        msg_id: "msg-1".to_string(),
    };
    
    let _send_ack = SwarmCommand::SendAck {
        peer_id: "peer".to_string(),
        msg_id: "msg-1".to_string(),
    };
}

