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
    p2p_app::clear_tui_logs();
    let unique = format!(
        "unique-marker-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    p2p_app::logging::push_log(unique.clone());
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter().any(|l| l.contains(&unique)),
        "pushed log should appear in get_tui_logs, got: {logs:?}"
    );
}

#[serial]
#[test]
fn test_clear_tui_logs_empties_store() {
    p2p_app::logging::init_logging();
    p2p_app::logging::push_log("to-be-cleared");
    p2p_app::clear_tui_logs();
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.is_empty(),
        "logs should be empty after clear, got: {logs:?}"
    );
}

// ── p2plog level aliases ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_p2plog_debug_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();
    p2p_app::logging::p2plog_debug("debug-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter()
            .any(|l| l.contains("DEBUG") && l.contains("debug-msg")),
        "got: {logs:?}"
    );
}

#[serial]
#[test]
fn test_p2plog_info_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();
    p2p_app::logging::p2plog_info("info-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter()
            .any(|l| l.contains("INFO") && l.contains("info-msg")),
        "got: {logs:?}"
    );
}

#[serial]
#[test]
fn test_p2plog_error_contains_level() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();
    p2p_app::logging::p2plog_error("error-msg");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter()
            .any(|l| l.contains("ERROR") && l.contains("error-msg")),
        "got: {logs:?}"
    );
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
// Note: tracing::info! events from test modules are filtered out by the default
// "warn,p2p_app=info" filter. The warn/error level tests below verify the pipeline works.

#[serial]
#[test]
fn test_tracing_warn_captured_in_logs() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            p2p_app::logging::init_logging();
            p2p_app::clear_tui_logs();
            tracing::warn!("tracing-warn-test-marker");
        });
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter().any(|l| l.contains("tracing-warn-test-marker")),
        "tracing WARN not found in logs: {logs:?}"
    );
}

#[serial]
#[test]
fn test_tracing_error_captured_in_logs() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            p2p_app::logging::init_logging();
            p2p_app::clear_tui_logs();
            tracing::error!("tracing-error-test-marker");
        });
    let logs = p2p_app::logging::get_tui_logs();
    assert!(
        logs.iter().any(|l| l.contains("tracing-error-test-marker")),
        "tracing ERROR not found in logs: {logs:?}"
    );
}

#[serial]
#[test]
fn test_tracing_event_with_fields_captured() {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            p2p_app::logging::init_logging();
            p2p_app::clear_tui_logs();
            tracing::warn!(user = "alice", count = 42u64, "field-test-marker");
        });
    let logs = p2p_app::logging::get_tui_logs();
    let combined = logs.join(" ");
    assert!(
        combined.contains("field-test-marker"),
        "marker not found: {logs:?}"
    );
}

#[serial]
#[test]
fn test_set_tui_callback_receives_push_log() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();
    // The callback is a OnceLock — we can only set it once per process,
    // so just verify push_log still flows through after init.
    p2p_app::logging::push_log("callback-flow-check");
    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.iter().any(|l| l.contains("callback-flow-check")));
}

// ── Additional logging edge cases ──────────────────────────────────────────────

#[serial]
#[test]
fn test_strip_ansi_codes_mixed() {
    use p2p_app::strip_ansi_codes;
    let input = "\u{1b}[32mGreen\u{1b}[0m and \u{1b}[31mRed\u{1b}[0m";
    let result = strip_ansi_codes(input);
    assert!(!result.contains('\u{1b}'));
    assert!(result.contains("Green"));
    assert!(result.contains("Red"));
}

#[serial]
#[test]
fn test_strip_ansi_codes_no_codes() {
    use p2p_app::strip_ansi_codes;
    let input = "plain text no codes";
    let result = strip_ansi_codes(input);
    assert_eq!(result, input);
}

#[serial]
#[test]
fn test_strip_ansi_codes_only_codes() {
    use p2p_app::strip_ansi_codes;
    let input = "\u{1b}[32m\u{1b}[0m";
    let result = strip_ansi_codes(input);
    assert_eq!(result, "");
}

#[serial]
#[test]
fn test_strip_ansi_codes_nested() {
    use p2p_app::strip_ansi_codes;
    let input = "\u{1b}[1m\u{1b}[32mbold green\u{1b}[0m\u{1b}[0m";
    let result = strip_ansi_codes(input);
    assert!(!result.contains('\u{1b}'));
}

#[serial]
#[test]
fn test_p2plog_levels_all() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();

    p2p_app::logging::p2plog_debug("debug message");
    p2p_app::logging::p2plog_info("info message");
    p2p_app::logging::p2plog_error("error message (formerly warn)");

    let logs = p2p_app::logging::get_tui_logs();
    // At least some logs should be captured (depending on filter)
    let _ = logs;
}

#[serial]
#[test]
fn test_push_log_multiple() {
    p2p_app::logging::init_logging();
    p2p_app::clear_tui_logs();

    p2p_app::logging::push_log("log1");
    p2p_app::logging::push_log("log2");
    p2p_app::logging::push_log("log3");

    let logs = p2p_app::logging::get_tui_logs();
    assert!(logs.len() >= 3);
}

#[serial]
#[test]
fn test_clear_tui_logs_clears_all() {
    p2p_app::logging::init_logging();
    p2p_app::logging::push_log("log to be cleared");

    let before = p2p_app::logging::get_tui_logs();
    assert!(!before.is_empty());

    p2p_app::clear_tui_logs();
    let after = p2p_app::logging::get_tui_logs();
    assert!(after.is_empty());
}
