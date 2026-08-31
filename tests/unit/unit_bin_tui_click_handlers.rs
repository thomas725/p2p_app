#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
#![allow(
    clippy::used_underscore_binding,
    clippy::cast_possible_truncation,
    clippy::as_conversions
)]
use super::*;
use crate::tui::test_helpers::{app_state_with_dm_messages, app_state_with_peers, test_app_state};
use std::collections::HashMap;
use tempfile::TempDir;

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
    // DM tab title format: "p1 [X]" — total width = "p1 [X]".len() + 3 = 9
    let dm_idx = titles.iter().position(|t| t.contains("[X]")).unwrap();
    let col_pos: usize = titles.iter().take(dm_idx).map(|t| t.len() + 3).sum();
    let tab_end = col_pos + titles[dm_idx].len() + 3;
    let close_col = tab_end.saturating_sub(4);
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let handled = handle_tab_click(&mut state, close_col as u16, &titles);
    assert!(handled);
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before - 1);
}

#[test]
fn test_tab_click_close_button_on_peer_info_tab() {
    let mut state = app_state_with_dm_messages("p1", 3);
    state.dynamic_tabs.add_peer_info_tab("p1".to_string());
    let titles = state.dynamic_tabs.all_titles();
    let info_idx = titles.iter().position(|t| t.starts_with("Info:")).unwrap();
    let col_pos: usize = titles.iter().take(info_idx).map(|t| t.len() + 3).sum();
    let tab_end = col_pos + titles[info_idx].len() + 3;
    let close_col = tab_end.saturating_sub(4);
    let count_before = state.dynamic_tabs.peer_info_tab_count();
    let handled = handle_tab_click(&mut state, close_col as u16, &titles);
    assert!(handled);
    assert_eq!(state.dynamic_tabs.peer_info_tab_count(), count_before - 1);
}

// ── handle_peer_row_click ─────────────────────────────────────────────

#[test]
fn test_peer_row_click_opens_dm_tab() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let mut state = app_state_with_peers(3);
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    handle_peer_row_click(&mut state, 3); // row 3 = first peer (data starts after the header at row 2)
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);

    p2p_app::db::reset_db_url();
}

#[test]
fn test_peer_row_click_selects_correct_peer() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let mut state = app_state_with_peers(3);
    let peer_id = state.peers[1].peer_id.clone(); // second peer
    handle_peer_row_click(&mut state, 4); // data starts at row 3 (header at row 2), so row 4 = second peer
    assert!(state.dm_messages.contains_key(&peer_id));

    p2p_app::db::reset_db_url();
}

#[test]
fn test_peer_row_click_out_of_bounds() {
    let mut state = app_state_with_peers(3);
    handle_peer_row_click(&mut state, 99);
}

#[test]
fn test_peer_row_click_respects_scrolled_viewport() {
    let mut state = app_state_with_peers(30);
    state.chat_area_height = 18; // page size = 16 visible data rows
    state.peer_selection = 25; // near the bottom: viewport starts at row 10
    handle_peer_row_click(&mut state, 4); // first visible row = absolute peer 11
    assert_eq!(state.peer_selection, 11);
    // After the click the selection (11) is inside the first page, so the
    // viewport re-anchors to rows 0..16 and row 12 now maps to absolute 9.
    handle_peer_row_click(&mut state, 12);
    assert_eq!(state.peer_selection, 9);
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

    handle_mouse_left_click(&mut state, 3, 0, true); // row 3 = first peer data row

    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
}

#[test]
fn test_mouse_left_click_peers_header_toggles_sort() {
    let mut state = app_state_with_peers(3);
    state.terminal_width = 100;
    state.chat_area_height = 20;
    // Header sits at global row 2; clicking the Name column (x=0) toggles the
    // sort column without opening a DM tab.
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    handle_mouse_left_click(&mut state, 2, 0, true);
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before);
    assert_eq!(state.peer_sort_column, 0);
    // Clicking the same header again toggles the direction.
    handle_mouse_left_click(&mut state, 2, 0, true);
    assert!(state.peer_sort_ascending);
    p2p_app::db::reset_db_url();
}

