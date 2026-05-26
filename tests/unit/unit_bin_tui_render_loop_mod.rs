use super::*;
use crate::tui::state::AppState;
use std::collections::{HashMap, VecDeque};

fn app_state() -> AppState {
    AppState::new(
        "topic".to_string(),
        "TestUser".to_string(),
        "local-peer".to_string(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        VecDeque::new(),
        VecDeque::new(),
        HashMap::new(),
        VecDeque::new(),
        HashMap::new(),
        HashMap::new(),
    )
}

#[test]
fn test_app_state_to_render_state_defaults() {
    let state = app_state();
    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.tab_titles.len(), 3); // Chat, Peers, Log
    assert_eq!(rs.active_tab, 0);
    assert!(rs.messages.is_empty());
    assert!(rs.peers.is_empty());
    assert_eq!(rs.input_text, "");
    assert!(!rs.editing_nickname);
    assert!(rs.connected);
    assert_eq!(rs.peer_count, 0);
    assert!(rs.mouse_capture);
    assert_eq!(rs.popup, None);
    assert_eq!(rs.chat_scroll_offset, 0);
    assert!(rs.chat_auto_scroll);
    assert_eq!(rs.peer_selection, 0);
}

#[test]
fn test_app_state_to_render_state_messages() {
    let mut state = app_state();
    state
        .messages
        .push_back(("hello".to_string(), Some("p1".to_string())));
    state.messages.push_back(("world".to_string(), None));
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
    state.peers.push_back((
        "p1".into(),
        "2024-01-01 12:00:00".into(),
        "2024-01-02 12:00:00".into(),
    ));
    state.peers.push_back((
        "p2".into(),
        "2024-01-03 12:00:00".into(),
        "2024-01-04 12:00:00".into(),
    ));
    state.peer_selection = 1;
    state.concurrent_peers = 5;

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.peers.len(), 2);
    assert_eq!(rs.peers[0].0, "p1");
    assert_eq!(rs.peer_selection, 1);
    assert_eq!(rs.peer_count, 5);
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
    state.editing_nickname = true;
    state.editing_nickname_peer = Some("peer-edit".to_string());
    let rs = app_state_to_render_state(&state);
    assert!(rs.editing_nickname);
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
    state.chat_auto_scroll = false;
    state.broadcast_selection = Some(3);
    state.mouse_capture = false;

    let rs = app_state_to_render_state(&state);
    assert_eq!(rs.chat_scroll_offset, 10);
    assert!(!rs.chat_auto_scroll);
    assert_eq!(rs.broadcast_selection, Some(3));
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
