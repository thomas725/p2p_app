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

    let new_sel = sort_peers_by_last_seen(&mut peers, 0);

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
    let long = std::iter::repeat('a').take(50).collect::<String>();
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
