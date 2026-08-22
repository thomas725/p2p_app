//! Centralized logging system using the `tracing` crate.
//!
//! This module provides a unified logging solution that:
//! - Uses `tracing` (Rust's most popular structured logging library)
//! - Supports a single callback registry for all frontends (TUI, Flutter, Dioxus)
//! - Writes to both terminal (if no callback) and a persistent log file
//! - Integrates with libp2p's existing tracing usage

use std::collections::VecDeque;
use std::io::Write;
use std::sync::{Arc, Mutex, OnceLock};
use tracing::field::Visit;

type LogCallback = Arc<dyn Fn(String) + Send + Sync>;
type RedrawHook = Arc<dyn Fn() + Send + Sync>;

/// Maximum number of logs to keep in memory for UI replay
const MAX_LOGS: usize = 1000;

/// In-memory log storage for UI access and replay when callbacks attach later
static LOG_BUFFER: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

/// Unified callback registry: all frontends (Flutter FRB, TUI, Dioxus)
/// register here to receive log messages. One entry per registration.
static LOG_CALLBACKS: Mutex<Vec<LogCallback>> = Mutex::new(vec![]);

/// Optional hook that requests a TUI redraw when new logs arrive.
static TUI_REDRAW_HOOK: OnceLock<Mutex<Option<RedrawHook>>> = OnceLock::new();

/// Custom tracing layer that sends log events through `push_log`
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

        push_log(msg);
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

    // Initialize log buffer storage
    let _ = LOG_BUFFER.get_or_init(|| Mutex::new(VecDeque::new()));

    // Build filter - use environment or default to warn
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn,p2p_app=info"));

    // Create the subscriber with our custom layer
    let subscriber = tracing_subscriber::registry()
        .with(filter)
        .with(TuiTracingLayer);

    let _ = subscriber.try_init();
}

#[cfg(not(feature = "tracing"))]
pub fn init_logging() {
    let _ = LOG_BUFFER.get_or_init(|| Mutex::new(VecDeque::new()));
}

/// Set or replace the redraw hook used after log messages.
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

/// Get all stored log messages from the buffer.
pub fn get_tui_logs() -> Vec<String> {
    LOG_BUFFER
        .get()
        .map(|m| {
            m.lock()
                .expect("Log buffer not poisoned")
                .clone()
                .into_iter()
                .collect()
        })
        .unwrap_or_default()
}

/// Path to the application log file.
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
static LOG_FILE: Mutex<Option<std::fs::File>> = Mutex::new(None);

/// Write a log line to the persistent log file.
fn write_to_file(formatted: &str) {
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

/// Push a log message to the buffer, file, and all registered callbacks.
///
/// If no callbacks are registered, the message is printed to stderr.
pub fn push_log(message: impl Into<String>) {
    let msg = message.into();
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let formatted = format!("[{ts}] {msg}\n");

    // Store in buffer for replay when callbacks are added later
    {
        let _ = LOG_BUFFER.get_or_init(|| Mutex::new(VecDeque::new()));
        if let Some(logs) = LOG_BUFFER.get()
            && let Ok(mut l) = logs.lock()
        {
            let display = format!("[{ts}] {msg}");
            l.push_back(display.clone());
            if l.len() > MAX_LOGS {
                l.pop_front();
            }
        }
    }

    // Write to persistent log file
    write_to_file(&formatted);

    // Notify all registered callbacks (or stderr if none registered)
    let callbacks = LOG_CALLBACKS.lock().unwrap();
    if callbacks.is_empty() {
        // Use the line without trailing newline for terminal output
        eprintln!("[{ts}] {msg}");
    } else {
        for cb in callbacks.iter() {
            cb(format!("[{ts}] {msg}"));
        }
    }

    // When compiled for the Flutter/mobile frontend, also mirror every log line
    // to stderr so the lines shown in the in-app log tab are also visible in the
    // terminal that launched the app (run_flutter_desktop.sh / run_waydroid.sh).
    // Excluded under `cfg(test)` so the test harness stays quiet.
    #[cfg(all(feature = "mobile", not(test)))]
    {
        eprintln!("[{ts}] {msg}");
    }
}

/// Register a callback that will receive all log messages.
/// External frontends (Flutter FRB, TUI, Dioxus) can use this to add their
/// own log display. When registered, all previously accumulated logs in the
/// buffer are immediately replayed to the callback.
pub fn register_log_callback(callback: LogCallback) {
    let mut callbacks = LOG_CALLBACKS.lock().unwrap();

    // Replay accumulated logs that were captured before this callback was registered
    if let Some(logs) = LOG_BUFFER.get()
        && let Ok(l) = logs.lock()
    {
        for log in l.iter() {
            callback(log.clone());
        }
    }

    callbacks.push(callback);
}

/// Remove ANSI escape codes from a string (e.g., color/formatting codes).
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
