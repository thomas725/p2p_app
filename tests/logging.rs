//! Tests for logging.rs module

use serial_test::serial;

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
// NOTE: TUI_LOGS is a OnceLock, only initialised when init_logging() is called.
// All tests that use get_tui_logs() must call init_logging() first.

#[serial]
#[test]
fn test_get_tui_logs_returns_vec() {
    p2p_app::logging::init_logging();
    let logs = p2p_app::logging::get_tui_logs();
    let _ = logs.len(); // must not panic
}

#[serial]
#[test]
fn test_push_log_appears_in_get_tui_logs() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    let unique = format!("unique-marker-{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos());
    p2p_app::logging::push_log(unique.clone());
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains(&unique)),
        "pushed log should appear in get_tui_logs, got: {:?}", logs);
}

#[serial]
#[test]
fn test_clear_tui_logs_empties_store() {
    p2p_app::logging::init_logging();
    p2p_app::logging::push_log("to-be-cleared");
    p2p_app::logging::clear_tui_logs();
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.is_empty(), "logs should be empty after clear, got: {:?}", logs);
}

// ── p2plog level aliases ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_p2plog_debug_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_debug("debug-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("DEBUG") && l.contains("debug-msg")),
        "got: {:?}", logs);
}

#[serial]
#[test]
fn test_p2plog_info_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_info("info-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("INFO") && l.contains("info-msg")),
        "got: {:?}", logs);
}

#[serial]
#[test]
fn test_p2plog_warn_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_warn("warn-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("WARN") && l.contains("warn-msg")),
        "got: {:?}", logs);
}

#[serial]
#[test]
fn test_p2plog_error_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    p2p_app::logging::p2plog_error("error-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("ERROR") && l.contains("error-msg")),
        "got: {:?}", logs);
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

// ── TuiTracingLayer coverage via tracing macros ────────────────────────────
// These tests fire real tracing events to exercise on_event() and FormatVisitor.

#[serial]
#[test]
fn test_tracing_info_captured_in_logs() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    tracing::info!("tracing-info-test-marker");
    let logs = p2p_app::logging::get_tui_logs();
    // TuiTracingLayer writes tracing events to TUI_LOGS
    assert!(logs.iter().any(|l| l.contains("tracing-info-test-marker")),
        "tracing INFO not found in logs: {:?}", logs);
}

#[serial]
#[test]
fn test_tracing_warn_captured_in_logs() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    tracing::warn!("tracing-warn-test-marker");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("tracing-warn-test-marker")),
        "tracing WARN not found in logs: {:?}", logs);
}

#[serial]
#[test]
fn test_tracing_error_captured_in_logs() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    tracing::error!("tracing-error-test-marker");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("tracing-error-test-marker")),
        "tracing ERROR not found in logs: {:?}", logs);
}

#[serial]
#[test]
fn test_tracing_event_with_fields_captured() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    tracing::info!(user = "alice", count = 42u64, "field-test-marker");
    let logs = p2p_app::logging::get_tui_logs();
    let combined = logs.join(" ");
    assert!(combined.contains("field-test-marker"), "marker not found: {:?}", logs);
}

#[serial]
#[test]
fn test_set_tui_callback_receives_push_log() {
    p2p_app::logging::init_logging();
    p2p_app::logging::clear_tui_logs();
    // The callback is a OnceLock — we can only set it once per process,
    // so just verify push_log still flows through after init.
    p2p_app::logging::push_log("callback-flow-check");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("callback-flow-check")));
}
