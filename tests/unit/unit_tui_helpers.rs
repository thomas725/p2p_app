use super::*;
use crossterm::event::KeyCode;
use std::collections::VecDeque;

#[test]
fn peer_table_sorts_by_active_column_like_flutter() {
    use crate::tui_helpers::{sort_peers_table, PeerMessageMap};
    use std::collections::HashMap;

    let peers = VecDeque::from([
        PeerRecord {
            peer_id: "a".to_string(),
            first_seen: "t".to_string(),
            last_seen: "2024-01-01 00:00:03".to_string(),
        },
        PeerRecord {
            peer_id: "b".to_string(),
            first_seen: "t".to_string(),
            last_seen: "2024-01-01 00:00:01".to_string(),
        },
        PeerRecord {
            peer_id: "c".to_string(),
            first_seen: "t".to_string(),
            last_seen: "2024-01-01 00:00:02".to_string(),
        },
    ]);
    let mut dm: HashMap<String, VecDeque<String>> = HashMap::new();
    dm.insert("a".to_string(), VecDeque::from(["x".to_string(), "y".to_string()])); // 2 dms
    dm.insert("b".to_string(), VecDeque::from(["z".to_string()])); // 1 dm
    let msgs: VecDeque<Option<String>> =
        VecDeque::from([Some("a".to_string()), Some("a".to_string()), Some("c".to_string())]);

    // Default Flutter sort: Last Seen, descending -> c(02), a(03)? a=03 is latest.
    // last_seen desc: a(03) > c(02) > b(01)
    let rows = sort_peers_table(peers.as_slices().0, &dm, &msgs, 3, false);
    assert_eq!(rows.iter().map(|r| r.peer_id.as_str()).collect::<Vec<_>>(), vec!["a", "c", "b"]);

    // DM count ascending: b(1) < a(2) < c(0)
    let rows = sort_peers_table(peers.as_slices().0, &dm, &msgs, 1, true);
    assert_eq!(rows.iter().map(|r| r.peer_id.as_str()).collect::<Vec<_>>(), vec!["c", "b", "a"]);
    assert_eq!(rows[0].dm_count, 0);
    assert_eq!(rows[1].dm_count, 1);
    assert_eq!(rows[2].dm_count, 2);

    // Broadcast count: a=2, c=1, b=0
    let rows = sort_peers_table(peers.as_slices().0, &dm, &msgs, 2, false);
    assert_eq!(rows.iter().map(|r| r.peer_id.as_str()).collect::<Vec<_>>(), vec!["a", "c", "b"]);

    assert_eq!(dm.dm_count_for("a"), 2);
}

#[test]
fn parse_last_seen_ms_parses_flutter_format() {
    use crate::tui_helpers::parse_last_seen_ms;
    let ms = parse_last_seen_ms("2024-01-01 00:00:01");
    let direct = parse_last_seen_ms("2024-01-01T00:00:01");
    assert_eq!(ms, direct);
    assert!(ms > 0);
    assert_eq!(parse_last_seen_ms("not-a-date"), 0);
}

#[test]
fn peer_sort_and_upsert_keep_selection() {
    let mut peers = VecDeque::from([
        PeerRecord {
            peer_id: "a".to_string(),
            first_seen: "t1".to_string(),
            last_seen: "2024-01-01T00:00:01".to_string(),
        },
        PeerRecord {
            peer_id: "b".to_string(),
            first_seen: "t1".to_string(),
            last_seen: "2024-01-01T00:00:02".to_string(),
        },
    ]);
    let idx = sort_peers_by_last_seen(&mut peers, 0);
    assert_eq!(idx, 1);
    let idx2 = upsert_peer_last_seen(&mut peers, idx, "a", "2024-01-01T00:00:03");
    assert_eq!(peers[0].peer_id, "a");
    assert_eq!(idx2, 0);
}

