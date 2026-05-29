use super::*;

#[test]
fn test_swarm_event_broadcast_message() {
    let event = SwarmEvent::BroadcastMessage {
        content: "hello".to_string(),
        peer_id: "peer1".to_string(),
        latency: Some("10ms".to_string()),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
    };
    match event {
        SwarmEvent::BroadcastMessage {
            content, peer_id, ..
        } => {
            assert_eq!(content, "hello");
            assert_eq!(peer_id, "peer1");
        }
        _ => panic!("expected BroadcastMessage"),
    }
}

#[test]
fn test_swarm_event_direct_message() {
    let event = SwarmEvent::DirectMessage {
        content: "hi".to_string(),
        peer_id: "peer2".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    };
    match event {
        SwarmEvent::DirectMessage {
            content, peer_id, ..
        } => {
            assert_eq!(content, "hi");
            assert_eq!(peer_id, "peer2");
        }
        _ => panic!("expected DirectMessage"),
    }
}

#[test]
fn test_swarm_event_receipt() {
    let event = SwarmEvent::Receipt {
        peer_id: "peer1".to_string(),
        ack_for: "msg-1".to_string(),
        received_at: Some(123456.0),
    };
    match event {
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => {
            assert_eq!(peer_id, "peer1");
            assert_eq!(ack_for, "msg-1");
        }
        _ => panic!("expected Receipt"),
    }
}

#[test]
fn test_swarm_event_peer_connected() {
    let event = SwarmEvent::PeerConnected("peer1".to_string());
    match event {
        SwarmEvent::PeerConnected(id) => assert_eq!(id, "peer1"),
        _ => panic!("expected PeerConnected"),
    }
}

#[test]
fn test_swarm_event_peer_disconnected() {
    let event = SwarmEvent::PeerDisconnected("peer1".to_string());
    match event {
        SwarmEvent::PeerDisconnected(id) => assert_eq!(id, "peer1"),
        _ => panic!("expected PeerDisconnected"),
    }
}

#[test]
fn test_swarm_event_listen_addr() {
    let event = SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/1234".to_string());
    match event {
        SwarmEvent::ListenAddrEstablished(addr) => assert!(addr.contains("127.0.0.1")),
        _ => panic!("expected ListenAddrEstablished"),
    }
}

#[test]
fn test_swarm_command_publish() {
    let cmd = SwarmCommand::Publish {
        content: "hello".to_string(),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
    };
    match cmd {
        SwarmCommand::Publish {
            content,
            nickname,
            msg_id: _,
        } => {
            assert_eq!(content, "hello");
            assert_eq!(nickname, Some("Alice".to_string()));
        }
        _ => panic!("expected Publish"),
    }
}

#[test]
fn test_swarm_command_send_dm() {
    let cmd = SwarmCommand::SendDm {
        peer_id: "peer1".to_string(),
        content: "hi".to_string(),
        nickname: Some("Bob".to_string()),
        msg_id: Some("dm-1".to_string()),
        ack_for: Some("orig-msg".to_string()),
    };
    match cmd {
        SwarmCommand::SendDm {
            peer_id,
            content,
            ack_for,
            ..
        } => {
            assert_eq!(peer_id, "peer1");
            assert_eq!(content, "hi");
            assert_eq!(ack_for, Some("orig-msg".to_string()));
        }
        _ => panic!("expected SendDm"),
    }
}

#[cfg(feature = "mdns")]
#[test]
fn test_swarm_event_peer_discovered() {
    let addr: Multiaddr = "/ip4/192.168.1.1/tcp/9000".parse().unwrap();
    let event = SwarmEvent::PeerDiscovered {
        peer_id: "peer1".to_string(),
        addresses: vec![addr.clone()],
    };
    match event {
        SwarmEvent::PeerDiscovered { peer_id, addresses } => {
            assert_eq!(peer_id, "peer1");
            assert_eq!(addresses, vec![addr]);
        }
        _ => panic!("expected PeerDiscovered"),
    }
}

