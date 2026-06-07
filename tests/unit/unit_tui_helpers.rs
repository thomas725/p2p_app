use super::*;
use crossterm::event::KeyCode;
use std::collections::VecDeque;

#[test]
fn peer_sort_and_upsert_keep_selection() {
    let mut peers = VecDeque::from([
        (
            "a".to_string(),
            "t1".to_string(),
            "2024-01-01T00:00:01".to_string(),
        ),
        (
            "b".to_string(),
            "t1".to_string(),
            "2024-01-01T00:00:02".to_string(),
        ),
    ]);
    let idx = sort_peers_by_last_seen(&mut peers, 0);
    assert_eq!(idx, 1);
    let idx2 = upsert_peer_last_seen(&mut peers, idx, "a", "2024-01-01T00:00:03");
    assert_eq!(peers[0].0, "a");
    assert_eq!(idx2, 0);
}

#[test]
fn peer_sort_none_selected_and_upsert_insert_branch() {
    let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
    let idx = sort_peers_by_last_seen(&mut peers, 5);
    assert_eq!(idx, 0);

    let idx2 = upsert_peer_last_seen(&mut peers, 0, "x", "2024-01-01T00:00:00");
    assert_eq!(idx2, 0);
    assert_eq!(peers.len(), 1);
}

#[test]
fn nickname_and_validation_helpers() {
    assert!(is_nickname_update("", Some("nick")));
    assert!(!is_nickname_update("hi", Some("nick")));
    assert!(validate_nickname("peer-123"));
    assert!(!validate_nickname(""));
    assert!(!validate_nickname("bad nick"));
}

#[test]
fn range_and_truncate_helpers() {
    assert_eq!(calculate_visible_range(10, 4, 3), (4, 7));
    assert_eq!(truncate_message("hello", 10), "hello");
    assert_eq!(truncate_message("abcdefghij", 6), "abc...");
}

#[test]
fn latency_and_bottom_helpers() {
    assert_eq!(parse_latency("<1ms"), Some(0.5));
    assert_eq!(parse_latency("12ms"), Some(12.0));
    assert_eq!(parse_latency("1.5s"), Some(1500.0));
    assert_eq!(parse_latency("bad"), None);
    assert!(is_at_bottom(7, 10, 3));
    assert!(!is_at_bottom(6, 10, 3));
}

#[test]
fn peer_item_and_lines_helpers() {
    assert_eq!(crate::count_lines("x", 0), 1);
    assert_eq!(crate::count_lines("abcdefghij", 5), 2);
}

#[test]
fn scroll_helpers() {
    let mut auto = true;
    let mut off = 2;
    disable_auto_scroll_to_max(&mut auto, &mut off, 9);
    assert!(!auto);
    assert_eq!(off, 9);

    scroll_up_lines(&mut off, 3);
    assert_eq!(off, 6);
    scroll_down_lines(&mut off, &mut auto, 3, 9);
    assert!(auto);
    assert_eq!(off, 9);

    assert_eq!(key_code_to_scroll_action(KeyCode::Up), Some("Up"));
    assert_eq!(key_code_to_scroll_action(KeyCode::Down), Some("Down"));
    assert_eq!(key_code_to_scroll_action(KeyCode::PageUp), Some("PageUp"));
    assert_eq!(
        key_code_to_scroll_action(KeyCode::PageDown),
        Some("PageDown")
    );
    assert_eq!(key_code_to_scroll_action(KeyCode::Home), Some("Home"));
    assert_eq!(key_code_to_scroll_action(KeyCode::End), Some("End"));
    assert_eq!(key_code_to_scroll_action(KeyCode::Char('x')), None);

    assert_eq!(handle_scroll_key_for_section("Up", 9, true, 9), (8, false));
    assert_eq!(
        handle_scroll_key_for_section("Down", 0, false, 9),
        (1, false)
    );
    assert_eq!(
        handle_scroll_key_for_section("PageUp", 9, false, 9),
        (1, false)
    );
    assert_eq!(
        handle_scroll_key_for_section("PageDown", 0, false, 9),
        (8, false)
    );
    assert_eq!(
        handle_scroll_key_for_section("Home", 4, true, 9),
        (0, false)
    );
    assert_eq!(handle_scroll_key_for_section("End", 4, false, 9), (9, true));
    assert_eq!(
        handle_scroll_key_for_section("Unknown", 3, false, 9),
        (3, false)
    );
}

#[test]
fn transcript_and_tabs_helpers() {
    let mut lines = VecDeque::from(["[old] hello".to_string(), "[other] keep".to_string()]);
    relabel_dm_transcript(&mut lines, "old", "new");
    assert_eq!(lines[0], "[new] hello");
    assert_eq!(next_tab_index(0, -1, 4), 3);
    assert_eq!(next_tab_index(3, 1, 4), 0);
    assert_eq!(next_tab_index(0, 1, 0), 0);
}
