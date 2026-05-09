//! Tests for TUI helper functions

#[test]
fn test_sort_peers_by_last_seen() {
    use p2p_app::tui_helpers::sort_peers_by_last_seen;
    use std::collections::VecDeque;

    let mut peers = VecDeque::from(vec![
        (
            "peer1".to_string(),
            "10:00".to_string(),
            "10:00".to_string(),
        ),
        (
            "peer2".to_string(),
            "09:00".to_string(),
            "09:00".to_string(),
        ),
        (
            "peer3".to_string(),
            "11:00".to_string(),
            "11:00".to_string(),
        ),
    ]);

    let _new_sel = sort_peers_by_last_seen(&mut peers, 0);

    assert_eq!(peers[0].0, "peer3");
    assert_eq!(peers[1].0, "peer1");
    assert_eq!(peers[2].0, "peer2");
}

#[test]
fn test_upsert_peer_last_seen() {
    use p2p_app::tui_helpers::upsert_peer_last_seen;
    use std::collections::VecDeque;

    let mut peers = VecDeque::new();
    let sel = upsert_peer_last_seen(&mut peers, 0, "new_peer", "12:00");
    assert_eq!(peers.len(), 1);
    assert_eq!(sel, 0);
}

#[test]
fn test_is_nickname_update() {
    use p2p_app::tui_helpers::is_nickname_update;

    assert!(is_nickname_update("", Some("Alice")));
    assert!(!is_nickname_update("Hello", Some("Alice")));
    assert!(!is_nickname_update("Hello", None));
}

#[test]
fn test_calculate_auto_scroll() {
    use p2p_app::tui_helpers::calculate_auto_scroll;

    assert_eq!(calculate_auto_scroll(100, 20), 80);
    assert_eq!(calculate_auto_scroll(10, 20), 0);
}

#[test]
fn test_calculate_visible_range() {
    use p2p_app::tui_helpers::calculate_visible_range;

    let (start, end) = calculate_visible_range(100, 50, 20);
    assert_eq!(start, 50);
    assert_eq!(end, 70);
}

#[test]
fn test_parse_command() {
    use p2p_app::tui_helpers::parse_command;

    assert_eq!(parse_command("/nick Alice"), Some(("/nick", "Alice")));
    assert_eq!(parse_command("/quit"), Some(("/quit", "")));
    assert_eq!(parse_command("hello"), None);
}

#[test]
fn test_is_command() {
    use p2p_app::tui_helpers::is_command;

    assert!(is_command("/nick"));
    assert!(is_command("/msg peer hello"));
    assert!(!is_command("Just a message"));
}

#[test]
fn test_get_command_name() {
    use p2p_app::tui_helpers::get_command_name;

    assert_eq!(get_command_name("/nick Alice"), Some("/nick"));
    assert_eq!(get_command_name("/quit"), Some("/quit"));
    assert_eq!(get_command_name("hello"), None);
}

#[test]
fn test_get_command_arg() {
    use p2p_app::tui_helpers::get_command_arg;

    assert_eq!(get_command_arg("/nick Alice"), Some("Alice"));
    assert_eq!(get_command_arg("/nick"), None);
    assert_eq!(get_command_arg("hello"), None);
}

#[test]
fn test_validate_nickname() {
    use p2p_app::tui_helpers::validate_nickname;

    assert!(validate_nickname("Alice"));
    assert!(validate_nickname("Bob-123"));
    assert!(validate_nickname("TestNick"));
    assert!(!validate_nickname(""));
    assert!(!validate_nickname(&"a".repeat(21)));
    assert!(!validate_nickname("Bob@home"));
}

#[test]
fn test_truncate_message() {
    use p2p_app::tui_helpers::truncate_message;

    assert_eq!(truncate_message("hello", 10), "hello");
    assert_eq!(truncate_message("hello world!", 8), "hello...");
}

