//! Tests for types.rs module

#[test]
fn test_swarm_event_debug() {
    let event = p2p_app::types::SwarmEvent::BroadcastMessage {
        content: "hello".to_string(),
        peer_id: "Peer123".to_string(),
        latency: Some("100ms".to_string()),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("BroadcastMessage"));
    assert!(debug.contains("hello"));
}

#[test]
fn test_swarm_event_direct_message() {
    let event = p2p_app::types::SwarmEvent::DirectMessage {
        content: "direct msg".to_string(),
        peer_id: "Peer456".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("DirectMessage"));
}

#[test]
fn test_swarm_event_receipt() {
    let event = p2p_app::types::SwarmEvent::Receipt {
        peer_id: "Peer789".to_string(),
        ack_for: "msg-abc".to_string(),
        received_at: Some(1234567890.0),
    };
    let debug = format!("{:?}", event);
    assert!(debug.contains("Receipt"));
    assert!(debug.contains("msg-abc"));
}

#[test]
fn test_swarm_event_peer_connected() {
    let event = p2p_app::types::SwarmEvent::PeerConnected("PeerABC".to_string());
    let debug = format!("{:?}", event);
    assert!(debug.contains("PeerConnected"));
}

#[test]
fn test_swarm_event_peer_disconnected() {
    let event = p2p_app::types::SwarmEvent::PeerDisconnected("PeerDEF".to_string());
    let debug = format!("{:?}", event);
    assert!(debug.contains("PeerDisconnected"));
}

#[test]
fn test_swarm_event_listen_addr() {
    let event =
        p2p_app::types::SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/8080".to_string());
    let debug = format!("{:?}", event);
    assert!(debug.contains("ListenAddrEstablished"));
}

#[test]
fn test_swarm_command_publish() {
    let cmd = p2p_app::types::SwarmCommand::Publish {
        content: "test msg".to_string(),
        nickname: Some("Bob".to_string()),
        msg_id: Some("msg-xyz".to_string()),
    };
    let debug = format!("{:?}", cmd);
    assert!(debug.contains("Publish"));
}

#[test]
fn test_swarm_command_send_dm() {
    let cmd = p2p_app::types::SwarmCommand::SendDm {
        peer_id: "PeerTarget".to_string(),
        content: "dm content".to_string(),
        nickname: Some("Charlie".to_string()),
        msg_id: Some("dm-1".to_string()),
        ack_for: Some("orig-msg".to_string()),
    };
    let debug = format!("{:?}", cmd);
    assert!(debug.contains("SendDm"));
}

#[test]
fn test_swarm_event_clone() {
    let event1 = p2p_app::types::SwarmEvent::PeerConnected("PeerClone".to_string());
    let event2 = event1.clone();
    assert_eq!(format!("{:?}", event1), format!("{:?}", event2));
}

#[test]
fn test_swarm_command_clone() {
    let cmd1 = p2p_app::types::SwarmCommand::Publish {
        content: "test".to_string(),
        nickname: None,
        msg_id: None,
    };
    let cmd2 = cmd1.clone();
    assert_eq!(format!("{:?}", cmd1), format!("{:?}", cmd2));
}
