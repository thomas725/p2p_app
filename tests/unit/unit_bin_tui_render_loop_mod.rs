use super::*;
use crate::tui::state::AppState;
use p2p_app::{DisplayMessage, PeerRecord};
use serial_test::serial;
use std::collections::{HashMap, VecDeque};

fn app_state() -> AppState {
    AppState::new(
        "topic".to_string(),
        "TestUser".to_string(),
        "local-peer".to_string(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        crate::tui::state::InitialStateData::default(),
    )
}

#[test]
fn test_app_state_to_render_state_defaults() {
    let state = app_state();
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.tab_titles.len(), 5); // Chat, Peers, Groups, Log, Settings
    assert_eq!(rs.active_tab, 0);
    assert!(rs.messages.is_empty());
    assert_eq!(rs.peers.len(), 0);
    assert_eq!(rs.input_text, "");
    assert!(!rs.is_editing_nickname());
    assert!(rs.runtime.connected);
    assert_eq!(rs.peer_count, 0);
    assert!(rs.mouse_capture);
    assert_eq!(rs.popup, None);
    assert_eq!(rs.chat_scroll_offset, 0);
    assert!(rs.auto_scroll.chat);
    assert_eq!(rs.peer_selection, 0);
}

#[test]
fn test_app_state_to_render_state_connected_peer_ids() {
    let mut state = app_state();
    state.peers.push_back(p2p_app::PeerRecord {
        peer_id: "connected-peer".to_string(),
        first_seen: "x".to_string(),
        last_seen: "y".to_string(),
    });
    state.peers.push_back(p2p_app::PeerRecord {
        peer_id: "disconnected-peer".to_string(),
        first_seen: "x".to_string(),
        last_seen: "y".to_string(),
    });
    state
        .connected
        .on_peer_connected("connected-peer".to_string());

    let rs = app_state_to_render_state(&state);
    assert!(rs.connected_peer_ids.contains("connected-peer"));
    assert!(!rs.connected_peer_ids.contains("disconnected-peer"));
    assert_eq!(rs.peers.len(), 2);
}

#[test]
fn test_app_state_to_render_state_last_disconnection() {
    let mut state = app_state();
    state.connected.on_peer_connected("p1".to_string());
    state.connected.on_peer_disconnected("p1", 1234.0);

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.peer_count, 0);
    assert!(rs.connected_peer_ids.is_empty());
    assert_eq!(rs.last_connection_peer.as_deref(), Some("p1"));
    assert_eq!(rs.last_connection_lost, Some(1234.0));
}

#[test]
fn test_app_state_to_render_state_messages() {
    let mut state = app_state();
    state.messages.push_back(DisplayMessage {
        text: "hello".to_string(),
        sender_peer_id: Some("p1".to_string()),
    });
    state.messages.push_back(DisplayMessage {
        text: "world".to_string(),
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some("m1".to_string()));
    state.message_ids.push_back(Some("m2".to_string()));

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.messages.len(), 2);
    assert_eq!(rs.messages[0], "hello");
    assert_eq!(rs.messages[1], "world");
    assert_eq!(rs.message_ids.len(), 2);
}

#[test]
fn test_app_state_to_render_state_peers() {
    let mut state = app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "p1".into(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-02 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "p2".into(),
        first_seen: "2024-01-03 12:00:00".into(),
        last_seen: "2024-01-04 12:00:00".into(),
    });
    state.peer_selection = 1;
    state.concurrent_peers = 5;
    state.connected.on_peer_connected("p1".into());
    state.connected.on_peer_connected("p2".into());

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.peers.len(), 2);
    assert_eq!(rs.peers[0].peer_id, "p1");
    assert_eq!(rs.peer_selection, 1);
    // peer_count reflects the connected-peer set, not the raw connection counter.
    assert_eq!(rs.peer_count, 2);
}

#[test]
fn test_app_state_to_render_state_dm_messages() {
    let mut state = app_state();
    state
        .dm_messages
        .insert("peer-a".to_string(), VecDeque::from(["dm msg".to_string()]));
    state.dm_message_ids.insert(
        "peer-a".to_string(),
        VecDeque::from([Some("dm1".to_string())]),
    );
    state
        .dm_scroll_state
        .insert("peer-a".to_string(), (2, true));
    state
        .dm_broadcast_scroll_state
        .insert("peer-a".to_string(), (1, false));

    let rs = app_state_to_render_state(&state);
    assert!(rs.dm_messages.contains_key("peer-a"));
    assert_eq!(rs.dm_messages["peer-a"][0], "dm msg");
    assert!(rs.dm_message_ids.contains_key("peer-a"));
    assert_eq!(rs.dm_scroll_state.get("peer-a"), Some(&(2, true)));
    assert_eq!(
        rs.dm_broadcast_scroll_state.get("peer-a"),
        Some(&(1, false))
    );
}

