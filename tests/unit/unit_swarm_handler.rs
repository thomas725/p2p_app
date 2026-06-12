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
fn test_make_ack_dm_with_content() {
    let dm = make_ack_dm("ok".to_string(), Some("msg-1".to_string()));
    assert_eq!(dm.content, "ok");
    assert_eq!(dm.ack_for, Some("msg-1".to_string()));
    assert!(dm.nickname.is_none());
    assert!(dm.msg_id.is_none());
    assert!(dm.sent_at.is_some());
    assert!(dm.received_at.is_some());
}

#[test]
fn test_make_ack_dm_no_ack_for() {
    let dm = make_ack_dm(String::new(), None);
    assert_eq!(dm.content, "");
    assert!(dm.ack_for.is_none());
    assert!(dm.sent_at.is_some());
    assert!(dm.received_at.is_some());
}

#[test]
fn test_swarm_command_variants_all() {
    // Test all SwarmCommand variants are constructible
    let publish = SwarmCommand::Publish {
        content: "msg".to_string(),
        nickname: None,
        msg_id: None,
    };

    // Verify Publish variant
    match publish {
        SwarmCommand::Publish { content, .. } => {
            assert_eq!(content, "msg");
        }
        _ => panic!("expected Publish"),
    }

    let send_dm = SwarmCommand::SendDm {
        peer_id: "peer".to_string(),
        content: "dm msg".to_string(),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
        ack_for: Some("prev-msg".to_string()),
    };

    // Verify SendDm variant with all fields
    match send_dm {
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
        } => {
            assert_eq!(peer_id, "peer");
            assert_eq!(content, "dm msg");
            assert_eq!(nickname, Some("Alice".to_string()));
            assert_eq!(msg_id, Some("msg-1".to_string()));
            assert_eq!(ack_for, Some("prev-msg".to_string()));
        }
        _ => panic!("expected SendDm"),
    }
}