#[test]
fn test_peer_header_click_maps_each_column() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut state = app_state_with_peers(3);
    state.terminal_width = 100;
    state.chat_area_height = 20;
    // Resolve the real column geometry exactly as the click handler does (via
    // `peer_table_column_widths` + ratatui's Layout), then click one character
    // inside each column to verify the mapping. The geometry is recomputed for
    // every column because moving the sort indicator to the clicked column
    // widens it and shifts the following columns.
    for i in 0..4 {
        let sender_ids = std::collections::VecDeque::new();
        let peers: Vec<p2p_app::PeerRecord> = state.peers.iter().cloned().collect();
        let rows =
            p2p_app::tui_helpers::peer_table_rows_ordered(&peers, &state.dm_messages, &sender_ids);
        let widths = p2p_app::tui_helpers::peer_table_column_widths(
            &rows,
            state.peer_sort_column,
            state.peer_sort_ascending,
        );
        let constraints: Vec<ratatui::layout::Constraint> = widths
            .iter()
            .map(|w| ratatui::layout::Constraint::Length(*w as u16))
            .collect();
        let width = state.terminal_width as u16;
        let area = ratatui::layout::Rect::new(0, 0, width, 1);
        let inner = ratatui::widgets::Block::default()
            .borders(ratatui::widgets::Borders::ALL)
            .inner(area);
        let cols = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints(constraints)
            .spacing(1)
            .split(inner);
        let c = *cols.get(i).unwrap();
        let x = c.x + 1; // one char inside the column
        handle_mouse_left_click(&mut state, 2, x, true);
        assert_eq!(state.peer_sort_column, i, "column {i} click at x={x}");
    }
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

#[test]
fn test_message_click_opens_peer_info_tab() {
    let mut state = test_app_state();
    state.terminal_width = 1000;
    state.chat_area_height = 20;
    state.chat_auto_scroll = true;
    state.chat_scroll_offset = 0;
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "hello from peer".to_string(),
        sender_peer_id: Some("peer-abc".to_string()),
    });

    // Row 2 is the first message row; with width 1000 it's a single line.
    let handled = handle_mouse_left_click(&mut state, 2, 0, false);

    assert!(handled);
    let content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    assert!(matches!(
        content,
        p2p_app::tui_tabs::TabContent::PeerInfo(id) if id == "peer-abc"
    ));
}

#[test]
fn test_message_click_each_message_is_one_row() {
    // The chat renderer shows each message as a single (non-wrapped, clipped)
    // List item, so message N occupies exactly terminal row 2 + N regardless of
    // how long the message text is.
    let mut state = test_app_state();
    state.terminal_width = 20;
    state.chat_area_height = 20;
    state.chat_auto_scroll = true;
    state.chat_scroll_offset = 0;
    // First message is long, but the renderer does not wrap it.
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "aaaaaaaaaaaaaaaaaaaa".to_string(),
        sender_peer_id: Some("peer-abc".to_string()),
    });
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "short".to_string(),
        sender_peer_id: Some("peer-xyz".to_string()),
    });

    // Row 3 is the second message (one row per message, no wrapping).
    let handled = handle_mouse_left_click(&mut state, 3, 0, false);

    assert!(handled);
    let content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    assert!(matches!(
        content,
        p2p_app::tui_tabs::TabContent::PeerInfo(id) if id == "peer-xyz"
    ));
}

#[test]
fn test_message_click_on_own_message_is_noop() {
    let mut state = test_app_state();
    state.terminal_width = 1000;
    state.chat_area_height = 20;
    state.chat_auto_scroll = true;
    state.chat_scroll_offset = 0;
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "my own message".to_string(),
        sender_peer_id: None,
    });

    let handled = handle_mouse_left_click(&mut state, 2, 0, false);

    assert!(!handled);
    assert_eq!(state.dynamic_tabs.peer_info_tab_count(), 0);
}

#[test]
fn test_message_click_autoscroll_maps_to_first_visible_peer() {
    // 20 single-line messages, auto-scroll, height 10 -> top visible is index 10.
    let mut state = test_app_state();
    state.terminal_width = 100;
    state.chat_area_height = 10;
    state.chat_auto_scroll = true;
    state.chat_scroll_offset = 0;
    for i in 0..20 {
        let sender = if i % 2 == 0 {
            Some(format!("peer-{i}"))
        } else {
            None
        };
        state.messages.push_back(p2p_app::DisplayMessage {
            text: format!("msg {i}"),
            sender_peer_id: sender,
        });
    }

    // Row 2 is the first visible (top) message -> index 10 (peer-10).
    let handled = handle_mouse_left_click(&mut state, 2, 0, false);

    assert!(handled);
    let content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    assert!(matches!(
        content,
        p2p_app::tui_tabs::TabContent::PeerInfo(id) if id == "peer-10"
    ));
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