#[test]
fn test_app_state_to_render_state_input_text() {
    let mut state = app_state();
    let mut ta = ratatui_textarea::TextArea::default();
    ta.insert_str("hello\nworld");
    state.chat_input = ta;
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.input_text, "hello\nworld");
}

#[test]
fn test_app_state_to_render_state_editing_nickname() {
    let mut state = app_state();
    state.input_mode = p2p_app::InputMode::EditingNickname;
    state.editing_nickname_peer = Some("peer-edit".to_string());
    let rs = app_state_to_render_state(&state);
    assert!(rs.is_editing_nickname());
    assert!(!rs.is_editing_local_nickname());
    assert_eq!(rs.nickname_peer_id, "peer-edit");

    // The local-nickname flavor carries the same target but a distinct mode, so
    // the input-section title reads "Local Nickname" instead of "Edit Nickname".
    state.input_mode = p2p_app::InputMode::EditingLocalNickname;
    let rs = app_state_to_render_state(&state);
    assert!(rs.is_editing_nickname());
    assert!(rs.is_editing_local_nickname());
    assert_eq!(rs.nickname_peer_id, "peer-edit");
}

#[test]
fn test_app_state_to_render_state_popup() {
    let mut state = app_state();
    state.popup = Some("popup text".to_string());
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.popup, Some("popup text".to_string()));
}

#[test]
fn test_app_state_to_render_state_scroll_and_selection() {
    let mut state = app_state();
    state.chat_scroll_offset = 10;
    state.auto_scroll.chat = false;
    state.env.mouse_capture = false;

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.chat_scroll_offset, 10);
    assert!(!rs.auto_scroll.chat);
    assert!(!rs.mouse_capture);
}

#[test]
fn test_app_state_to_render_state_broadcast_receipts() {
    let mut state = app_state();
    let mut inner = HashMap::new();
    inner.insert("p1".to_string(), 100.0);
    state.broadcast_receipts.insert("msg-1".to_string(), inner);
    state
        .dm_receipts
        .insert("dm-1".to_string(), ("p2".to_string(), 200.0));

    let rs = app_state_to_render_state(&state);
    assert!(rs.broadcast_receipts.contains_key("msg-1"));
    assert_eq!(rs.dm_receipts.get("dm-1"), Some(&("p2".to_string(), 200.0)));
}

#[test]
#[serial]
fn test_app_state_to_render_state_log_messages() {
    p2p_app::clear_tui_logs();
    p2p_app::init_logging();
    p2p_app::push_log("log line one");
    p2p_app::push_log("log line two");

    let state = app_state();
    let rs = app_state_to_render_state(&state);
    assert!(
        rs.log_messages
            .iter()
            .any(|line| line.contains("log line one"))
    );
    assert!(
        rs.log_messages
            .iter()
            .any(|line| line.contains("log line two"))
    );
}

#[test]
#[serial]
fn test_device_binding_prompt_renders_and_wins_over_popup() {
    let mut state = app_state();
    state.popup = Some("receipt detail".to_string());
    assert_eq!(render_popup_text(&state).as_deref(), Some("receipt detail"));

    // An active device-binding prompt replaces any ad-hoc popup text.
    state.device_binding_prompt = Some(crate::tui::state::DeviceBindingPrompt {
        peer_id: "12D3Koooprompt-peer".to_string(),
        device_pubkey: vec![1, 2, 3],
    });
    let text = render_popup_text(&state).expect("prompt text");
    assert!(text.contains("Device-binding intro"));
    assert!(text.contains("Accept with 'a'"));
    assert!(text.contains("pubkey 3 bytes"));
    // app_state_to_render_state surfaces the same prompt text.
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.popup.as_deref(), Some(text.as_str()));
}