#[test]
fn test_message_line_count() {
    use p2p_app::tui_helpers::message_line_count;

    assert_eq!(message_line_count("short", 80), 1);
    assert_eq!(message_line_count("line1\nline2", 80), 2);
    // Test wrapping: 50 chars at width 20 = 3 lines
    let long = "a".repeat(50);
    assert_eq!(message_line_count(&long, 20), 3);
}

// Skip: conflicts with fmt::short_peer_id
// Use p2p_app::fmt::short_peer_id instead

#[test]
fn test_parse_latency() {
    use p2p_app::tui_helpers::parse_latency;

    assert_eq!(parse_latency("<1ms"), Some(0.5));
    assert_eq!(parse_latency("100ms"), Some(100.0));
    assert_eq!(parse_latency("1.5s"), Some(1500.0));
    assert_eq!(parse_latency("invalid"), None);
}

#[test]
fn test_is_at_bottom() {
    use p2p_app::tui_helpers::is_at_bottom;

    assert!(is_at_bottom(80, 100, 20));
    assert!(!is_at_bottom(50, 100, 20));
}

#[test]
fn test_format_peer_list_item() {
    use p2p_app::tui_helpers::format_peer_list_item;

    let item = format_peer_list_item("12D3KooWSkP1pEPy2", Some("Alice"), "10:00");
    assert!(item.contains("Alice"));

    let item2 = format_peer_list_item("12D3KooWSkP1pEPy2", None, "10:00");
    assert!(item2.contains("12D3KooW"));
}

// Scroll handler tests
#[test]
fn test_disable_auto_scroll_to_max() {
    use p2p_app::tui_helpers::{PAGE_SIZE, WHEEL_SCROLL_LINES, disable_auto_scroll_to_max};

    // Test constants
    assert_eq!(PAGE_SIZE, 8);
    assert_eq!(WHEEL_SCROLL_LINES, 3);

    // Test function
    let mut auto = true;
    let mut offset = 50;
    disable_auto_scroll_to_max(&mut auto, &mut offset, 100);
    assert!(!auto);
    assert_eq!(offset, 100);
}

#[test]
fn test_scroll_up_lines() {
    use p2p_app::tui_helpers::scroll_up_lines;

    let mut offset = 10;
    scroll_up_lines(&mut offset, 1);
    assert_eq!(offset, 9);

    offset = 0;
    scroll_up_lines(&mut offset, 5);
    assert_eq!(offset, 0); // saturating
}

#[test]
fn test_scroll_down_lines() {
    use p2p_app::tui_helpers::scroll_down_lines;

    let mut offset = 0;
    let mut auto = false;
    scroll_down_lines(&mut offset, &mut auto, 1, 50);
    assert_eq!(offset, 1);
    assert!(!auto); // not at max yet

    // At max offset - should enable auto-scroll
    let mut offset2 = 49;
    let mut auto2 = false;
    scroll_down_lines(&mut offset2, &mut auto2, 1, 50);
    assert_eq!(offset2, 50);
    assert!(auto2);
}

#[test]
fn test_calc_max_scroll() {
    use p2p_app::tui_helpers::calc_max_scroll;

    assert_eq!(calc_max_scroll(100, 20), 80);
    assert_eq!(calc_max_scroll(10, 20), 0); // saturating
}

#[test]
fn test_handle_scroll_key_for_section() {
    use p2p_app::tui_helpers::handle_scroll_key_for_section;

    // Up key
    let (offset, auto) = handle_scroll_key_for_section("Up", 50, false, 80);
    assert_eq!(offset, 49);
    assert!(!auto);

    // Down key
    let (offset, _auto) = handle_scroll_key_for_section("Down", 50, false, 80);
    assert_eq!(offset, 51);

    // PageUp
    let (offset, _auto) = handle_scroll_key_for_section("PageUp", 50, false, 80);
    assert_eq!(offset, 42); // 50 - 8

    // PageDown - auto at max
    let (offset, auto) = handle_scroll_key_for_section("PageDown", 78, false, 80);
    assert!(auto);
    assert_eq!(offset, 80);

    // Home - go to top
    let (offset, auto) = handle_scroll_key_for_section("Home", 50, true, 80);
    assert_eq!(offset, 0);
    assert!(!auto);

    // End - go to bottom
    let (_offset, auto) = handle_scroll_key_for_section("End", 50, false, 80);
    assert!(auto);

    // Unknown key - no change
    let (offset, auto) = handle_scroll_key_for_section("Unknown", 50, false, 80);
    assert_eq!(offset, 50);
    assert!(!auto);
}

