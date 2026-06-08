//! Pure helper functions for TUI modules that can be unit tested.
//! These functions avoid async state, channels, and external I/O.

use crate::PeerRecord;
use std::collections::VecDeque;

/// Sort peers by last seen time (descending)
pub fn sort_peers_by_last_seen(
    peers: &mut VecDeque<PeerRecord>,
    current_selection: usize,
) -> usize {
    let selected_peer_id = peers.get(current_selection).map(|p| p.peer_id.clone());

    let mut peers_vec: Vec<_> = peers.drain(..).collect();
    peers_vec.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    *peers = peers_vec.into();

    match selected_peer_id {
        Some(sel_id) => peers
            .iter()
            .position(|p| p.peer_id == sel_id)
            .unwrap_or(0)
            .min(peers.len().saturating_sub(1)),
        None => current_selection.min(peers.len().saturating_sub(1)),
    }
}

/// Insert or update peer last seen time
pub fn upsert_peer_last_seen(
    peers: &mut VecDeque<PeerRecord>,
    current_selection: usize,
    peer_id: &str,
    seen_at: &str,
) -> usize {
    if let Some(p) = peers.iter_mut().find(|p| p.peer_id == peer_id) {
        p.last_seen = seen_at.to_string();
    } else {
        peers.push_back(PeerRecord {
            peer_id: peer_id.to_string(),
            first_seen: seen_at.to_string(),
            last_seen: seen_at.to_string(),
        });
    }
    sort_peers_by_last_seen(peers, current_selection)
}

/// Check if message content indicates a nickname-only update
#[must_use]
pub fn is_nickname_update(content: &str, nickname: Option<&str>) -> bool {
    content.trim().is_empty() && nickname.is_some()
}

/// Calculate first visible message index accounting for scroll
#[must_use]
pub fn calculate_visible_range(
    total_messages: usize,
    scroll_offset: usize,
    visible_count: usize,
) -> (usize, usize) {
    let start = scroll_offset.min(total_messages.saturating_sub(1));
    let end = (start + visible_count).min(total_messages);
    (start, end)
}

/// Validate nickname (alphanumeric and dash only, max 20 chars)
#[must_use]
pub fn validate_nickname(nick: &str) -> bool {
    !nick.is_empty() && nick.len() <= 20 && nick.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Truncate message for display
#[must_use]
pub fn truncate_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...", &msg[..max_len.saturating_sub(3)])
    }
}

/// Parse latency string to milliseconds
#[must_use]
pub fn parse_latency(latency: &str) -> Option<f64> {
    if latency == "<1ms" {
        Some(0.5)
    } else if let Some(ms) = latency.strip_suffix("ms") {
        ms.parse().ok()
    } else if let Some(s) = latency.strip_suffix('s') {
        s.parse::<f64>().ok().map(|s| s * 1000.0)
    } else {
        None
    }
}

/// Check if scroll position indicates at bottom
#[must_use]
pub fn is_at_bottom(scroll_offset: usize, total: usize, visible: usize) -> bool {
    scroll_offset >= total.saturating_sub(visible)
}

// ============================================
// Scroll handler pure functions
// ============================================

/// Default page size for scrolling
pub const PAGE_SIZE: usize = 8;

/// Default wheel scroll lines
pub const WHEEL_SCROLL_LINES: usize = 3;

/// Disables auto-scroll and sets offset to max if auto-scroll was enabled
pub fn disable_auto_scroll_to_max(
    auto_scroll: &mut bool,
    scroll_offset: &mut usize,
    max_offset: usize,
) {
    if *auto_scroll {
        *auto_scroll = false;
        *scroll_offset = max_offset;
    }
}

/// Scroll up by one line or page
pub fn scroll_up_lines(scroll_offset: &mut usize, lines: usize) {
    *scroll_offset = scroll_offset.saturating_sub(lines);
}

/// Scroll down to target, enabling auto-scroll if reaching max
pub fn scroll_down_lines(
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    lines: usize,
    max_offset: usize,
) {
    *scroll_offset = (*scroll_offset + lines).min(max_offset);
    if *scroll_offset >= max_offset {
        *auto_scroll = true;
    }
}

/// Convert crossterm `KeyCode` to scroll action string
#[must_use]
pub fn key_code_to_scroll_action(key_code: crossterm::event::KeyCode) -> Option<&'static str> {
    match key_code {
        crossterm::event::KeyCode::Up => Some("Up"),
        crossterm::event::KeyCode::Down => Some("Down"),
        crossterm::event::KeyCode::PageUp => Some("PageUp"),
        crossterm::event::KeyCode::PageDown => Some("PageDown"),
        crossterm::event::KeyCode::Home => Some("Home"),
        crossterm::event::KeyCode::End => Some("End"),
        _ => None,
    }
}

/// Handle scroll key for a section - returns new (`scroll_offset`, `auto_scroll`)
#[must_use]
pub fn handle_scroll_key_for_section(
    key_code: &str,
    scroll_offset: usize,
    auto_scroll: bool,
    max_offset: usize,
) -> (usize, bool) {
    let mut new_offset = scroll_offset;
    let mut new_auto = auto_scroll;

    match key_code {
        "Up" => {
            disable_auto_scroll_to_max(&mut new_auto, &mut new_offset, max_offset);
            scroll_up_lines(&mut new_offset, 1);
        }
        "Down" => {
            disable_auto_scroll_to_max(&mut new_auto, &mut new_offset, max_offset);
            scroll_down_lines(&mut new_offset, &mut new_auto, 1, max_offset);
        }
        "PageUp" => {
            disable_auto_scroll_to_max(&mut new_auto, &mut new_offset, max_offset);
            scroll_up_lines(&mut new_offset, PAGE_SIZE);
        }
        "PageDown" => {
            disable_auto_scroll_to_max(&mut new_auto, &mut new_offset, max_offset);
            scroll_down_lines(&mut new_offset, &mut new_auto, PAGE_SIZE, max_offset);
        }
        "Home" => {
            new_auto = false;
            new_offset = 0;
        }
        "End" => {
            new_auto = true;
            new_offset = max_offset;
        }
        _ => {}
    }
    (new_offset, new_auto)
}

/// Rename sender labels in a DM transcript when a nickname changes.
///
/// Replaces all occurrences of `[old_nick] ` with `[new_nick] ` so the
/// conversation history stays consistent after a nickname update.
pub fn relabel_dm_transcript(
    messages: &mut std::collections::VecDeque<String>,
    old_nick: &str,
    new_nick: &str,
) {
    let from = format!("[{old_nick}] ");
    let to = format!("[{new_nick}] ");
    for line in messages.iter_mut() {
        if line.contains(&from) {
            *line = line.replace(&from, &to);
        }
    }
}

/// Calculate tab index from current + delta (wrapping)
#[must_use]
pub fn next_tab_index(current: usize, delta: isize, max_tabs: usize) -> usize {
    if max_tabs == 0 {
        return 0;
    }
    let sum = current as isize + delta;
    ((sum % max_tabs as isize) + max_tabs as isize) as usize % max_tabs
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_helpers.rs"]
mod tests;
