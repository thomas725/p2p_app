use super::*;

#[test]
fn test_swarm_event_broadcast_message() {
    let event = SwarmEvent::BroadcastMessage(MessageEvent {
        content: "hello".to_string(),
        peer_id: "peer1".to_string(),
        latency: Some("10ms".to_string()),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
    });
    match event {
        SwarmEvent::BroadcastMessage(MessageEvent {
            content, peer_id, ..
        }) => {
            assert_eq!(content, "hello");
            assert_eq!(peer_id, "peer1");
        }
        _ => panic!("expected BroadcastMessage"),
    }
}

#[test]
fn test_swarm_event_direct_message() {
    let event = SwarmEvent::DirectMessage(MessageEvent {
        content: "hi".to_string(),
        peer_id: "peer2".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    });
    match event {
        SwarmEvent::DirectMessage(MessageEvent {
            content, peer_id, ..
        }) => {
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
fn test_peer_record_display() {
    let record = PeerRecord {
        peer_id: "peer1".to_string(),
        first_seen: "2024-01-01 12:00:00".to_string(),
        last_seen: "2024-01-02 12:00:00".to_string(),
    };
    let formatted = format!("{}", record);
    assert_eq!(formatted, "peer1 (2024-01-02 12:00:00)");
}

#[test]
fn test_display_message_self_sent() {
    let msg = DisplayMessage {
        text: "hello".to_string(),
        sender_peer_id: None,
    };
    assert_eq!(msg.text, "hello");
    assert!(msg.sender_peer_id.is_none());
}

#[test]
fn test_display_message_from_peer() {
    let msg = DisplayMessage {
        text: "hello".to_string(),
        sender_peer_id: Some("peer1".to_string()),
    };
    assert_eq!(msg.sender_peer_id, Some("peer1".to_string()));
}

#[test]
fn test_receipt_no_timestamp() {
    let event = SwarmEvent::Receipt {
        peer_id: "peer1".to_string(),
        ack_for: "msg-1".to_string(),
        received_at: None,
    };
    match event {
        SwarmEvent::Receipt {
            peer_id,
            ack_for,
            received_at,
        } => {
            assert_eq!(peer_id, "peer1");
            assert_eq!(ack_for, "msg-1");
            assert!(received_at.is_none());
        }
        _ => panic!("expected Receipt"),
    }
}

#[test]
fn test_message_event_empty_content() {
    let event = SwarmEvent::BroadcastMessage(MessageEvent {
        content: String::new(),
        peer_id: "peer1".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    });
    match event {
        SwarmEvent::BroadcastMessage(me) => {
            assert!(me.content.is_empty());
            assert!(me.latency.is_none());
            assert!(me.nickname.is_none());
            assert!(me.msg_id.is_none());
        }
        _ => panic!("expected BroadcastMessage"),
    }
}
