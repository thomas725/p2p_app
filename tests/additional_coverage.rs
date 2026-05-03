//! Additional unit tests to increase coverage

// Test Debug format coverage (no serde needed)
#[test]
fn test_direct_message_debug() {
    use p2p_app::DirectMessage;

    let dm = DirectMessage {
        content: "test".to_string(),
        timestamp: 12345,
        sent_at: Some(1234567890.0),
        nickname: Some("TestNick".to_string()),
        msg_id: Some("msg-1".to_string()),
        ack_for: None,
        received_at: None,
    };

    let debug = format!("{:?}", dm);
    assert!(debug.contains("test"));
    assert!(debug.contains("TestNick"));
}

#[test]
fn test_broadcast_message_debug() {
    use p2p_app::BroadcastMessage;

    let bm = BroadcastMessage {
        content: "broadcast".to_string(),
        sent_at: Some(1234567890.0),
        nickname: Some("Broadcaster".to_string()),
        msg_id: Some("bcast-1".to_string()),
    };

    let debug = format!("{:?}", bm);
    assert!(debug.contains("broadcast"));
    assert!(debug.contains("Broadcaster"));
}

#[test]
fn test_network_size_classification() {
    use p2p_app::network::NetworkSize;

    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(2.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(10.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);
}

#[test]
fn test_swarm_command_debug() {
    use p2p_app::types::SwarmCommand;

    let publish = SwarmCommand::Publish {
        content: "msg".to_string(),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
    };
    let publish_debug = format!("{:?}", publish);
    assert!(publish_debug.contains("msg"));

    let send_dm = SwarmCommand::SendDm {
        peer_id: "peer123".to_string(),
        content: "dm".to_string(),
        nickname: Some("sender".to_string()),
        msg_id: Some("dm-1".to_string()),
        ack_for: Some("original".to_string()),
    };
    let dm_debug = format!("{:?}", send_dm);
    assert!(dm_debug.contains("peer123"));
}

#[test]
fn test_swarm_event_debug() {
    use p2p_app::types::SwarmEvent;

    let bcast = SwarmEvent::BroadcastMessage {
        content: "msg".to_string(),
        peer_id: "peer1".to_string(),
        latency: Some("100ms".to_string()),
        nickname: Some("nick".to_string()),
        msg_id: Some("id".to_string()),
    };
    let bcast_debug = format!("{:?}", bcast);
    assert!(bcast_debug.contains("msg"));

    let direct = SwarmEvent::DirectMessage {
        content: "direct".to_string(),
        peer_id: "peer2".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    };
    let direct_debug = format!("{:?}", direct);
    assert!(direct_debug.contains("direct"));

    let receipt = SwarmEvent::Receipt {
        peer_id: "peer3".to_string(),
        ack_for: "msg-abc".to_string(),
        received_at: Some(1234567890.0),
    };
    let receipt_debug = format!("{:?}", receipt);
    assert!(receipt_debug.contains("msg-abc"));

    let connected = SwarmEvent::PeerConnected("peer4".to_string());
    let connected_debug = format!("{:?}", connected);
    assert!(connected_debug.contains("peer4"));

    let disconnected = SwarmEvent::PeerDisconnected("peer5".to_string());
    let disconnected_debug = format!("{:?}", disconnected);
    assert!(disconnected_debug.contains("peer5"));

    let listen = SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/8080".to_string());
    let listen_debug = format!("{:?}", listen);
    assert!(listen_debug.contains("8080"));
}
