//! Centralized logging system using the `tracing` crate.
//!
//! This module provides a unified logging solution that:
//! - Uses `tracing` (Rust's most popular structured logging library)
//! - Supports multiple subscribers (file, stdout, TUI)
//! - Provides a TUI callback for displaying logs in the UI
//! - Integrates with libp2p's existing tracing usage
//! - Writes to a persistent log file in `target/` or CWD
//! - Allows frontends (Flutter, TUI, Dioxus) to register log callbacks

use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::Visit;

/// Maximum number of logs to keep in memory for TUI
const MAX_TUI_LOGS: usize = 1000;

/// Global TUI callback for forwarding logs to UI
static TUI_CALLBACK: OnceLock<Arc<dyn Fn(String) + Send + Sync>> = OnceLock::new();

/// Optional hook that requests a TUI redraw when new logs arrive.
#[allow(clippy::type_complexity)]
static TUI_REDRAW_HOOK: OnceLock<Mutex<Option<Arc<dyn Fn() + Send + Sync>>>> = OnceLock::new();

/// In-memory log storage for TUI access
static TUI_LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

/// Custom tracing layer that writes to TUI logs instead of stdout/stderr
struct TuiTracingLayer;

impl<S> tracing_subscriber::layer::Layer<S> for TuiTracingLayer
where
    S: tracing::Subscriber + 'static,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: tracing_subscriber::layer::Context<S>) {
        let mut buf = String::new();
        let mut visitor = FormatVisitor(&mut buf);
        event.record(&mut visitor);

        let ts = chrono::Local::now().format("%H:%M:%S.%3f");
        let level = event.metadata().level().to_string();
        let target = event.metadata().target();
        let msg = if buf.is_empty() {
            format!("{ts} [{level}] {target}")
        } else {
            format!("{ts} [{level}] {target} {buf}")
        };

        if let Some(logs) = TUI_LOGS.get()
            && let Ok(mut l) = logs.lock()
        {
            l.push_back(msg.clone());
            if l.len() > MAX_TUI_LOGS {
                l.pop_front();
            }
        }

        if let Some(callback) = TUI_CALLBACK.get() {
            callback(msg);
        }
    }
}

/// Visitor that formats event fields into a string
struct FormatVisitor<'a>(&'a mut String);

