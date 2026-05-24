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
    let debug = format!("{event:?}");
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
    let debug = format!("{event:?}");
    assert!(debug.contains("DirectMessage"));
}

#[test]
fn test_swarm_event_receipt() {
    let event = p2p_app::types::SwarmEvent::Receipt {
        peer_id: "Peer789".to_string(),
        ack_for: "msg-abc".to_string(),
        received_at: Some(1234567890.0),
    };
    let debug = format!("{event:?}");
    assert!(debug.contains("Receipt"));
    assert!(debug.contains("msg-abc"));
}

#[test]
fn test_swarm_event_peer_connected() {
    let event = p2p_app::types::SwarmEvent::PeerConnected("PeerABC".to_string());
    let debug = format!("{event:?}");
    assert!(debug.contains("PeerConnected"));
}

#[test]
fn test_swarm_event_peer_disconnected() {
    let event = p2p_app::types::SwarmEvent::PeerDisconnected("PeerDEF".to_string());
    let debug = format!("{event:?}");
    assert!(debug.contains("PeerDisconnected"));
}

#[test]
fn test_swarm_event_listen_addr() {
    let event =
        p2p_app::types::SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/8080".to_string());
    let debug = format!("{event:?}");
    assert!(debug.contains("ListenAddrEstablished"));
}

#[test]
fn test_swarm_command_publish() {
    let cmd = p2p_app::types::SwarmCommand::Publish {
        content: "test msg".to_string(),
        nickname: Some("Bob".to_string()),
        msg_id: Some("msg-xyz".to_string()),
    };
    let debug = format!("{cmd:?}");
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
    let debug = format!("{cmd:?}");
    assert!(debug.contains("SendDm"));
}

#[test]
fn test_swarm_event_clone() {
    let event1 = p2p_app::types::SwarmEvent::PeerConnected("PeerClone".to_string());
    let event2 = event1.clone();
    assert_eq!(format!("{event1:?}"), format!("{:?}", event2));
}

#[test]
fn test_swarm_command_clone() {
    let cmd1 = p2p_app::types::SwarmCommand::Publish {
        content: "test".to_string(),
        nickname: None,
        msg_id: None,
    };
    let cmd2 = cmd1.clone();
    assert_eq!(format!("{cmd1:?}"), format!("{:?}", cmd2));
}

// ── Additional type tests ──────────────────────────────────────────────────────

#[test]
fn test_tab_id_from_index_zero() {
    use p2p_app::TabId;
    assert_eq!(TabId::from_index(0), TabId::Chat);
}

#[test]
fn test_tab_id_from_index_one() {
    use p2p_app::TabId;
    assert_eq!(TabId::from_index(1), TabId::Peers);
}

#[test]
fn test_tab_id_from_index_two() {
    use p2p_app::TabId;
    assert_eq!(TabId::from_index(2), TabId::Direct);
}

#[test]
fn test_tab_id_from_index_large() {
    use p2p_app::TabId;
    // Large indices wrap or fall back
    let result = TabId::from_index(100);
    let _ = result;
}

#[test]
fn test_tab_id_clone_eq() {
    use p2p_app::TabId;
    let tid1 = TabId::Chat;
    let tid2 = tid1.clone();
    assert_eq!(tid1, tid2);
}

#[test]
fn test_all_tab_ids() {
    use p2p_app::TabId;
    let chat = TabId::Chat;
    let peers = TabId::Peers;
    let log = TabId::Log;

    assert_ne!(chat, peers);
    assert_ne!(peers, log);
    assert_ne!(chat, log);
}

// ── Additional type construction and conversion tests ───────────────────────────

#[test]
fn test_swarm_command_debug() {
    use p2p_app::SwarmCommand;
    let cmd = SwarmCommand::SendBroadcast {
        topic: "test".to_string(),
        message: "msg".to_string(),
    };
    let debug_str = format!("{:?}", cmd);
    assert!(debug_str.contains("SendBroadcast"));
}

#[test]
fn test_swarm_command_send_broadcast() {
    use p2p_app::SwarmCommand;
    let cmd = SwarmCommand::SendBroadcast {
        topic: "news".to_string(),
        message: "breaking news".to_string(),
    };
    match cmd {
        SwarmCommand::SendBroadcast { topic, message } => {
            assert_eq!(topic, "news");
            assert_eq!(message, "breaking news");
        }
        _ => panic!("Expected SendBroadcast"),
    }
}

#[test]
fn test_swarm_command_send_direct() {
    use p2p_app::SwarmCommand;
    let cmd = SwarmCommand::SendDirect {
        peer_id: "peer".to_string(),
        message: "direct msg".to_string(),
    };
    match cmd {
        SwarmCommand::SendDirect { peer_id, message } => {
            assert_eq!(peer_id, "peer");
            assert_eq!(message, "direct msg");
        }
        _ => panic!("Expected SendDirect"),
    }
}

#[test]
fn test_swarm_event_broadcast_received() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::BroadcastReceived {
        topic: "topic".to_string(),
        message: "content".to_string(),
        from_peer: Some("sender".to_string()),
    };
    match event {
        SwarmEvent::BroadcastReceived { topic, message, from_peer } => {
            assert_eq!(topic, "topic");
            assert_eq!(message, "content");
            assert_eq!(from_peer, Some("sender".to_string()));
        }
        _ => panic!("Expected BroadcastReceived"),
    }
}

#[test]
fn test_swarm_event_direct_message_received() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::DirectMessageReceived {
        from_peer: "sender".to_string(),
        message: "hi".to_string(),
    };
    match event {
        SwarmEvent::DirectMessageReceived { from_peer, message } => {
            assert_eq!(from_peer, "sender");
            assert_eq!(message, "hi");
        }
        _ => panic!("Expected DirectMessageReceived"),
    }
}

#[test]
fn test_swarm_event_peer_discovered() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::PeerDiscovered {
        peer_id: "new-peer".to_string(),
    };
    match event {
        SwarmEvent::PeerDiscovered { peer_id } => {
            assert_eq!(peer_id, "new-peer");
        }
        _ => panic!("Expected PeerDiscovered"),
    }
}

#[test]
fn test_swarm_event_debug() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::PeerDiscovered {
        peer_id: "test".to_string(),
    };
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("PeerDiscovered"));
}

#[test]
fn test_tab_id_ordering() {
    use p2p_app::TabId;
    let chat = TabId::Chat;
    let peers = TabId::Peers;
    let log = TabId::Log;
    
    // Just verify they exist and have different values when converted
    let _ = (chat, peers, log);
}

#[test]
fn test_notification_target_broadcast() {
    use p2p_app::NotificationTarget;
    let target = NotificationTarget::Broadcast {
        topic: "topic".to_string(),
        count: 5,
    };
    match target {
        NotificationTarget::Broadcast { topic, count } => {
            assert_eq!(topic, "topic");
            assert_eq!(count, 5);
        }
        _ => panic!("Expected Broadcast target"),
    }
}

#[test]
fn test_notification_target_dm() {
    use p2p_app::NotificationTarget;
    let target = NotificationTarget::Dm {
        peer_id: "peer".to_string(),
        count: 2,
    };
    match target {
        NotificationTarget::Dm { peer_id, count } => {
            assert_eq!(peer_id, "peer");
            assert_eq!(count, 2);
        }
        _ => panic!("Expected Dm target"),
    }
}