#[cfg(feature = "mdns")]
#[test]
fn test_swarm_event_peer_expired() {
    let event = SwarmEvent::PeerExpired {
        peer_id: "peer1".to_string(),
    };
    match event {
        SwarmEvent::PeerExpired { peer_id } => assert_eq!(peer_id, "peer1"),
        _ => panic!("expected PeerExpired"),
    }
}

#[test]
fn test_swarm_event_clone() {
    let event = SwarmEvent::PeerConnected("peer1".to_string());
    let cloned = event.clone();
    match cloned {
        SwarmEvent::PeerConnected(id) => assert_eq!(id, "peer1"),
        _ => panic!("expected PeerConnected"),
    }
}

#[test]
fn test_swarm_command_clone() {
    let cmd = SwarmCommand::Publish {
        content: "test".to_string(),
        nickname: None,
        msg_id: None,
    };
    let cloned = cmd.clone();
    match cloned {
        SwarmCommand::Publish { content, .. } => assert_eq!(content, "test"),
        _ => panic!("expected Publish"),
    }
}

#[test]
fn test_swarm_event_dm_delivered() {
    let event = SwarmEvent::DmDelivered {
        peer_id: "peer2".to_string(),
        msg_id: "msg-1".to_string(),
        latency: Some("5ms".to_string()),
    };
    match event {
        SwarmEvent::DmDelivered { peer_id, msg_id, latency } => {
            assert_eq!(peer_id, "peer2");
            assert_eq!(msg_id, "msg-1");
            assert_eq!(latency, Some("5ms".to_string()));
        }
        _ => panic!("expected DmDelivered"),
    }
}

#[test]
fn test_swarm_event_dm_message() {
    let event = SwarmEvent::DmMessage {
        content: "hello".to_string(),
        peer_id: "peer-sender".to_string(),
        latency: Some("3ms".to_string()),
        nickname: Some("Alice".to_string()),
        msg_id: Some("dm-1".to_string()),
    };
    match event {
        SwarmEvent::DmMessage { content, peer_id, nickname, .. } => {
            assert_eq!(content, "hello");
            assert_eq!(peer_id, "peer-sender");
            assert_eq!(nickname, Some("Alice".to_string()));
        }
        _ => panic!("expected DmMessage"),
    }
}

#[test]
fn test_swarm_command_send_dm() {
    let cmd = SwarmCommand::SendDm {
        content: "private msg".to_string(),
        target_peer: "peer-target".to_string(),
        nickname: Some("Bob".to_string()),
        msg_id: Some("dm-2".to_string()),
    };
    match cmd {
        SwarmCommand::SendDm { content, target_peer, nickname, msg_id } => {
            assert_eq!(content, "private msg");
            assert_eq!(target_peer, "peer-target");
            assert_eq!(nickname, Some("Bob".to_string()));
            assert_eq!(msg_id, Some("dm-2".to_string()));
        }
        _ => panic!("expected SendDm"),
    }
}

#[test]
fn test_swarm_command_set_nickname() {
    let cmd = SwarmCommand::SetNickname("Charlie".to_string());
    match cmd {
        SwarmCommand::SetNickname(name) => assert_eq!(name, "Charlie"),
        _ => panic!("expected SetNickname"),
    }
}

#[test]
fn test_swarm_event_all_variants() {
    // Test that all SwarmEvent variants can be constructed and matched
    let variants = vec![
        SwarmEvent::PeerConnected("peer1".to_string()),
        SwarmEvent::PeerDisconnected("peer2".to_string()),
        SwarmEvent::PeerExpired { peer_id: "peer3".to_string() },
        SwarmEvent::BroadcastMessage {
            content: "msg".to_string(),
            peer_id: "peer4".to_string(),
            latency: None,
            nickname: None,
            msg_id: None,
        },
    ];
    
    assert_eq!(variants.len(), 4, "Should have 4 variants");
}
