use std::collections::HashMap;
use std::time::SystemTime;

use p2p_app::{
    BroadcastMessage, DirectMessage, NetworkSize, SwarmCommand, SwarmEvent, auto_scroll_offset,
    build_broadcast_message, current_timestamp, format_latency, format_system_time, gen_msg_id,
    now_timestamp, peer_display_name, scroll_title, short_peer_id,
};

// ---------------------------------------------------------------------------
// SwarmEvent tests
// ---------------------------------------------------------------------------

#[test]
fn swarm_event_broadcast_message() {
    let ev = SwarmEvent::BroadcastMessage(p2p_app::MessageEvent {
        content: "hello".into(),
        peer_id: "12D3KooWABC".into(),
        latency: Some("42ms".into()),
        nickname: Some("alice".into()),
        msg_id: Some("id1".into()),
    });
    assert!(format!("{ev:?}").contains("BroadcastMessage"));
}

#[test]
fn swarm_event_direct_message() {
    let ev = SwarmEvent::DirectMessage(p2p_app::MessageEvent {
        content: "dm".into(),
        peer_id: "12D3KooWABC".into(),
        latency: None,
        nickname: None,
        msg_id: None,
    });
    assert!(format!("{ev:?}").contains("DirectMessage"));
}

#[test]
fn swarm_event_receipt() {
    let ev = SwarmEvent::Receipt {
        peer_id: "12D3KooWABC".into(),
        ack_for: "msg-001".into(),
        received_at: Some(1_234_567_890.0),
    };
    assert!(format!("{ev:?}").contains("Receipt"));
}

#[test]
fn swarm_event_peer_connected() {
    let ev = SwarmEvent::PeerConnected("12D3KooWABC".into());
    assert_eq!(format!("{ev:?}"), "PeerConnected(\"12D3KooWABC\")");
}

#[test]
fn swarm_event_peer_disconnected() {
    let ev = SwarmEvent::PeerDisconnected("12D3KooWABC".into());
    assert_eq!(format!("{ev:?}"), "PeerDisconnected(\"12D3KooWABC\")");
}

#[test]
fn swarm_event_listen_addr_established() {
    let ev = SwarmEvent::ListenAddrEstablished("/ip4/0.0.0.0".into());
    assert_eq!(format!("{ev:?}"), "ListenAddrEstablished(\"/ip4/0.0.0.0\")");
}

#[cfg(feature = "mdns")]
#[test]
fn swarm_event_peer_discovered() {
    use libp2p::Multiaddr;
    let ev = SwarmEvent::PeerDiscovered {
        peer_id: "12D3KooWABC".into(),
        addresses: vec!["/ip4/127.0.0.1/tcp/9999".parse::<Multiaddr>().unwrap()],
    };
    assert!(format!("{ev:?}").contains("PeerDiscovered"));
}

#[cfg(feature = "mdns")]
#[test]
fn swarm_event_peer_expired() {
    let ev = SwarmEvent::PeerExpired {
        peer_id: "12D3KooWABC".into(),
    };
    assert!(format!("{ev:?}").contains("PeerExpired"));
}

// ---------------------------------------------------------------------------
// SwarmCommand tests
// ---------------------------------------------------------------------------

#[test]
fn swarm_command_publish() {
    let cmd = SwarmCommand::Publish {
        content: "hello".into(),
        nickname: Some("alice".into()),
        msg_id: Some("id1".into()),
    };
    assert!(format!("{cmd:?}").contains("Publish"));
}

#[test]
fn swarm_command_send_dm() {
    let cmd = SwarmCommand::SendDm {
        peer_id: "12D3KooWABC".into(),
        content: "hello".into(),
        nickname: None,
        msg_id: None,
        ack_for: None,
    };
    assert!(format!("{cmd:?}").contains("SendDm"));
}

// ---------------------------------------------------------------------------
// BroadcastMessage
// ---------------------------------------------------------------------------

#[test]
fn broadcast_message_fields() {
    let msg = BroadcastMessage {
        content: "hello".into(),
        sent_at: Some(1000.0),
        nickname: Some("alice".into()),
        msg_id: Some("id1".into()),
    };
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.sent_at, Some(1000.0));
    assert_eq!(msg.nickname, Some("alice".into()));
    assert_eq!(msg.msg_id, Some("id1".into()));
}

