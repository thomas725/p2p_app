//! Centralized logging system using the `tracing` crate.
//!
//! This module provides a unified logging solution that:
//! - Uses `tracing` (Rust's most popular structured logging library)
//! - Supports multiple subscribers (file, stdout, TUI)
//! - Provides a TUI callback for displaying logs in the UI
//! - Integrates with libp2p's existing tracing usage

use std::collections::VecDeque;
use std::sync::OnceLock;
use std::sync::{Arc, Mutex};

/// Maximum number of logs to keep in memory for TUI
const MAX_TUI_LOGS: usize = 1000;

/// Global TUI callback for forwarding logs to UI
static TUI_CALLBACK: OnceLock<Arc<dyn Fn(String) + Send + Sync>> = OnceLock::new();

/// In-memory log storage for TUI access
static TUI_LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

/// Initialize the logging system.
///
/// Must be called once at application startup before any logging occurs.
/// When the `tracing` feature is enabled, this sets up the tracing subscriber.
#[cfg(feature = "tracing")]
pub fn init_logging() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    // Initialize TUI logs storage
    let _ = TUI_LOGS.get_or_init(|| Mutex::new(VecDeque::new()));

    // Build filter - use environment or default to warn
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,p2p_app=info"));

    // Create the subscriber
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true).with_thread_ids(true));

    // Try to init (may fail if already initialized)
    let _ = subscriber.try_init();
}

#[cfg(not(feature = "tracing"))]
pub fn init_logging() {
    let _ = TUI_LOGS.get_or_init(|| Mutex::new(VecDeque::new()));
}

/// Set a callback to receive log messages for TUI display.
pub fn set_tui_callback<F>(callback: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    let _ = TUI_CALLBACK.set(Arc::new(callback));
}

/// Get all stored TUI log messages.
pub fn get_tui_logs() -> Vec<String> {
    TUI_LOGS
        .get()
        .map(|m| {
            m.lock()
                .expect("TUI logs not poisoned")
                .clone()
                .into_iter()
                .collect()
        })
        .unwrap_or_default()
}

/// Clear TUI log storage.
pub fn clear_tui_logs() {
    if let Some(logs) = TUI_LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.clear();
    }
}

/// Push a log message to TUI storage and callback.
pub fn push_log(message: impl Into<String>) {
    let msg = message.into();
    let ts = chrono::Local::now().format("%H:%M:%S.%3f");
    let formatted = format!("[{}] {}", ts, msg);

    if let Some(logs) = TUI_LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > MAX_TUI_LOGS {
            l.pop_front();
        }
    }

    if let Some(callback) = TUI_CALLBACK.get() {
        callback(formatted);
    } else {
        eprintln!("{}", formatted);
    }
}

/// Legacy log function (maintained for backward compatibility)
#[allow(dead_code)]
pub fn p2plog(level: &str, msg: String) {
    let ts = chrono::Local::now().format("%H:%M:%S.%3f").to_string();
    let formatted = format!("[{}] [{}] {}", ts, level, msg);

    if let Some(logs) = TUI_LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > MAX_TUI_LOGS {
            l.pop_front();
        }
    }

    if let Some(callback) = TUI_CALLBACK.get() {
        callback(formatted);
    } else {
        eprintln!("{}", formatted);
    }
}

/// Legacy debug log alias
#[allow(dead_code)]
pub fn p2plog_debug(msg: impl Into<String>) {
    p2plog("DEBUG", msg.into());
}

/// Legacy info log alias
#[allow(dead_code)]
pub fn p2plog_info(msg: impl Into<String>) {
    p2plog("INFO", msg.into());
}

/// Legacy warn log alias
#[allow(dead_code)]
pub fn p2plog_warn(msg: impl Into<String>) {
    p2plog("WARN", msg.into());
}

/// Legacy error log alias
#[allow(dead_code)]
pub fn p2plog_error(msg: impl Into<String>) {
    p2plog("ERROR", msg.into());
}

/// Remove ANSI escape codes from a string (e.g., color/formatting codes).
///
/// Useful for cleaning terminal output before storing in logs or displaying in TUI.
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Format a naive UTC datetime as a local time string with timezone.
pub fn format_peer_datetime(time: chrono::NaiveDateTime) -> String {
    let local = time.and_utc().with_timezone(&chrono::Local);
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

/// Get the current time as a formatted local time string with timezone.
pub fn now_timestamp() -> String {
    let local = chrono::Local::now();
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}