#[test]
fn test_handle_mouse_wheel_scroll() {
    use p2p_app::tui_helpers::handle_mouse_wheel_scroll;

    let offset = handle_mouse_wheel_scroll("up", 10, 50);
    assert_eq!(offset, 7); // 10 - 3

    let offset = handle_mouse_wheel_scroll("down", 10, 50);
    assert_eq!(offset, 13); // 10 + 3

    let offset = handle_mouse_wheel_scroll("unknown", 10, 50);
    assert_eq!(offset, 10); // no change

    // Edge: can't go below 0
    let offset = handle_mouse_wheel_scroll("up", 0, 50);
    assert_eq!(offset, 0);
}

#[test]
fn test_next_tab_index() {
    use p2p_app::tui_helpers::next_tab_index;

    // Next tab
    assert_eq!(next_tab_index(0, 1, 3), 1);
    assert_eq!(next_tab_index(1, 1, 3), 2);
    assert_eq!(next_tab_index(2, 1, 3), 0); // wraps

    // Previous tab
    assert_eq!(next_tab_index(0, -1, 3), 2); // wraps back
    assert_eq!(next_tab_index(1, -1, 3), 0);

    // Zero tabs
    assert_eq!(next_tab_index(0, 1, 0), 0);
}

#[test]
fn test_relabel_dm_transcript_replaces_matching_lines() {
    use p2p_app::tui_helpers::relabel_dm_transcript;
    use std::collections::VecDeque;
    let mut msgs: VecDeque<String> = VecDeque::from([
        "[Alice] hello".into(),
        "[Bob] hi there".into(),
        "[Alice] how are you?".into(),
    ]);
    relabel_dm_transcript(&mut msgs, "Alice", "AliceNew");
    assert_eq!(msgs[0], "[AliceNew] hello");
    assert_eq!(msgs[1], "[Bob] hi there"); // untouched
    assert_eq!(msgs[2], "[AliceNew] how are you?");
}

#[test]
fn test_relabel_dm_transcript_empty() {
    use p2p_app::tui_helpers::relabel_dm_transcript;
    use std::collections::VecDeque;
    let mut msgs: VecDeque<String> = VecDeque::new();
    relabel_dm_transcript(&mut msgs, "Alice", "New"); // must not panic
    assert!(msgs.is_empty());
}

#[test]
fn test_relabel_dm_transcript_no_match() {
    use p2p_app::tui_helpers::relabel_dm_transcript;
    use std::collections::VecDeque;
    let mut msgs: VecDeque<String> = VecDeque::from(["[Bob] unchanged".into()]);
    relabel_dm_transcript(&mut msgs, "Alice", "New");
    assert_eq!(msgs[0], "[Bob] unchanged");
}

#[test]
fn test_sort_peers_preserves_selection() {
    use p2p_app::tui_helpers::sort_peers_by_last_seen;
    use std::collections::VecDeque;
    let mut peers = VecDeque::from([
        (
            "a".into(),
            "2024-01-01 00:00:00".into(),
            "2024-01-01 00:00:00".into(),
        ),
        (
            "b".into(),
            "2024-01-03 00:00:00".into(),
            "2024-01-03 00:00:00".into(),
        ),
        (
            "c".into(),
            "2024-01-02 00:00:00".into(),
            "2024-01-02 00:00:00".into(),
        ),
    ]);
    // Select peer "a" (index 0 before sort)
    let new_sel = sort_peers_by_last_seen(&mut peers, 0);
    // After sort by desc: b, c, a — peer "a" should now be at index 2
    assert_eq!(peers[0].0, "b");
    assert_eq!(peers[1].0, "c");
    assert_eq!(peers[2].0, "a");
    assert_eq!(
        new_sel, 2,
        "selection should track peer 'a' to its new index"
    );
}

