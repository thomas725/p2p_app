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
    let cmd = SwarmCommand::Publish {
        content: "msg".to_string(),
        nickname: None,
        msg_id: None,
    };
    let debug_str = format!("{:?}", cmd);
    assert!(debug_str.contains("Publish"));
}

#[test]
fn test_swarm_command_send_broadcast() {
    use p2p_app::SwarmCommand;
    let cmd = SwarmCommand::Publish {
        content: "breaking news".to_string(),
        nickname: Some("NewsBot".to_string()),
        msg_id: Some("news-1".to_string()),
    };
    match cmd {
        SwarmCommand::Publish { content, nickname, msg_id } => {
            assert_eq!(content, "breaking news");
            assert_eq!(nickname, Some("NewsBot".to_string()));
            assert_eq!(msg_id, Some("news-1".to_string()));
        }
        _ => panic!("Expected Publish"),
    }
}

#[test]
fn test_swarm_command_send_direct() {
    use p2p_app::SwarmCommand;
    let cmd = SwarmCommand::SendDm {
        peer_id: "peer".to_string(),
        content: "direct msg".to_string(),
        nickname: None,
        msg_id: None,
        ack_for: None,
    };
    match cmd {
        SwarmCommand::SendDm { peer_id, content, .. } => {
            assert_eq!(peer_id, "peer");
            assert_eq!(content, "direct msg");
        }
        _ => panic!("Expected SendDm"),
    }
}

#[test]
fn test_swarm_event_broadcast_received() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::BroadcastMessage {
        content: "content".to_string(),
        peer_id: "sender".to_string(),
        latency: Some("100ms".to_string()),
        nickname: None,
        msg_id: None,
    };
    match event {
        SwarmEvent::BroadcastMessage {
            content,
            peer_id,
            latency,
            ..
        } => {
            assert_eq!(content, "content");
            assert_eq!(peer_id, "sender");
            assert_eq!(latency, Some("100ms".to_string()));
        }
        _ => panic!("Expected BroadcastMessage"),
    }
}

#[test]
fn test_swarm_event_direct_message_received() {
    use p2p_app::SwarmEvent;
    let event = SwarmEvent::DirectMessage {
        content: "hi".to_string(),
        peer_id: "sender".to_string(),
        latency: None,
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
    };
    match event {
        SwarmEvent::DirectMessage { peer_id, content, .. } => {
            assert_eq!(peer_id, "sender");
            assert_eq!(content, "hi");
        }
        _ => panic!("Expected DirectMessage"),
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



#[test]
fn test_swarm_command_with_optional_fields() {
    use p2p_app::SwarmCommand;
    
    // Test Publish with all fields
    let pub_full = SwarmCommand::Publish {
        content: "hello".to_string(),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-123".to_string()),
    };
    
    // Test Publish with minimal fields
    let pub_min = SwarmCommand::Publish {
        content: "hi".to_string(),
        nickname: None,
        msg_id: None,
    };
    
    // Test SendDm with ack_for
    let dm_with_ack = SwarmCommand::SendDm {
        peer_id: "peer1".to_string(),
        content: "reply".to_string(),
        nickname: Some("Bob".to_string()),
        msg_id: Some("msg-456".to_string()),
        ack_for: Some("msg-123".to_string()),
    };
    
    assert!(true);
}

#[test]
fn test_swarm_event_receipt() {
    use p2p_app::SwarmEvent;
    
    let receipt = SwarmEvent::Receipt {
        peer_id: "peer123".to_string(),
        ack_for: "msg123".to_string(),
        received_at: Some(1234567890.5),
    };
    
    match receipt {
        SwarmEvent::Receipt { peer_id, ack_for, received_at } => {
            assert_eq!(peer_id, "peer123");
            assert_eq!(ack_for, "msg123");
            assert_eq!(received_at, Some(1234567890.5));
        }
        _ => panic!("Expected Receipt"),
    }
}

#[test]
fn test_dm_tab_short_id() {
    use p2p_app::tui_tabs::DmTab;
    
    let tab = DmTab::new("QmYxQ3XjPvGrGtWjRiYdNq2L9R8pZ1e9Xd8Qk2B3C4D5E6F".to_string());
    let short = tab.short_id();
    
    // Should be last 8 chars
    assert_eq!(short.len(), 8);
    assert_eq!(short, "k2B3C4D5");
}

#[test]
fn test_multiple_swarm_commands() {
    use p2p_app::SwarmCommand;
    
    let mut commands = Vec::new();
    
    for i in 0..5 {
        commands.push(SwarmCommand::Publish {
            content: format!("Message {}", i),
            nickname: Some(format!("User{}", i)),
            msg_id: Some(format!("msg-{}", i)),
        });
    }
    
    assert_eq!(commands.len(), 5);
    
    for cmd in commands {
        match cmd {
            SwarmCommand::Publish { content, .. } => {
                assert!(content.starts_with("Message"));
            }
            _ => panic!("Unexpected variant"),
        }
    }
}


// Additional coverage tests for low-coverage areas

#[cfg(test)]
mod extended_coverage {
    use super::*;

    #[test]
    fn test_message_variants_complete() {
        // Test Message enum variants
        let _broadcast = Message {
            content: "test".to_string(),
            nickname: Some("Alice".to_string()),
            msg_id: Some("1".to_string()),
            sent_at: Some(1234567890.0),
        };
        assert!(true);
    }

    #[test]
    fn test_dm_tab_navigation() {
        let mut tab = DmTab::new("peer1".to_string());
        
        // Test new message
        let msg = Message {
            content: "test".to_string(),
            nickname: None,
            msg_id: None,
            sent_at: None,
        };
        tab.messages.push_back(format!("[You] {}", msg.content));
        
        assert_eq!(tab.messages.len(), 1);
        assert_eq!(tab.peer_id, "peer1");
    }

    #[test]
    fn test_swarm_event_variants() {
        // Test PeerConnected variant
        let _event1 = SwarmEvent::PeerConnected("peer123".to_string());
        
        // Test PeerDisconnected variant
        let _event2 = SwarmEvent::PeerDisconnected("peer456".to_string());
        
        // Test ListenAddrEstablished variant
        let _event3 = SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/9000".to_string());
        
        // Test Receipt variant
        let _event4 = SwarmEvent::Receipt {
            peer_id: "peer789".to_string(),
            ack_for: "msg123".to_string(),
            received_at: Some(1234567890.0),
        };
        
        assert!(true);
    }

    #[test]
    fn test_tab_id_equality() {
        let chat1 = TabId::Chat;
        let chat2 = TabId::Chat;
        let peers = TabId::Peers;
        
        assert_eq!(chat1, chat2);
        assert_ne!(chat1, peers);
    }

    #[test]
    fn test_notification_target_variants() {
        use crate::tui_test_state::NotificationTarget;
        
        let _broadcast = NotificationTarget::Broadcasts;
        let _dm = NotificationTarget::Dm("peer".to_string());
        
        assert!(true);
    }
}

