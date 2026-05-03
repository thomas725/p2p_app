//! Integration tests that exercise full binary workflows using library APIs
//!
//! These tests simulate realistic TUI usage patterns without requiring database access

#[test]
fn test_message_meta_serialization() {
    let meta = p2p_app::messages::MessageMeta {
        sender_nickname: Some("TestSender".to_string()),
        msg_id: Some("msg-abc-123".to_string()),
        sent_at: Some(1234567890.123),
    };

    // Verify default works
    let default = p2p_app::messages::MessageMeta::default();
    assert!(default.sender_nickname.is_none());
    assert!(default.msg_id.is_none());
    assert!(default.sent_at.is_none());

    // Non-default has values
    assert!(meta.sender_nickname.is_some());
}

#[test]
fn test_peer_display_name_precedence() {
    let peer_id = "12D3KooWTestPeerDisplay";
    let mut local = std::collections::HashMap::new();
    let mut received = std::collections::HashMap::new();

    // No nicknames -> short ID
    let name = p2p_app::peer_display_name(peer_id, &local, &received);
    assert_eq!(name.len(), 8);

    // Add received nickname
    received.insert(peer_id.to_string(), "ReceivedNick".to_string());
    let name2 = p2p_app::peer_display_name(peer_id, &local, &received);
    assert_eq!(name2, "ReceivedNick");

    // Local nickname takes precedence
    local.insert(peer_id.to_string(), "LocalNick".to_string());
    let name3 = p2p_app::peer_display_name(peer_id, &local, &received);
    assert_eq!(name3, "LocalNick");
}

#[test]
fn test_swarm_command_variants() {
    use p2p_app::types::SwarmCommand;

    // Test Publish
    let publish = SwarmCommand::Publish {
        content: "test content".to_string(),
        nickname: Some("TestNick".to_string()),
        msg_id: Some("msg-123".to_string()),
    };

    let debug = format!("{:?}", publish);
    assert!(debug.contains("Publish"));

    // Test clone
    let cloned = publish.clone();
    match cloned {
        SwarmCommand::Publish { content, .. } => {
            assert_eq!(content, "test content");
        }
        _ => panic!("Expected Publish"),
    }

    // Test SendDm
    let dm = SwarmCommand::SendDm {
        peer_id: "target-peer".to_string(),
        content: "dm content".to_string(),
        nickname: Some("DMNick".to_string()),
        msg_id: Some("dm-1".to_string()),
        ack_for: Some("ack-msg".to_string()),
    };

    let dm_debug = format!("{:?}", dm);
    assert!(dm_debug.contains("SendDm"));
}

#[test]
fn test_swarm_event_variants() {
    use p2p_app::types::SwarmEvent;

    // Test BroadcastMessage
    let bcast = SwarmEvent::BroadcastMessage {
        content: "broadcast test".to_string(),
        peer_id: "PeerEventTest".to_string(),
        latency: Some("50ms".to_string()),
        nickname: Some("EventNick".to_string()),
        msg_id: Some("evt-1".to_string()),
    };

    let debug = format!("{:?}", bcast);
    assert!(debug.contains("BroadcastMessage"));
    assert!(debug.contains("EventNick"));

    // Test DirectMessage
    let direct = SwarmEvent::DirectMessage {
        content: "direct test".to_string(),
        peer_id: "PeerDirect".to_string(),
        latency: None,
        nickname: None,
        msg_id: None,
    };

    let direct_debug = format!("{:?}", direct);
    assert!(direct_debug.contains("DirectMessage"));

    // Test Receipt
    let receipt = SwarmEvent::Receipt {
        peer_id: "ReceiptPeer".to_string(),
        ack_for: "ack-msg-123".to_string(),
        received_at: Some(1234567890.0),
    };

    let receipt_debug = format!("{:?}", receipt);
    assert!(receipt_debug.contains("Receipt"));
    assert!(receipt_debug.contains("ack-msg-123"));

    // Test PeerConnected/Disconnected
    let connected = SwarmEvent::PeerConnected("ConnectedPeer".to_string());
    assert!(format!("{:?}", connected).contains("PeerConnected"));

    let disconnected = SwarmEvent::PeerDisconnected("DisconnectedPeer".to_string());
    assert!(format!("{:?}", disconnected).contains("PeerDisconnected"));

    // Test ListenAddrEstablished
    let listen = SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/8080".to_string());
    assert!(format!("{:?}", listen).contains("ListenAddrEstablished"));
}