#[test]
fn test_sort_peers_empty() {
    use p2p_app::tui_helpers::sort_peers_by_last_seen;
    use std::collections::VecDeque;
    let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
    let sel = sort_peers_by_last_seen(&mut peers, 0);
    assert_eq!(sel, 0);
}

#[test]
fn test_upsert_adds_new_peer() {
    use p2p_app::tui_helpers::upsert_peer_last_seen;
    use std::collections::VecDeque;
    let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
    upsert_peer_last_seen(&mut peers, 0, "new-peer", "2024-05-01 12:00:00");
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].0, "new-peer");
    assert_eq!(peers[0].2, "2024-05-01 12:00:00");
}

#[test]
fn test_upsert_updates_existing_peer() {
    use p2p_app::tui_helpers::upsert_peer_last_seen;
    use std::collections::VecDeque;
    let mut peers = VecDeque::from([(
        "p1".into(),
        "2024-01-01 00:00:00".into(),
        "2024-01-01 00:00:00".into(),
    )]);
    upsert_peer_last_seen(&mut peers, 0, "p1", "2024-06-01 00:00:00");
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].2, "2024-06-01 00:00:00");
}

#[test]
fn test_handle_scroll_end_sets_auto_scroll() {
    use p2p_app::tui_helpers::handle_scroll_key_for_section;
    let (offset, auto) = handle_scroll_key_for_section("End", 5, false, 100);
    assert!(auto);
    // End doesn't change offset — it just re-enables auto-scroll
    assert_eq!(offset, 100);
}

#[test]
fn test_handle_scroll_home_clears_auto_scroll() {
    use p2p_app::tui_helpers::handle_scroll_key_for_section;
    let (offset, auto) = handle_scroll_key_for_section("Home", 50, true, 100);
    assert_eq!(offset, 0);
    assert!(!auto);
}

#[test]
fn test_handle_scroll_up_from_auto_jumps_to_bottom_first() {
    use p2p_app::tui_helpers::handle_scroll_key_for_section;
    // When auto_scroll is true, pressing Up should first pin to max then scroll up 1
    let (offset, auto) = handle_scroll_key_for_section("Up", 0, true, 50);
    assert!(!auto);
    assert_eq!(offset, 49); // pinned to 50 then scrolled up 1
}

#[test]
fn test_handle_scroll_page_down_clamps_at_max() {
    use p2p_app::tui_helpers::handle_scroll_key_for_section;
    let (offset, auto) = handle_scroll_key_for_section("PageDown", 95, false, 100);
    assert_eq!(offset, 100);
    assert!(auto); // reached max so auto-scroll re-engages
}

#[test]
fn test_mouse_wheel_scroll_ignored_when_auto_scroll() {
    use p2p_app::tui_helpers::handle_mouse_wheel_scroll;
    // Existing function doesn't take auto_scroll — test the no-change path via direction ""
    let offset = handle_mouse_wheel_scroll("", 30, 100);
    assert_eq!(offset, 30); // unknown direction is a no-op
}

#[test]
fn test_next_tab_index_wraps_forward() {
    use p2p_app::tui_helpers::next_tab_index;
    assert_eq!(next_tab_index(3, 1, 4), 0); // 3+1=4 wraps to 0
}

#[test]
fn test_next_tab_index_wraps_backward() {
    use p2p_app::tui_helpers::next_tab_index;
    assert_eq!(next_tab_index(0, -1, 4), 3); // 0-1=-1 wraps to 3
}