#[test]
fn test_group_leave_prompt_renders_and_ranks_below_device_binding() {
    let mut state = app_state();
    state.popup = Some("ad-hoc".to_string());
    state.group_leave_prompt = Some(crate::tui::state::GroupLeavePrompt {
        group_id: "g-1".to_string(),
        display_name: "Team Rocket".to_string(),
    });

    // The confirmation outranks an ad-hoc popup and spells out the stakes.
    let text = render_popup_text(&state).expect("leave prompt text");
    assert!(text.contains("Leave group \"Team Rocket\"?"));
    assert!(text.contains("Confirm with 'y' or Enter"));
    assert!(!text.contains("ad-hoc"));
    // app_state_to_render_state surfaces the same prompt text.
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.popup.as_deref(), Some(text.as_str()));

    // An incoming device-binding intro still outranks the leave prompt.
    state.device_binding_prompt = Some(crate::tui::state::DeviceBindingPrompt {
        peer_id: "12D3Koooprompt-peer".to_string(),
        device_pubkey: vec![1],
    });
    let text = render_popup_text(&state).expect("device prompt wins");
    assert!(text.contains("Device-binding intro"));
    assert!(!text.contains("Leave group"));
}

#[test]
fn test_call_overlay_renders_each_state() {
    for (state, expected) in [
        (p2p_app::CallState::Ringing, "Incoming call from"),
        (p2p_app::CallState::Calling, "Calling"),
        (p2p_app::CallState::InCall, "In call with"),
    ] {
        let mut s = app_state();
        s.call_session = Some(p2p_app::CallSession {
            call_id: "call-1".to_string(),
            peer_id: "bob-d1".to_string(),
            state,
        });
        let text = call_overlay_text(&s).expect("overlay text");
        assert!(text.contains(expected), "{text}");
    }
    // A settled session renders no overlay.
    let mut s = app_state();
    s.call_session = Some(p2p_app::CallSession {
        call_id: "call-1".to_string(),
        peer_id: "bob-d1".to_string(),
        state: p2p_app::CallState::Ended,
    });
    assert_eq!(call_overlay_text(&s), None);
}

#[test]
fn test_render_popup_text_call_overlay_beats_popup_and_loses_to_device_prompt() {
    let mut state = app_state();
    state.call_session = Some(p2p_app::CallSession::incoming("call-1", "bob-d1"));
    state.popup = Some("ad-hoc".to_string());
    let text = render_popup_text(&state).expect("overlay wins over popup");
    assert!(text.contains("Incoming call from"));
    assert!(!text.contains("ad-hoc"));

    // The device-binding prompt outranks the call overlay.
    state.device_binding_prompt = Some(crate::tui::state::DeviceBindingPrompt {
        peer_id: "12D3Koooprompt-peer".to_string(),
        device_pubkey: vec![1],
    });
    let text = render_popup_text(&state).expect("prompt wins over overlay");
    assert!(text.contains("Device-binding intro"));
    assert!(!text.contains("Incoming call from"));

    // app_state_to_render_state surfaces the same overlay text.
    let mut idle = app_state();
    idle.call_session = Some(p2p_app::CallSession::incoming("call-1", "bob-d1"));
    let rs = app_state_to_render_state(&idle);
    assert!(
        rs.popup
            .as_deref()
            .is_some_and(|t| t.contains("Incoming call from"))
    );
}

#[test]
fn test_app_state_to_render_state_own_user_identity() {
    let mut state = app_state();
    assert_eq!(app_state_to_render_state(&state).own_user_id, None);
    assert_eq!(app_state_to_render_state(&state).own_user_devices.len(), 0);

    state.own_user_id = Some("user-abc".to_string());
    state.own_user_devices = vec![("dev-1".to_string(), "2026-09-25 10:00:00".to_string())];
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.own_user_id.as_deref(), Some("user-abc"));
    assert_eq!(
        rs.own_user_devices,
        vec![("dev-1".to_string(), "2026-09-25 10:00:00".to_string())]
    );
}

#[test]
fn test_app_state_to_render_state_network_peers() {
    let mut state = app_state();
    state.network_peers = vec![p2p_app::network_config::NetworkPeer {
        id: 7,
        kind: "relay".to_string(),
        multiaddr: "/ip4/5.6.7.8/tcp/4001/p2p/relay".to_string(),
        created_at: "2026-11-08 12:00:00".to_string(),
    }];
    state.network_peer_selection = 0;
    state.network_peer_kind = Some(p2p_app::network_config::NetworkPeerKind::Relay);
    state.input_mode = p2p_app::InputMode::AddingNetworkPeer;

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.network_peers.len(), 1);
    assert_eq!(rs.network_peers[0].id, 7);
    assert_eq!(rs.network_peer_selection, 0);
    assert_eq!(rs.network_peer_kind.as_deref(), Some("relay"));
    assert!(rs.is_adding_network_peer());
}