impl Visit for FormatVisitor<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            self.0.push_str(&format!(" {}={value}", field.name()));
        }
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0.push_str(&format!(" {}={value}", field.name()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0.push_str(&format!(" {}={value}", field.name()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0.push_str(&format!(" {}={value}", field.name()));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!(" {}={value:?}", field.name()));
    }
}

/// Initialize the logging system.
///
/// Must be called once at application startup before any logging occurs.
/// When the `tracing` feature is enabled, this sets up the tracing subscriber.
#[cfg(feature = "tracing")]
pub fn init_logging() {
    use tracing_subscriber::prelude::*;

    // Initialize TUI logs storage
    let _ = TUI_LOGS.get_or_init(|| Mutex::new(VecDeque::new()));

    // Build filter - use environment or default to warn
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,p2p_app=info"));

    // Create the subscriber with our custom TUI layer (no stdout/stderr output)
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(TuiTracingLayer);

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

/// Set or replace the redraw hook used by the TUI log callback.
pub fn set_tui_redraw_hook<F>(hook: F)
where
    F: Fn() + Send + Sync + 'static,
{
    let hook_cell = TUI_REDRAW_HOOK.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = hook_cell.lock() {
        *guard = Some(Arc::new(hook));
    }
}

/// Request a TUI redraw if a redraw hook has been installed.
pub fn request_tui_redraw() {
    let Some(hook_cell) = OnceLock::get(&TUI_REDRAW_HOOK) else {
        return;
    };
    if let Ok(guard) = hook_cell.lock()
        && let Some(hook) = guard.as_ref()
    {
        hook();
    }
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

/// Push a log message to TUI storage and callback.
pub fn push_log(message: impl Into<String>) {
    let msg = message.into();
    let ts = chrono::Local::now().format("%H:%M:%S.%3f");
    let formatted = format!("[{ts}] {msg}");

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
        eprintln!("{formatted}");
    }
}

// ============================================================
// === Persistent log file + frontend callback registry ===
// ============================================================

/// Path to the application log file.
///
/// Checks for `target/` folder first (if it exists as a subdirectory of CWD),
/// otherwise falls back to the current working directory.
fn log_file_path() -> std::path::PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let target_dir = cwd.join("target");
    if target_dir.is_dir() {
        target_dir.join(format!("p2p_app_{}.log", chrono::Local::now().format("%F_%H%M-%S")))
    } else {
        cwd.join(format!("p2p_app_{}.log", chrono::Local::now().format("%F_%H%M-%S")))
    }
}

/// Global log file handle (append mode, thread-safe via Mutex).
///
/// Writes to `target/p2p_app_YYYY-MM-DD_HHMM-SS.log` if the `target/` folder
/// exists as a subdirectory of the current working directory.
/// Otherwise falls back to `p2p_app_YYYY-MM-DD_HHMM-SS.log` in the current directory.
static LOG_FILE: std::sync::Mutex<Option<std::fs::File>> = std::sync::Mutex::new(None);

/// Global callback registry: external UIs (Flutter, TUI, Dioxus) can register
/// a closure that receives log messages and appends them to the same log file.
static LOG_CALLBACKS: std::sync::Mutex<Vec<Arc<dyn Fn(String) + Send + Sync>>> = std::sync::Mutex::new(vec![]);

/// Register a callback that will receive all log messages.
/// External frontends (Flutter FRB, TUI, Dioxus) can use this to add their
/// own log entries to the Rust-managed log file.
pub fn register_log_callback(callback: Arc<dyn Fn(String) + Send + Sync>) {
    let mut callbacks = LOG_CALLBACKS.lock().unwrap();
    callbacks.push(callback);
}

/// Push a log message to the file and all registered callbacks.
pub fn push_log_to_file(message: impl Into<String>) {
    let msg = message.into();
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let formatted = format!("[{ts}] {msg}\n");

    // Write to file (create if needed, append otherwise)
    {
        let mut file = LOG_FILE.lock().unwrap();
        if file.is_none() {
            let path = log_file_path();
            let _ = std::fs::create_dir_all(path.parent().unwrap_or(&std::path::PathBuf::new()));
            match std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
            {
                Ok(f) => *file = Some(f),
                Err(_) => *file = None,
            }
        }
        if let Some(f) = file.as_mut() {
            let _ = f.write_all(formatted.as_bytes());
        }
    }

    // Notify all registered callbacks
    let callbacks = LOG_CALLBACKS.lock().unwrap();
    for cb in callbacks.iter() {
        let _ = cb(formatted.clone());
    }
}

/// Remove ANSI escape codes from a string (e.g., color/formatting codes).
///
/// Useful for cleaning terminal output before storing in logs or displaying in TUI.
#[must_use]
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

/// Log function implementation
#[allow(dead_code)]
fn p2plog(level: &str, msg: String) {
    push_log(format!("[{level}] {msg}"));
}

/// Debug log alias
pub fn p2plog_debug(msg: impl Into<String>) {
    push_log(format!("[DEBUG] {}", msg.into()));
}

/// Info log alias
pub fn p2plog_info(msg: impl Into<String>) {
    push_log(format!("[INFO] {}", msg.into()));
}

/// Error log alias
pub fn p2plog_error(msg: impl Into<String>) {
    push_log(format!("[ERROR] {}", msg.into()));
}

#[cfg(any(test, feature = "test-utils"))]
#[path = "../tests/shared/logging_test_utils.rs"]
mod test_utils;

#[cfg(any(test, feature = "test-utils"))]
pub use test_utils::{clear_tui_logs, tracing_filter};

#[cfg(test)]
#[path = "../tests/unit/unit_logging.rs"]
mod tests;