#[test]
fn test_next_tab_index_zero_tabs() {
    use p2p_app::tui_helpers::next_tab_index;
    assert_eq!(next_tab_index(0, 1, 0), 0); // must not panic
}

// ── validate_nickname edge cases ──────────────────────────────────────────────

#[test]
fn test_validate_nickname_valid() {
    use p2p_app::tui_helpers::validate_nickname;
    assert!(validate_nickname("alice"));
    assert!(validate_nickname("Alice123"));
    assert!(validate_nickname("cool-nick"));
    assert!(validate_nickname("A")); // single char
}

#[test]
fn test_validate_nickname_empty_rejected() {
    use p2p_app::tui_helpers::validate_nickname;
    assert!(!validate_nickname(""));
}

#[test]
fn test_validate_nickname_too_long_rejected() {
    use p2p_app::tui_helpers::validate_nickname;
    assert!(!validate_nickname("this-nickname-is-way-too-long-123"));
}

#[test]
fn test_validate_nickname_invalid_chars_rejected() {
    use p2p_app::tui_helpers::validate_nickname;
    assert!(!validate_nickname("has space"));
    assert!(!validate_nickname("has@symbol"));
    assert!(!validate_nickname("has.dot"));
}

#[test]
fn test_validate_nickname_exactly_20_chars() {
    use p2p_app::tui_helpers::validate_nickname;
    assert!(validate_nickname("abcdefghijklmnopqrst")); // exactly 20
    assert!(!validate_nickname("abcdefghijklmnopqrstu")); // 21
}

// ── truncate_message ──────────────────────────────────────────────────────────

#[test]
fn test_truncate_message_short_unchanged() {
    use p2p_app::tui_helpers::truncate_message;
    assert_eq!(truncate_message("hello", 20), "hello");
}

#[test]
fn test_truncate_message_exact_length_unchanged() {
    use p2p_app::tui_helpers::truncate_message;
    assert_eq!(truncate_message("hello", 5), "hello");
}

#[test]
fn test_truncate_message_long_gets_ellipsis() {
    use p2p_app::tui_helpers::truncate_message;
    let result = truncate_message("hello world this is a long message", 10);
    assert!(result.ends_with("..."));
    assert!(result.len() <= 10);
}

// ── message_line_count ────────────────────────────────────────────────────────

#[test]
fn test_message_line_count_zero_width_returns_one() {
    use p2p_app::tui_helpers::message_line_count;
    assert_eq!(message_line_count("hello", 0), 1);
}

#[test]
fn test_message_line_count_empty_string() {
    use p2p_app::tui_helpers::message_line_count;
    // Empty string has no lines(), returns max(0,1) = 1
    assert_eq!(message_line_count("", 80), 1);
}

#[test]
fn test_message_line_count_single_short_line() {
    use p2p_app::tui_helpers::message_line_count;
    assert_eq!(message_line_count("hello", 80), 1);
}

#[test]
fn test_message_line_count_wraps_long_line() {
    use p2p_app::tui_helpers::message_line_count;
    // 100 chars in a 40-wide terminal → ceil(100/40) = 3 lines
    let msg = "a".repeat(100);
    assert_eq!(message_line_count(&msg, 40), 3);
}

#[test]
fn test_message_line_count_multiline_with_empty_line() {
    use p2p_app::tui_helpers::message_line_count;
    // "hello\n\nworld" → 3 lines (hello=1, empty=1, world=1)
    assert_eq!(message_line_count("hello\n\nworld", 80), 3);
}

// ── format_peer_list_item ─────────────────────────────────────────────────────

#[test]
fn test_format_peer_list_item_with_nickname() {
    use p2p_app::tui_helpers::format_peer_list_item;
    let result = format_peer_list_item("12D3KooWABCDEFGH", Some("Alice"), "2024-01-01");
    assert!(result.contains("Alice"));
    assert!(result.contains("ABCDEFGH"));
    assert!(result.contains("2024-01-01"));
}