#[test]
fn test_tab_content_variants() {
    use p2p_app::tui_tabs::TabContent;

    let chat = TabContent::Chat;
    assert_eq!(chat.peer_id(), None);

    let direct = TabContent::Direct("peer123".to_string());
    assert_eq!(direct.peer_id(), Some("peer123"));

    let peers = TabContent::Peers;
    assert_eq!(peers.peer_id(), None);

    let log = TabContent::Log;
    assert_eq!(log.peer_id(), None);

    // Test Debug
    assert!(format!("{:?}", chat).contains("Chat"));
    assert!(format!("{:?}", direct).contains("Direct"));
    assert!(format!("{:?}", peers).contains("Peers"));
    assert!(format!("{:?}", log).contains("Log"));
}

#[test]
fn test_dynamic_tabs_operations() {
    use p2p_app::tui_tabs::DynamicTabs;

    let mut tabs = DynamicTabs::new();

    assert_eq!(tabs.total_tab_count(), 3); // Chat, Peers, Log
    assert_eq!(tabs.dm_tab_count(), 0);

    // Add DM tab
    let idx1 = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx1, 2);
    assert_eq!(tabs.dm_tab_count(), 1);

    // Add another
    let idx2 = tabs.add_dm_tab("peer2".to_string());
    assert_eq!(idx2, 3);
    assert_eq!(tabs.dm_tab_count(), 2);

    // Remove one
    let removed = tabs.remove_dm_tab("peer1");
    assert!(removed.is_some());
    assert_eq!(tabs.dm_tab_count(), 1);

    // Total should be 4 (Chat, Peers, Log, + 1 DM)
    assert_eq!(tabs.total_tab_count(), 4);
}

#[test]
fn test_dm_tab_creation() {
    use p2p_app::tui_tabs::DmTab;

    let dm = DmTab::new("peer-test".to_string());
    assert_eq!(dm.peer_id, "peer-test");
    assert!(dm.messages.is_empty());

    let msgs = std::collections::VecDeque::from(vec!["msg1".to_string(), "msg2".to_string()]);
    let dm2 = DmTab::with_messages("peer2".to_string(), msgs.clone());
    assert_eq!(dm2.messages.len(), 2);
}

#[test]
fn test_network_size_classification() {
    use p2p_app::network::NetworkSize;

    // Small: 0-3 peers
    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(1.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);

    // Medium: 4-15 peers
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(10.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);

    // Large: 16+ peers
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);

    // Test Debug
    assert!(format!("{:?}", NetworkSize::Small).contains("Small"));
    assert!(format!("{:?}", NetworkSize::Medium).contains("Medium"));
    assert!(format!("{:?}", NetworkSize::Large).contains("Large"));
}

#[test]
fn test_tui_test_state() {
    use p2p_app::tui_test_state::TuiTestState;

    let state = TuiTestState::new();
    assert_eq!(state.active_tab, 0);
    assert!(!state.messages.is_empty());

    // Test with custom messages
    let custom_msgs =
        std::collections::VecDeque::from(vec!["custom1".to_string(), "custom2".to_string()]);
    let state2 = TuiTestState::with_messages(custom_msgs);
    assert_eq!(state2.messages.len(), 2);

    // Test with width
    let state3 = TuiTestState::with_messages_and_width(
        std::collections::VecDeque::from(vec!["test".to_string()]),
        100,
    );
    assert_eq!(state3.terminal_width, 100);
}

#[test]
fn test_direct_message_struct() {
    use p2p_app::DirectMessage;

    let dm = DirectMessage {
        content: "test content".to_string(),
        timestamp: 1234567890,
        sent_at: Some(1234567890.5),
        nickname: Some("DMNick".to_string()),
        msg_id: Some("dm-msg-1".to_string()),
        ack_for: Some("original-msg".to_string()),
        received_at: Some(1234567891.0),
    };

    // Test Debug format
    let debug = format!("{:?}", dm);
    assert!(debug.contains("test content"));
    assert!(debug.contains("DMNick"));

    // Test default
    let default = DirectMessage::default();
    assert!(default.content.is_empty());
    assert_eq!(default.timestamp, 0);
}

#[test]
fn test_broadcast_message_struct() {
    use p2p_app::BroadcastMessage;

    let bm = BroadcastMessage {
        content: "broadcast content".to_string(),
        sent_at: Some(1234567890.5),
        nickname: Some("BcastNick".to_string()),
        msg_id: Some("bcast-msg-1".to_string()),
    };

    let debug = format!("{:?}", bm);
    assert!(debug.contains("broadcast content"));
    assert!(debug.contains("BcastNick"));

    let default = BroadcastMessage::default();
    assert!(default.content.is_empty());
}
