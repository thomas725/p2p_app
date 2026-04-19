use chrono::Local;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::sync::OnceLock;

/// Initialize the logging system by creating the global logs queue.
///
/// Must be called once at application startup before any logging occurs.
pub fn init_logging() {
    LOGS.get_or_init(|| Mutex::new(VecDeque::new()));
}

/// Remove ANSI escape codes from a string (e.g., color/formatting codes).
///
/// Useful for cleaning terminal output before storing in logs or displaying in TUI.
///
/// # Arguments
/// * `s` - Input string that may contain ANSI escape sequences
///
/// # Returns
/// A new String with all ANSI codes stripped
///
/// # Example
/// ```
/// # use p2p_app::strip_ansi_codes;
/// let colored = "\x1b[32mHello\x1b[0m";
/// assert_eq!(strip_ansi_codes(colored), "Hello");
/// ```
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
///
/// # Arguments
/// * `time` - NaiveDateTime in UTC
///
/// # Returns
/// Formatted string in timezone-aware local time: "YYYY-MM-DD HH:MM:SS +ZZZZ"
pub fn format_peer_datetime(time: chrono::NaiveDateTime) -> String {
    let local = time.and_utc().with_timezone(&chrono::Local);
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

/// Get the current time as a formatted local time string with timezone.
///
/// # Returns
/// Formatted string: "YYYY-MM-DD HH:MM:SS +ZZZZ" in local time
pub fn now_timestamp() -> String {
    let local = chrono::Local::now();
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

static LOG_TUI_CALLBACK: OnceLock<Box<dyn Fn(String) + Send + Sync>> = OnceLock::new();
static LOGS: OnceLock<Mutex<VecDeque<String>>> = OnceLock::new();

pub fn push_log(message: impl Into<String>) {
    let ts = chrono::Local::now().format("%H:%M:%S.%3f");
    let formatted = format!("[{}] {}", ts, message.into());
    let has_callback = LOG_TUI_CALLBACK.get().is_some();
    if let Some(callback) = LOG_TUI_CALLBACK.get() {
        (callback)(formatted.clone());
    }
    if let Some(logs) = LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > 1000 {
            l.pop_front();
        }
    }
    if !has_callback {
        eprintln!("{}", formatted);
    }
}

pub fn set_tui_log_callback<F>(callback: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    let _ = LOG_TUI_CALLBACK.set(Box::new(callback));
}

/// Get the current TUI log messages.
pub fn get_tui_logs() -> VecDeque<String> {
    LOGS.get()
        .map(|m| m.lock().expect("TUI logs mutex not poisoned").clone())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn p2plog(level: &str, msg: String) {
    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
    let formatted = format!("[{}] [{}] {}", ts, level, msg);

    if let Some(callback) = LOG_TUI_CALLBACK.get() {
        (callback)(formatted.clone());
    }

    if let Some(logs) = LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > 1000 {
            l.pop_front();
        }
    }

    // Only print to stderr if TUI is not active (no callback set)
    if LOG_TUI_CALLBACK.get().is_none() {
        eprintln!("{}", formatted);
    }
}

#[allow(dead_code)]
pub fn p2plog_debug(msg: String) {
    p2plog("DEBUG", msg);
}

#[allow(dead_code)]
pub fn p2plog_info(msg: String) {
    p2plog("INFO", msg);
}

#[allow(dead_code)]
pub fn p2plog_warn(msg: String) {
    p2plog("WARN", msg);
}

#[allow(dead_code)]
pub fn p2plog_error(msg: String) {
    p2plog("ERROR", msg);
}