#[test]
fn test_format_peer_list_item_no_nickname() {
    use p2p_app::tui_helpers::format_peer_list_item;
    let result = format_peer_list_item("12D3KooWABCDEFGH", None, "2024-01-01");
    assert!(result.contains("ABCDEFGH"));
    assert!(result.contains("2024-01-01"));
    assert!(!result.contains("Alice"));
}

// ── is_at_bottom ─────────────────────────────────────────────────────────────

#[test]
fn test_is_at_bottom_when_at_end() {
    use p2p_app::tui_helpers::is_at_bottom;
    assert!(is_at_bottom(90, 100, 10)); // 90 >= 100-10=90
}

#[test]
fn test_is_at_bottom_when_not_at_end() {
    use p2p_app::tui_helpers::is_at_bottom;
    assert!(!is_at_bottom(50, 100, 10)); // 50 < 90
}

#[test]
fn test_is_at_bottom_empty_list() {
    use p2p_app::tui_helpers::is_at_bottom;
    assert!(is_at_bottom(0, 0, 10)); // 0 >= 0.saturating_sub(10)=0
}

// ── parse_command edge cases ──────────────────────────────────────────────────
// parse_command returns Option<(&str,&str)>; get_command_name/arg take &str directly

#[test]
fn test_parse_command_with_arg() {
    use p2p_app::tui_helpers::{parse_command, get_command_name, get_command_arg};
    // parse_command returns the raw tuple
    let parsed = parse_command("/nick alice");
    assert_eq!(parsed, Some(("/nick", "alice")));
    // get_command_name/arg accept the original &str input
    assert_eq!(get_command_name("/nick alice"), Some("/nick"));
    assert_eq!(get_command_arg("/nick alice"), Some("alice"));
}

#[test]
fn test_parse_command_no_arg() {
    use p2p_app::tui_helpers::{parse_command, get_command_name, get_command_arg};
    let parsed = parse_command("/quit");
    assert_eq!(parsed, Some(("/quit", "")));
    assert_eq!(get_command_name("/quit"), Some("/quit"));
    assert!(get_command_arg("/quit").is_none());
}

#[test]
fn test_parse_command_not_a_command() {
    use p2p_app::tui_helpers::{parse_command, get_command_name};
    assert!(parse_command("hello world").is_none());
    assert!(get_command_name("hello world").is_none());
}

#[test]
fn test_parse_command_empty_arg_stripped() {
    use p2p_app::tui_helpers::get_command_arg;
    // trailing space — get_command_arg returns None for empty arg
    assert!(get_command_arg("/nick ").is_none());
}

// ── scroll calc helpers ───────────────────────────────────────────────────────

#[test]
fn test_calc_max_scroll_fewer_items_than_visible() {
    use p2p_app::tui_helpers::calc_max_scroll;
    assert_eq!(calc_max_scroll(5, 10), 0);
}

#[test]
fn test_calc_max_scroll_more_items_than_visible() {
    use p2p_app::tui_helpers::calc_max_scroll;
    assert_eq!(calc_max_scroll(100, 20), 80);
}

#[test]
fn test_calculate_visible_range_normal() {
    use p2p_app::tui_helpers::calculate_visible_range;
    let (start, end) = calculate_visible_range(100, 10, 20);
    assert_eq!(start, 10);
    assert_eq!(end, 30);
}

#[test]
fn test_calculate_visible_range_near_end() {
    use p2p_app::tui_helpers::calculate_visible_range;
    let (start, end) = calculate_visible_range(100, 90, 20);
    assert_eq!(start, 90);
    assert_eq!(end, 100); // clamped
}

#[test]
fn test_calculate_visible_range_empty() {
    use p2p_app::tui_helpers::calculate_visible_range;
    let (start, end) = calculate_visible_range(0, 0, 20);
    assert_eq!(start, 0);
    assert_eq!(end, 0);
}