#[test]
fn broadcast_message_default() {
    let msg = BroadcastMessage::default();
    assert_eq!(msg.content, String::new());
    assert_eq!(msg.sent_at, None);
}

// ---------------------------------------------------------------------------
// DirectMessage
// ---------------------------------------------------------------------------

#[test]
fn direct_message_fields() {
    let msg = DirectMessage {
        content: "hi".into(),
        timestamp: 12345,
        sent_at: Some(1000.0),
        nickname: Some("bob".into()),
        msg_id: Some("id2".into()),
        ack_for: Some("orig_id".into()),
        received_at: Some(2000.0),
    };
    assert_eq!(msg.content, "hi");
    assert_eq!(msg.timestamp, 12345);
    assert_eq!(msg.ack_for, Some("orig_id".into()));
}

#[test]
fn direct_message_default() {
    let msg = DirectMessage::default();
    assert_eq!(msg.content, String::new());
    assert_eq!(msg.timestamp, 0);
    assert_eq!(msg.ack_for, None);
}

// ---------------------------------------------------------------------------
// build_broadcast_message
// ---------------------------------------------------------------------------

#[test]
fn build_broadcast_message_sets_sent_at() {
    let msg = build_broadcast_message("hello".into(), None, None);
    assert_eq!(msg.content, "hello");
    assert!(msg.sent_at.is_some());
}

#[test]
fn build_broadcast_message_with_all_fields() {
    let msg = build_broadcast_message("hello".into(), Some("alice".into()), Some("msg42".into()));
    assert_eq!(msg.nickname, Some("alice".into()));
    assert_eq!(msg.msg_id, Some("msg42".into()));
}

// ---------------------------------------------------------------------------
// gen_msg_id
// ---------------------------------------------------------------------------

#[test]
fn gen_msg_id_is_non_empty() {
    assert!(!gen_msg_id().is_empty());
}

#[test]
fn gen_msg_ids_are_unique() {
    assert_ne!(gen_msg_id(), gen_msg_id());
}

// ---------------------------------------------------------------------------
// short_peer_id
// ---------------------------------------------------------------------------

#[test]
fn short_peer_id_last_8_chars() {
    assert_eq!(short_peer_id("12D3KooWHashABCD1234"), "ABCD1234");
}

#[test]
fn short_peer_id_shorter_than_8() {
    assert_eq!(short_peer_id("abc"), "abc");
}

// ---------------------------------------------------------------------------
// peer_display_name
// ---------------------------------------------------------------------------

#[test]
fn peer_display_name_prefers_local_nickname() {
    let mut local = HashMap::new();
    local.insert("peer1".into(), "LocalName".into());
    let received = HashMap::new();
    assert_eq!(peer_display_name("peer1", &local, &received), "LocalName");
}

#[test]
fn peer_display_name_falls_back_to_received() {
    let local = HashMap::new();
    let mut received = HashMap::new();
    received.insert("peer1".into(), "ReceivedName".into());
    assert_eq!(
        peer_display_name("peer1", &local, &received),
        "ReceivedName"
    );
}

#[test]
fn peer_display_name_falls_back_to_short_id() {
    assert_eq!(
        peer_display_name("12D3KooWHashXYZ", &HashMap::new(), &HashMap::new()),
        "WHashXYZ"
    );
}

// ---------------------------------------------------------------------------
// auto_scroll_offset
// ---------------------------------------------------------------------------

#[test]
fn auto_scroll_offset_zero_when_fits() {
    assert_eq!(auto_scroll_offset(3, 10), 0);
}

#[test]
fn auto_scroll_offset_positive_when_exceeds() {
    assert_eq!(auto_scroll_offset(10, 5), 5);
}

#[test]
fn auto_scroll_offset_never_panics() {
    assert_eq!(auto_scroll_offset(0, 0), 0);
}

// ---------------------------------------------------------------------------
// format_latency
// ---------------------------------------------------------------------------

#[test]
fn format_latency_unknown() {
    assert_eq!(format_latency(None, SystemTime::now()), "?");
}

#[test]
fn format_latency_sub_ms() {
    let now = SystemTime::now();
    let sent = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    assert_eq!(format_latency(Some(sent), now), "<1ms");
}

// ---------------------------------------------------------------------------
// current_timestamp
// ---------------------------------------------------------------------------

#[test]
fn current_timestamp_is_positive() {
    assert!(current_timestamp() > 1_700_000_000.0);
}

