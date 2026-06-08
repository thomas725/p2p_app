use super::*;
use crate::tui::state::AppState;
use crate::tui::test_helpers::{app_state_with_dm_messages, app_state_with_peers, test_app_state};
use std::collections::{HashMap, VecDeque};

fn empty_state() -> AppState {
    AppState::new(
        "topic".to_string(),
        "me".to_string(),
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
fn test_count_nicknames() {
    let peers: VecDeque<_> = VecDeque::from(vec![
        PeerRecord {
            peer_id: "peer1".to_string(),
            first_seen: "seen1".to_string(),
            last_seen: "last1".to_string(),
        },
        PeerRecord {
            peer_id: "peer2".to_string(),
            first_seen: "seen2".to_string(),
            last_seen: "last2".to_string(),
        },
    ]);
    let local = HashMap::from([("peer1".to_string(), "Alice".to_string())]);
    let received = HashMap::from([("peer2".to_string(), "Bob".to_string())]);

    let counts = super::count_nicknames(peers.iter(), &local, &received);
    assert_eq!(counts.get("Alice"), Some(&1));
    assert_eq!(counts.get("Bob"), Some(&1));
}

#[test]
fn test_count_nicknames_duplicates() {
    let peers: VecDeque<_> = VecDeque::from(vec![
        PeerRecord {
            peer_id: "peer1".to_string(),
            first_seen: "seen1".to_string(),
            last_seen: "last1".to_string(),
        },
        PeerRecord {
            peer_id: "peer2".to_string(),
            first_seen: "seen2".to_string(),
            last_seen: "last2".to_string(),
        },
    ]);
    let local = HashMap::from([
        ("peer1".to_string(), "Alice".to_string()),
        ("peer2".to_string(), "Alice".to_string()),
    ]);
    let received = HashMap::new();

    let counts = super::count_nicknames(peers.iter(), &local, &received);
    assert_eq!(counts.get("Alice"), Some(&2));
}

#[test]
fn test_format_peer_display_with_nickname_unique() {
    let counts = HashMap::from([("Alice".to_string(), 1usize)]);
    let result = super::format_peer_display("peer1", Some(&"Alice".to_string()), &counts, |id| {
        id.chars().rev().take(8).collect()
    });
    assert_eq!(result, "Alice");
}

#[test]
fn test_format_peer_display_with_nickname_duplicate() {
    let counts = HashMap::from([("Alice".to_string(), 2usize)]);
    let result = super::format_peer_display("peer1", Some(&"Alice".to_string()), &counts, |id| {
        id.chars().rev().take(8).collect()
    });
    assert!(result.contains("Alice"));
    // The short_peer_id function reverses and takes last 8 chars, so "peer1" -> "1reep"
    assert!(result.contains("1reep"));
}

#[test]
fn test_format_peer_display_no_nickname() {
    let counts = HashMap::new();
    let result = super::format_peer_display("peer1", None, &counts, |id| {
        id.chars().rev().take(8).collect()
    });
    // The short_peer_id function reverses and takes last 8 chars
    assert_eq!(result, "1reep");
}

#[test]
fn test_format_broadcast_receipt_popup_impl_empty() {
    let receipts = HashMap::new();
    let peers: VecDeque<p2p_app::PeerRecord> = VecDeque::new();
    let local = HashMap::new();
    let received = HashMap::new();
    let result =
        super::format_broadcast_receipt_popup_impl(&receipts, &peers, &local, &received, None);
    assert_eq!(
        result,
        Some("No peers have confirmed receipt yet.".to_string())
    );
}

#[test]
fn test_format_broadcast_receipt_popup_impl_with_data() {
    let receipts = HashMap::from([("peer1".to_string(), 2.0), ("peer2".to_string(), 3.0)]);
    let peers: VecDeque<_> = VecDeque::from(vec![
        PeerRecord {
            peer_id: "peer1".to_string(),
            first_seen: "s1".to_string(),
            last_seen: "l1".to_string(),
        },
        PeerRecord {
            peer_id: "peer2".to_string(),
            first_seen: "s2".to_string(),
            last_seen: "l2".to_string(),
        },
    ]);
    let local = HashMap::new();
    let received = HashMap::new();
    let result =
        super::format_broadcast_receipt_popup_impl(&receipts, &peers, &local, &received, Some(1.0));
    assert!(result.is_some());
    let s = result.unwrap();
    assert!(s.contains("peer1"));
    assert!(s.contains("peer2"));
    assert!(s.contains("1000ms"));
}

#[test]
fn test_format_dm_receipt_popup_impl_with_time() {
    let result = super::format_dm_receipt_popup_impl("peer1", 2.0, Some(1.0));
    assert!(result.contains("peer1"));
    assert!(result.contains("1000ms"));
}

#[test]
fn test_format_dm_receipt_popup_impl_confirmed() {
    let result = super::format_dm_receipt_popup_impl("peer1", 2.0, None);
    assert!(result.contains("peer1"));
    assert!(result.contains("confirmed"));
}

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

// ── format_broadcast_receipt_popup (private wrapper) ────────────────────

#[test]
fn test_broadcast_receipt_popup_returns_some_when_msg_exists() {
    let mut state = empty_state();
    state.broadcast_receipts.insert(
        "msg-1".to_string(),
        HashMap::from([("p1".to_string(), 2.0)]),
    );
    let result = super::format_broadcast_receipt_popup(&state, "msg-1", Some(1.0));
    assert!(result.is_some());
    assert!(result.unwrap().contains("p1"));
}

#[test]
fn test_broadcast_receipt_popup_returns_none_when_msg_missing() {
    let state = empty_state();
    let result = super::format_broadcast_receipt_popup(&state, "nonexistent", None);
    assert!(result.is_none());
}

// ── format_dm_receipt_popup (private wrapper) ────────────────────────────

#[test]
fn test_dm_receipt_popup_returns_some_when_msg_exists() {
    let mut state = empty_state();
    state
        .dm_receipts
        .insert("dm-1".to_string(), ("p1".to_string(), 2.5));
    state.sent_at_by_msg_id.insert("dm-1".to_string(), 1.0);
    let result = super::format_dm_receipt_popup(&state, "dm-1");
    assert!(result.is_some());
    assert!(result.unwrap().contains("p1"));
}

#[test]
fn test_dm_receipt_popup_returns_none_when_msg_missing() {
    let state = empty_state();
    let result = super::format_dm_receipt_popup(&state, "nonexistent");
    assert!(result.is_none());
}

// ── handle_mouse_left_click ──────────────────────────────────────────────

#[test]
fn test_mouse_left_click_row_zero_routes_to_tab_click() {
    let mut state = empty_state();
    handle_mouse_left_click(&mut state, 0, 0, false, false, None);
    // Tab click at column 0 on first tab is same-tab noop
    assert_eq!(state.active_tab, 0);
}

#[test]
fn test_mouse_left_click_peers_tab_routes_to_peer_row_click() {
    let mut state = app_state_with_peers(3);
    state.chat_area_height = 20;
    let dm_count_before = state.dynamic_tabs.dm_tab_count();

    handle_mouse_left_click(&mut state, 3, 0, true, false, None);

    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
}

#[test]
fn test_mouse_left_click_outside_content_area_is_noop() {
    let mut state = empty_state();
    state.chat_area_height = 20;
    handle_mouse_left_click(&mut state, 1, 0, false, false, None);
    // row 1 <= 2, outside content area
    assert_eq!(state.popup, None);
}

#[test]
fn test_mouse_left_click_below_max_row_is_noop() {
    let mut state = empty_state();
    state.chat_area_height = 20;
    handle_mouse_left_click(&mut state, 99, 0, false, false, None);
    // row 99 >= max_row (22), outside content area
    assert_eq!(state.popup, None);
}

#[test]
fn test_mouse_left_click_dm_tab_no_peer_id_does_nothing() {
    let mut state = empty_state();
    state.chat_area_height = 20;
    handle_mouse_left_click(&mut state, 5, 0, false, true, None);
    // peer_id is None, DM tab routing can't proceed
}

// ── start_peer_specific_nickname_edit ────────────────────────────────────

#[test]
fn test_start_nickname_edit_sets_editing_state() {
    let mut state = empty_state();
    state.own_nickname = "TestUser".to_string();
    super::start_peer_specific_nickname_edit(&mut state, "peer-1");
    assert!(state.editing_nickname);
    assert_eq!(state.editing_nickname_peer, Some("peer-1".to_string()));
}

#[test]
fn test_start_nickname_edit_uses_self_nickname_when_available() {
    let mut state = empty_state();
    state.own_nickname = "Global".to_string();
    state
        .self_nicknames_for_peers
        .insert("peer-1".to_string(), "PerPeer".to_string());
    super::start_peer_specific_nickname_edit(&mut state, "peer-1");
    assert!(state.chat_input.lines().join("").contains("PerPeer"));
}

#[test]
fn test_start_nickname_edit_falls_back_to_own_nickname() {
    let mut state = empty_state();
    state.own_nickname = "Global".to_string();
    super::start_peer_specific_nickname_edit(&mut state, "peer-1");
    assert!(state.chat_input.lines().join("").contains("Global"));
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
