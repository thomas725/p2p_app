//! Formatting and display utilities

use std::time::SystemTime;
use std::sync::atomic::{AtomicU64, Ordering};

/// Format a Chrono NaiveDateTime into "YYYY-MM-DD HH:MM:SS" format
#[must_use]
pub fn format_peer_datetime(time: chrono::NaiveDateTime) -> String {
    time.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Get current timestamp as formatted string "YYYY-MM-DD HH:MM:SS"
#[must_use]
pub fn now_timestamp() -> String {
    format_peer_datetime(chrono::Local::now().naive_local())
}

/// Format SystemTime into "HH:MM:SS.mmm" format (hours:minutes:seconds.milliseconds)
#[must_use]
pub fn format_system_time(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(time)
        .format("%H:%M:%S.%3f")
        .to_string()
}

/// Generate a best-effort unique message ID for receipt tracking.
///
/// This is intentionally dependency-free (no uuid crate).
#[must_use]
pub fn gen_msg_id() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now_ns = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u128;
    let c = COUNTER.fetch_add(1, Ordering::Relaxed) as u128;
    format!("{:x}{:x}", now_ns, c)
}

/// Get the last 8 characters of a peer ID string
#[must_use]
pub fn short_peer_id(id: &str) -> String {
    id.chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// Get display name for a peer - uses local nickname if set, else received nickname, else short ID
#[must_use]
pub fn peer_display_name(
    peer_id: &str,
    local_nicknames: &std::collections::HashMap<String, String>,
    received_nicknames: &std::collections::HashMap<String, String>,
) -> String {
    local_nicknames
        .get(peer_id)
        .or_else(|| received_nicknames.get(peer_id))
        .cloned()
        .unwrap_or_else(|| short_peer_id(peer_id))
}

/// Calculate the offset for auto-scrolling text display
///
/// Returns the line offset that should be visible when auto-scrolling to show the latest content.
#[must_use]
pub fn auto_scroll_offset(total: usize, visible: usize) -> usize {
    total.saturating_sub(visible)
}

/// Generate a title for scrollable content showing current scroll position
///
/// Format: "Prefix (offset/total)"
#[must_use]
pub fn scroll_title(prefix: &str, scroll_offset: usize, total: usize) -> String {
    format!("{} ({}/{})", prefix, scroll_offset.min(total), total)
}

/// Calculate the latency in milliseconds between sent and received times
#[must_use]
pub fn format_latency(sent_at: Option<f64>, received_at: SystemTime) -> String {
    match sent_at {
        Some(sent) => {
            let now = received_at
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();
            let elapsed = now - sent;
            if elapsed < 0.001 {
                "<1ms".to_string()
            } else if elapsed < 1.0 {
                format!("{:.0}ms", elapsed * 1000.0)
            } else {
                format!("{:.1}s", elapsed)
            }
        }
        None => "?".to_string(),
    }
}