// ---------------------------------------------------------------------------
// now_timestamp
// ---------------------------------------------------------------------------

#[test]
fn now_timestamp_is_19_chars() {
    assert_eq!(now_timestamp().len(), 19);
}

// ---------------------------------------------------------------------------
// format_system_time
// ---------------------------------------------------------------------------

#[test]
fn format_system_time_contains_time_separators() {
    let s = format_system_time(SystemTime::now());
    assert!(s.contains(':'));
}

// ---------------------------------------------------------------------------
// scroll_title
// ---------------------------------------------------------------------------

#[test]
fn scroll_title_basic() {
    assert_eq!(scroll_title("Chat", 5, 10), "Chat (5/10)");
}

#[test]
fn scroll_title_clamps_offset() {
    assert_eq!(scroll_title("Log", 99, 10), "Log (10/10)");
}

// ---------------------------------------------------------------------------
// NetworkSize
// ---------------------------------------------------------------------------

#[test]
fn network_size_display() {
    assert_eq!(NetworkSize::Small.to_string(), "Small");
    assert_eq!(NetworkSize::Medium.to_string(), "Medium");
    assert_eq!(NetworkSize::Large.to_string(), "Large");
}

#[test]
fn network_size_from_peer_count_classifications() {
    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);
}

#[test]
fn network_size_equality() {
    assert_eq!(NetworkSize::Small, NetworkSize::Small);
    assert_ne!(NetworkSize::Small, NetworkSize::Large);
}

// ===========================================================================
// Tests behind feature = "tui"
// ===========================================================================

#[cfg(feature = "tui")]
mod tui_tests {
    use p2p_app::{
        DmTab, DynamicTabs, TabContent, TabId, TuiRenderState, broadcast_receipt_prefix,
        calc_visible_strings, count_lines, dm_receipt_prefix, get_tab_content,
        row_to_visible_index,
    };
    use std::collections::{HashMap, VecDeque};

    // -----------------------------------------------------------------------
    // TabId
    // -----------------------------------------------------------------------

    #[test]
    fn tab_id_default_is_chat() {
        assert_eq!(TabId::default(), TabId::Chat);
    }

    #[test]
    fn tab_id_index_mappings() {
        assert_eq!(TabId::Chat.index(), 0);
        assert_eq!(TabId::Peers.index(), 1);
        assert_eq!(TabId::Direct.index(), 2);
        assert_eq!(TabId::Log.index(), 3);
    }

    #[test]
    fn tab_id_from_index() {
        assert_eq!(TabId::from_index(0), TabId::Chat);
        assert_eq!(TabId::from_index(1), TabId::Peers);
        assert_eq!(TabId::from_index(2), TabId::Direct);
        assert_eq!(TabId::from_index(3), TabId::Log);
        assert_eq!(TabId::from_index(99), TabId::Chat);
    }

    // -----------------------------------------------------------------------
    // DmTab
    // -----------------------------------------------------------------------

    #[test]
    fn dm_tab_new() {
        let tab = DmTab::new("12D3KooWABC".into());
        assert_eq!(tab.peer_id, "12D3KooWABC");
        assert!(tab.messages.is_empty());
    }

    #[cfg(feature = "test-utils")]
    #[test]
    fn dm_tab_with_messages() {
        let msgs = VecDeque::from(["msg1".into(), "msg2".into()]);
        let tab = DmTab::with_messages("peer1".into(), msgs);
        assert_eq!(tab.messages.len(), 2);
    }

    #[test]
    fn dm_tab_short_id() {
        let tab = DmTab::new("12D3KooWHashABCD".into());
        assert_eq!(tab.short_id(), "HashABCD");
    }

    // -----------------------------------------------------------------------
    // DynamicTabs
    // -----------------------------------------------------------------------

    #[test]
    fn dynamic_tabs_new_is_empty() {
        let tabs = DynamicTabs::new();
        assert_eq!(tabs.dm_tab_count(), 0);
    }

    #[test]
    fn dynamic_tabs_add_and_remove() {
        let mut tabs = DynamicTabs::new();
        let idx = tabs.add_dm_tab("peer1".into());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);

