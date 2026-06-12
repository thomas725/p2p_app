use super::*;
use crate::tui::test_helpers::{app_state_with_dm_messages, app_state_with_peers, test_app_state};
use std::collections::HashMap;

// ── handle_tab_click ──────────────────────────────────────────────────

#[test]
fn test_tab_click_switches_tab() {
    let mut state = test_app_state();
    let titles = state.dynamic_tabs.all_titles();
    // titles[0] = "Chat", titles[1] = "Peers", titles[2] = "Log"
    // tab_width = len + 3, so "Chat" is at cols 0..7, "Peers" at 7..15, etc.
    let peers_tab_col = titles[0].len() + 3; // column just past the first tab
    let handled = handle_tab_click(&mut state, peers_tab_col as u16, &titles);
    assert!(handled);
    assert_eq!(state.active_tab, 1);
}

#[test]
fn test_tab_click_same_tab_noop() {
    let mut state = test_app_state();
    let titles = state.dynamic_tabs.all_titles();
    let handled = handle_tab_click(&mut state, 0, &titles);
    assert!(!handled);
    assert_eq!(state.active_tab, 0);
}

#[test]
fn test_tab_click_out_of_bounds() {
    let mut state = test_app_state();
    let titles = state.dynamic_tabs.all_titles();
    let handled = handle_tab_click(&mut state, 999, &titles);
    assert!(!handled);
}

#[test]
fn test_tab_click_close_button_on_dm_tab() {
    // Use a short peer ID so short_id() doesn't truncate
    let mut state = app_state_with_dm_messages("p1", 3);
    let titles = state.dynamic_tabs.all_titles();
    // DM tab title format: "p1 (X)" — total width = "p1 (X)".len() + 3 = 9
    let dm_idx = titles.iter().position(|t| t.contains("(X)")).unwrap();
    let col_pos: usize = titles.iter().take(dm_idx).map(|t| t.len() + 3).sum();
    let tab_end = col_pos + titles[dm_idx].len() + 3;
    let close_col = tab_end.saturating_sub(4);
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let handled = handle_tab_click(&mut state, close_col as u16, &titles);
    assert!(handled);
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before - 1);
}

// ── handle_peer_row_click ─────────────────────────────────────────────

#[test]
fn test_peer_row_click_opens_dm_tab() {
    let mut state = app_state_with_peers(3);
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    handle_peer_row_click(&mut state, 3); // row 3 = first peer (header is at rows 0-2)
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
}

#[test]
fn test_peer_row_click_selects_correct_peer() {
    let mut state = app_state_with_peers(3);
    let peer_id = state.peers[1].peer_id.clone(); // second peer
    handle_peer_row_click(&mut state, 4); // row 4 = second peer (header offset)
    assert!(state.dm_messages.contains_key(&peer_id));
}

#[test]
fn test_peer_row_click_out_of_bounds() {
    let mut state = app_state_with_peers(3);
    handle_peer_row_click(&mut state, 99);
}

// ── handle_mouse_left_click ──────────────────────────────────────────────

#[test]
fn test_mouse_left_click_row_zero_routes_to_tab_click() {
    let mut state = test_app_state();
    handle_mouse_left_click(&mut state, 0, 0, false);
    assert_eq!(state.active_tab, 0);
}

#[test]
fn test_mouse_left_click_peers_tab_routes_to_peer_row_click() {
    let mut state = app_state_with_peers(3);
    state.chat_area_height = 20;
    let dm_count_before = state.dynamic_tabs.dm_tab_count();

    handle_mouse_left_click(&mut state, 3, 0, true);

    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
}

#[test]
fn test_mouse_left_click_outside_content_area_is_noop() {
    let mut state = test_app_state();
    state.chat_area_height = 20;
    handle_mouse_left_click(&mut state, 1, 0, false);
    assert_eq!(state.popup, None);
}

#[test]
fn test_mouse_left_click_below_max_row_is_noop() {
    let mut state = test_app_state();
    state.chat_area_height = 20;
    handle_mouse_left_click(&mut state, 99, 0, false);
    assert_eq!(state.popup, None);
}

// ── format_dm_messages_from_db ─────────────────────────────────────────

fn dm_msg(
    content: &str,
    peer_id: Option<&str>,
    sender_nickname: Option<&str>,
    created_at: &str,
) -> p2p_app::generated::models_queryable::Message {
    let dt = chrono::NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S").unwrap();
    p2p_app::generated::models_queryable::Message {
        id: 0,
        created_at: dt,
        content: content.to_string(),
        peer_id: peer_id.map(String::from),
        topic: "test".to_string(),
        sent: 0,
        is_direct: 1,
        target_peer: Some("me".to_string()),
        msg_id: None,
        sent_at: None,
        sender_nickname: sender_nickname.map(String::from),
    }
}

#[test]
fn test_format_dm_messages_from_db_empty() {
    let result = super::format_dm_messages_from_db(&[], "Me", &HashMap::new(), &HashMap::new());
    assert!(result.is_empty());
}

#[test]
fn test_format_dm_messages_from_db_outgoing() {
    let messages = [dm_msg("hello", None, None, "2024-01-01 12:00:00")];
    let result =
        super::format_dm_messages_from_db(&messages, "Me", &HashMap::new(), &HashMap::new());
    assert_eq!(result.len(), 1);
    assert!(result[0].contains("[Me]"));
    assert!(result[0].contains("hello"));
}

#[test]
fn test_format_dm_messages_from_db_incoming_uses_display_name() {
    let messages = [dm_msg("hi", Some("peer-abc"), None, "2024-01-01 12:00:00")];
    let local = HashMap::from([("peer-abc".to_string(), "Alice".to_string())]);
    let result = super::format_dm_messages_from_db(&messages, "Me", &local, &HashMap::new());
    assert!(result[0].contains("[Alice]"));
    assert!(result[0].contains("hi"));
}

#[test]
fn test_format_dm_messages_from_db_reverses_newest_first() {
    let messages = vec![
        dm_msg("second", Some("p1"), None, "2024-01-01 12:00:01"),
        dm_msg("first", Some("p1"), None, "2024-01-01 12:00:00"),
    ];
    let result =
        super::format_dm_messages_from_db(&messages, "Me", &HashMap::new(), &HashMap::new());
    assert_eq!(result.len(), 2);
    assert!(
        result[0].contains("first"),
        "first msg should be first after rev"
    );
    assert!(
        result[1].contains("second"),
        "second msg should be last after rev"
    );
}

#[test]
fn test_format_dm_messages_from_db_self_nick_override() {
    let messages = [dm_msg("my msg", None, None, "2024-01-01 12:00:00")];
    let result = super::format_dm_messages_from_db(
        &messages,
        "CustomNick",
        &HashMap::new(),
        &HashMap::new(),
    );
    assert!(result[0].contains("[CustomNick]"));
    assert!(result[0].contains("my msg"));
}

// ── load_dm_messages ───────────────────────────────────────────────────

#[test]
fn test_load_dm_messages_existing_no_scroll_state_initializes_scroll() {
    let mut state = app_state_with_dm_messages("peer-s", 5);
    // Remove scroll state to trigger the else-if branch
    state.dm_scroll_state.remove("peer-s");
    // dm_messages still has the peer -> enters else-if, initializes scroll state
    load_dm_messages(&mut state, "peer-s");
    let (offset, auto) = state.dm_scroll_state.get("peer-s").unwrap();
    assert_eq!(*offset, 5);
    assert!(*auto);
}
