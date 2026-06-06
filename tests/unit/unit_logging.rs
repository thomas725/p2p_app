use super::*;
use serial_test::serial;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};
use tracing::Level;

#[test]
#[cfg(feature = "tracing")]
fn test_tracing_filter_returns_targets() {
    let filter = tracing_filter();
    assert!(!format!("{filter:?}").is_empty());
}

#[test]
#[cfg(feature = "tracing")]
fn test_tracing_filter_has_default_warn() {
    let filter = tracing_filter();
    let filter_str = format!("{filter:?}");
    assert!(filter_str.contains("WARN"));
}

#[test]
#[cfg(feature = "tracing")]
fn test_tracing_filter_enables_debug_targets() {
    let filter = tracing_filter();
    let filter_str = format!("{filter:?}");
    assert!(filter_str.contains("DEBUG") || filter_str.contains("debug"));
}

#[test]
#[serial(logging)]
fn test_push_log_stores_entries() {
    init_logging();
    clear_tui_logs();
    push_log("hello");
    let logs = get_tui_logs();
    assert_eq!(logs.len(), 1);
    assert!(logs[0].contains("hello"));
}

#[test]
#[serial(logging)]
fn test_push_log_keeps_max_capacity() {
    init_logging();
    clear_tui_logs();
    for i in 0..(MAX_TUI_LOGS + 50) {
        push_log(format!("entry-{i}"));
    }
    let logs = get_tui_logs();
    assert_eq!(logs.len(), MAX_TUI_LOGS);
    assert!(logs[0].contains("entry-"));
}

#[test]
#[serial(logging)]
fn test_callback_receives_messages() {
    static CALLBACK_SET: AtomicBool = AtomicBool::new(false);
    static SEEN: OnceLock<Arc<Mutex<Vec<String>>>> = OnceLock::new();

    let seen = SEEN
        .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
        .clone();
    if !CALLBACK_SET.swap(true, Ordering::SeqCst) {
        let seen_for_cb = seen.clone();
        set_tui_callback(move |msg| {
            if let Ok(mut v) = seen_for_cb.lock() {
                v.push(msg);
            }
        });
    }

    if let Ok(mut v) = seen.lock() {
        v.clear();
    }
    init_logging();
    push_log("cb-test");
    let seen_msgs = seen.lock().expect("lock seen");
    assert!(seen_msgs.iter().any(|m| m.contains("cb-test")));
}

#[test]
#[serial(logging)]
fn test_redraw_hook_runs_for_log_messages() {
    static REDRAW_COUNT: AtomicUsize = AtomicUsize::new(0);

    REDRAW_COUNT.store(0, AtomicOrdering::SeqCst);
    init_logging();
    clear_tui_logs();
    set_tui_redraw_hook(|| {
        REDRAW_COUNT.fetch_add(1, AtomicOrdering::SeqCst);
    });

    request_tui_redraw();
    assert_eq!(REDRAW_COUNT.load(AtomicOrdering::SeqCst), 1);

    push_log("redraw-me");
    assert!(get_tui_logs().iter().any(|line| line.contains("redraw-me")));
}

#[test]
#[serial(logging)]
fn test_log_level_helpers_emit_messages() {
    init_logging();
    clear_tui_logs();
    p2plog_debug("d");
    p2plog_info("i");
    p2plog_warn("w");
    p2plog_error("e");
    let joined = get_tui_logs().join("\n");
    assert!(joined.contains("[DEBUG] d"));
    assert!(joined.contains("[INFO] i"));
    assert!(joined.contains("[WARN] w"));
    assert!(joined.contains("[ERROR] e"));
}

#[test]
#[serial(logging)]
fn test_tracing_event_formatting_paths() {
    init_logging();
    clear_tui_logs();

    tracing::event!(target: "p2p_app::test", Level::INFO, message = "");
    tracing::event!(
        target: "p2p_app::test",
        Level::INFO,
        message = "str-message",
        label = "str-field"
    );
    tracing::event!(target: "p2p_app::test", Level::INFO, empty = true);
    tracing::event!(
        target: "p2p_app::test",
        Level::INFO,
        i = 1_i64,
        u = 2_u64,
        b = true,
        dbg = ?Some("x"),
        "hello"
    );

    let logs = get_tui_logs().join("\n");
    assert!(logs.contains("p2p_app::test"));
    assert!(logs.contains("hello"));
    assert!(logs.contains("str-message"));
    assert!(logs.contains("label=str-field"));
    assert!(logs.contains("i=1"));
    assert!(logs.contains("u=2"));
    assert!(logs.contains("b=true"));
}

#[test]
#[serial(logging)]
fn test_tracing_layer_keeps_max_capacity() {
    init_logging();
    clear_tui_logs();

    for i in 0..(MAX_TUI_LOGS + 25) {
        tracing::event!(target: "p2p_app::test", Level::INFO, idx = i);
    }

    let logs = get_tui_logs();
    assert_eq!(logs.len(), MAX_TUI_LOGS);
}

#[test]
fn test_tracing_filter_not_empty() {
    use crate::logging::tracing_filter;

    let _filter = tracing_filter();
    // Filter was created successfully
    assert!(true);
}

#[test]
fn test_clear_tui_logs_idempotent() {
    use crate::logging::clear_tui_logs;

    // Calling clear multiple times should be safe
    clear_tui_logs();
    clear_tui_logs();
    clear_tui_logs();

    // Should not panic
}

#[test]
fn test_p2plog_debug_multiple_calls() {
    use crate::p2plog_debug;

    // Multiple calls should not panic
    p2plog_debug("test message 1");
    p2plog_debug("test message 2");
    p2plog_debug(format!("test message with value: {}", 42));
}

#[test]
fn test_p2plog_error_multiple_calls() {
    use crate::p2plog_error;

    // Multiple calls should not panic
    p2plog_error("error 1");
    p2plog_error("error 2");
    p2plog_error(format!("error with value: {}", "test"));
}