#[test]
fn peer_sort_none_selected_and_upsert_insert_branch() {
    let mut peers: VecDeque<PeerRecord> = VecDeque::new();
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

#[test]
fn peer_table_headers_mark_active_column() {
    assert_eq!(peer_sort_indicator(true), " ▲");
    assert_eq!(peer_sort_indicator(false), " ▼");
    assert_eq!(
        peer_table_header_labels(3, false),
        vec!["Name", "DM", "Broadcast", "Last Seen ▼"]
    );
    assert_eq!(
        peer_table_header_labels(3, true),
        vec!["Name", "DM", "Broadcast", "Last Seen ▲"]
    );
    assert_eq!(
        peer_table_header_labels(0, false),
        vec!["Name ▼", "DM", "Broadcast", "Last Seen"]
    );
}

#[test]
fn peer_table_column_widths_fit_longest_content() {
    use crate::tui_helpers::{peer_table_column_widths, PeerTableRow};

    // No peers: columns are only as wide as their headers.
    let empty: Vec<PeerTableRow> = Vec::new();
    assert_eq!(peer_table_column_widths(&empty, 3, false), vec![4, 2, 9, 11]); // "Last Seen ▼"
    // The sort indicator stays inside the column it marks.
    assert_eq!(peer_table_column_widths(&empty, 0, false), vec![6, 2, 9, 9]); // "Name ▼"

    // Long cells grow their column; short cells do not.
    let rows = vec![
        PeerTableRow::new("p1", "Bob", 12, 3, "2024-01-01 00:00:03"),
        PeerTableRow::new("p2", "Avery-TestName", 9999, 1000, "2024-01-01 00:00:02"),
    ];
    assert_eq!(peer_table_column_widths(&rows, 3, false), vec![14, 4, 9, 19]);
    // The table spans only its content: every column is far narrower than a
    // typical terminal width.
    assert!(peer_table_column_widths(&rows, 3, false).iter().all(|w| *w < 100));
}

#[test]
fn peer_table_column_widths_count_wide_chars() {
    use crate::tui_helpers::{peer_table_column_widths, PeerTableRow};
    let rows = vec![PeerTableRow::new("j", "飛鳥龍", 0, 0, "t")];
    let widths = peer_table_column_widths(&rows, 0, false);
    // 飛鳥龍 is 3 chars but 6 terminal columns wide (1 wide char + 2 wide chars).
    assert_eq!(widths[0], 6);
    assert_ne!(widths[0], 3);
}

#[test]
fn peer_table_visible_range_window_follows_selection() {
    // One full page from the start: rows 0..5.
    assert_eq!(peer_table_visible_range(0, Some(2), 10, 5), (0, 5));
    // Selection on the next page scrolls down exactly one page.
    assert_eq!(peer_table_visible_range(0, Some(9), 10, 5), (5, 10));
    // The offset anchors the window when the selection is already visible.
    assert_eq!(peer_table_visible_range(3, Some(6), 10, 5), (3, 8));
    // Fill downward from the offset, then scroll until the selection shows.
    assert_eq!(peer_table_visible_range(0, Some(6), 10, 5), (2, 7));
    // A page larger than the list shows everything from the start.
    assert_eq!(peer_table_visible_range(0, Some(9), 10, 20), (0, 10));
    // PageUp then PageDown (selection -10 then +10) restores a full-page jump.
    assert_eq!(peer_table_visible_range(0, Some(19), 30, 10), (10, 20));
}

#[test]
fn peer_table_visible_range_empty_and_clamped() {
    // Empty list: an empty window.
    assert_eq!(peer_table_visible_range(0, Some(0), 0, 5), (0, 0));
    // Selection beyond the list clamps to the last row.
    assert_eq!(peer_table_visible_range(0, Some(999), 10, 5), (5, 10));
    // An offset past the last row clamps to it.
    assert_eq!(peer_table_visible_range(999, Some(9), 10, 5), (9, 10));
    // A page height of zero still shows something, not a hang.
    assert_eq!(peer_table_visible_range(0, Some(9), 10, 0), (9, 10));
}
