//! Tests for logging.rs module

#[test]
fn test_push_log() {
    p2p_app::logging::push_log("test log message");
}

#[test]
fn test_strip_ansi_codes() {
    let input = "\x1b[31m\x1b[0m red text";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert!(!result.contains('\x1b'));
}

#[test]
fn test_strip_ansi_codes_plain() {
    let input = "plain text";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert_eq!(result, "plain text");
}

// ── get_tui_logs / clear_tui_logs ─────────────────────────────────────────────

#[test]
fn test_get_tui_logs_returns_vec() {
    // get_tui_logs returns empty vec or whatever is in the global store
    let logs = p2p_app::logging::get_tui_logs();
    // Just check it doesn't panic and returns a Vec
    let _ = logs.len();
}

#[test]
fn test_push_log_appears_in_get_tui_logs() {
    let unique = format!("unique-marker-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    p2p_app::logging::push_log(unique.clone());
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains(&unique)),
        "pushed log should appear in get_tui_logs");
}

#[test]
fn test_clear_tui_logs_empties_store() {
    p2p_app::logging::push_log("to-be-cleared");
    p2p_app::logging::clear_tui_logs();
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.is_empty(), "logs should be empty after clear");
}

// ── p2plog level aliases ──────────────────────────────────────────────────────

#[test]
fn test_p2plog_debug_contains_level() {
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_debug("debug-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("DEBUG") && l.contains("debug-msg")));
}

#[test]
fn test_p2plog_info_contains_level() {
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_info("info-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("INFO") && l.contains("info-msg")));
}

#[test]
fn test_p2plog_warn_contains_level() {
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_warn("warn-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("WARN") && l.contains("warn-msg")));
}

#[test]
fn test_p2plog_error_contains_level() {
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_error("error-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("ERROR") && l.contains("error-msg")));
}

// ── strip_ansi_codes edge cases ───────────────────────────────────────────────

#[test]
fn test_strip_ansi_codes_empty_string() {
    assert_eq!(p2p_app::logging::strip_ansi_codes(""), "");
}

#[test]
fn test_strip_ansi_codes_bold() {
    let input = "\x1b[1mbold\x1b[0m";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert_eq!(result, "bold");
}

#[test]
fn test_strip_ansi_codes_multiple_sequences() {
    let input = "\x1b[31mred\x1b[0m and \x1b[32mgreen\x1b[0m";
    let result = p2p_app::logging::strip_ansi_codes(input);
    assert_eq!(result, "red and green");
}

#[test]
fn test_strip_ansi_codes_preserves_all_non_escape_chars() {
    let input = "hello\nworld\t!";
    assert_eq!(p2p_app::logging::strip_ansi_codes(input), input);
}