        let removed = tabs.remove_dm_tab("peer1");
        assert_eq!(removed, Some(2));
        assert_eq!(tabs.dm_tab_count(), 0);
    }

    #[test]
    fn dynamic_tabs_add_duplicate_returns_same() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".into());
        assert_eq!(tabs.add_dm_tab("peer1".into()), 2);
    }

    #[test]
    fn dynamic_tabs_remove_nonexistent() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.remove_dm_tab("nobody"), None);
    }

    #[test]
    fn dynamic_tabs_get() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".into());
        assert!(tabs.get_dm_tab("peer1").is_some());
        assert!(tabs.get_dm_tab("nobody").is_none());
    }

    #[test]
    fn dynamic_tabs_get_mut() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".into());
        let tab = tabs.get_dm_tab_mut("peer1").unwrap();
        tab.messages.push_back("new".into());
        assert_eq!(tabs.get_dm_tab("peer1").unwrap().messages[0], "new");
    }

    #[test]
    fn dynamic_tabs_titles_without_dms() {
        assert_eq!(
            DynamicTabs::new().all_titles(),
            vec!["Chat", "Peers", "Log"]
        );
    }

    #[test]
    fn dynamic_tabs_titles_with_dm() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peerXYZ".into());
        let titles = tabs.all_titles();
        assert_eq!(titles.len(), 4);
        assert_eq!(titles[2], "peerXYZ (X)");
    }

    #[test]
    fn dynamic_tabs_total_tab_count() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.total_tab_count(), 3);
        tabs.add_dm_tab("p1".into());
        assert_eq!(tabs.total_tab_count(), 4);
        tabs.add_dm_tab("p2".into());
        assert_eq!(tabs.total_tab_count(), 5);
    }

    #[test]
    fn dynamic_tabs_tab_index_to_content() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.tab_index_to_content(0), TabContent::Chat);
        assert_eq!(tabs.tab_index_to_content(1), TabContent::Peers);
        assert_eq!(tabs.tab_index_to_content(2), TabContent::Log);

        tabs.add_dm_tab("peer_dm".into());
        assert_eq!(
            tabs.tab_index_to_content(2),
            TabContent::Direct("peer_dm".into())
        );
        assert_eq!(tabs.tab_index_to_content(3), TabContent::Log);
    }

    // -----------------------------------------------------------------------
    // TabContent
    // -----------------------------------------------------------------------

    #[test]
    fn tab_content_peer_id() {
        assert_eq!(TabContent::Chat.peer_id(), None);
        assert_eq!(TabContent::Peers.peer_id(), None);
        assert_eq!(TabContent::Log.peer_id(), None);
        assert_eq!(TabContent::Direct("abc".into()).peer_id(), Some("abc"));
    }

    #[test]
    fn tab_content_is_input_enabled() {
        assert!(TabContent::Chat.is_input_enabled());
        assert!(TabContent::Direct("x".into()).is_input_enabled());
        assert!(!TabContent::Peers.is_input_enabled());
        assert!(!TabContent::Log.is_input_enabled());
    }

    // -----------------------------------------------------------------------
    // TuiRenderState
    // -----------------------------------------------------------------------

    #[test]
    fn tui_render_state_new() {
        let state = TuiRenderState::new();
        assert_eq!(state.active_tab, 0);
        assert!(state.messages.is_empty());
        assert!(state.peers.is_empty());
        assert!(state.connected);
    }

    #[test]
    fn tui_render_state_with_sample_data() {
        let state = TuiRenderState::with_sample_data();
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.peers.len(), 2);
        assert_eq!(state.peer_count, 2);
    }

    #[test]
    fn tui_render_state_add_message() {
        let mut state = TuiRenderState::new();
        state.add_message("test msg");
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0], "test msg");
    }

    #[test]
    fn tui_render_state_add_peer() {
        let mut state = TuiRenderState::new();
        state.add_peer("id1", "Alice", "Online");
        assert_eq!(state.peers.len(), 1);
        assert_eq!(
            state.peers[0],
            ("id1".into(), "Alice".into(), "Online".into())
        );
    }

    #[test]
    fn tui_render_state_add_dm_message() {
        let mut state = TuiRenderState::new();
        state.add_dm_message("Alice", "hello");
        assert_eq!(state.dm_messages.get("Alice").unwrap().len(), 1);
    }

    // -----------------------------------------------------------------------
    // count_lines
    // -----------------------------------------------------------------------

    #[test]
    fn count_lines_single() {
        assert_eq!(count_lines("hello", 80), 1);
    }

    #[test]
    fn count_lines_wrapped() {
        assert_eq!(count_lines(&"a".repeat(200), 80), 3);
    }

    #[test]
    fn count_lines_empty() {
        assert_eq!(count_lines("", 80), 1);
    }

    #[test]
    fn count_lines_zero_width() {
        assert_eq!(count_lines("hello", 0), 1);
    }

    // -----------------------------------------------------------------------
    // calc_visible_strings
    // -----------------------------------------------------------------------

    #[test]
    fn calc_visible_strings_empty() {
        let msgs = VecDeque::new();
        assert_eq!(calc_visible_strings(&msgs, true, 0, 80, 10), (0, 0));
    }

    #[test]
    fn calc_visible_strings_with_content() {
        let msgs = VecDeque::from(["msg1".into(), "msg2".into()]);
        assert_eq!(calc_visible_strings(&msgs, true, 0, 80, 10).0, 2);
    }

    // -----------------------------------------------------------------------
    // broadcast_receipt_prefix
    // -----------------------------------------------------------------------

    #[test]
    fn broadcast_receipt_prefix_none() {
        assert_eq!(broadcast_receipt_prefix(None, &HashMap::new()), "  ");
    }

    #[test]
    fn broadcast_receipt_prefix_without_receipts() {
        assert_eq!(broadcast_receipt_prefix(Some("id1"), &HashMap::new()), "  ");
    }

    #[test]
    fn broadcast_receipt_prefix_with_receipts() {
        let mut receipts = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert("peer1".into(), 1000.0);
        receipts.insert("id1".into(), inner);
        assert_eq!(broadcast_receipt_prefix(Some("id1"), &receipts), "v ");
    }

    // -----------------------------------------------------------------------
    // dm_receipt_prefix
    // -----------------------------------------------------------------------

    #[test]
    fn dm_receipt_prefix_none() {
        assert_eq!(dm_receipt_prefix(None, &HashMap::new()), "  ");
    }

    #[test]
    fn dm_receipt_prefix_without_receipt() {
        assert_eq!(dm_receipt_prefix(Some("id1"), &HashMap::new()), "  ");
    }

    #[test]
    fn dm_receipt_prefix_with_receipt() {
        let mut receipts = HashMap::new();
        receipts.insert("id1".into(), ("peer1".into(), 1000.0));
        assert_eq!(dm_receipt_prefix(Some("id1"), &receipts), "v ");
    }

    // -----------------------------------------------------------------------
    // row_to_visible_index
    // -----------------------------------------------------------------------

    #[test]
    fn row_to_visible_index_before_content() {
        assert_eq!(row_to_visible_index(&[1, 2], 1, 0), None);
    }

    #[test]
    fn row_to_visible_index_first() {
        assert_eq!(row_to_visible_index(&[1, 2], 1, 1), Some(0));
    }

    #[test]
    fn row_to_visible_index_second() {
        assert_eq!(row_to_visible_index(&[1, 2], 1, 2), Some(1));
    }

    #[test]
    fn row_to_visible_index_out_of_range() {
        assert_eq!(row_to_visible_index(&[1], 0, 5), None);
    }

    // -----------------------------------------------------------------------
    // get_tab_content
    // -----------------------------------------------------------------------

    #[test]
    fn get_tab_content_chat() {
        let mut state = TuiRenderState::new();
        state.tab_titles = vec!["Chat".into(), "Peers".into(), "Log".into()];
        state.active_tab = 0;
        assert_eq!(get_tab_content(&state), TabContent::Chat);
    }

    #[test]
    fn get_tab_content_peers() {
        let mut state = TuiRenderState::new();
        state.tab_titles = vec!["Chat".into(), "Peers".into(), "Log".into()];
        state.active_tab = 1;
        assert_eq!(get_tab_content(&state), TabContent::Peers);
    }

    #[test]
    fn get_tab_content_log() {
        let mut state = TuiRenderState::new();
        state.tab_titles = vec!["Chat".into(), "Peers".into(), "Log".into()];
        state.active_tab = 2;
        assert_eq!(get_tab_content(&state), TabContent::Log);
    }

    #[test]
    fn get_tab_content_dm() {
        let mut state = TuiRenderState::new();
        state.tab_titles = vec![
            "Chat".into(),
            "Peers".into(),
            "DM: Alice".into(),
            "Log".into(),
        ];
        state.active_tab = 2;
        assert_eq!(get_tab_content(&state), TabContent::Direct("Alice".into()));
    }
}
