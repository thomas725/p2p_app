//! Pure helper functions for TUI modules that can be unit tested.
//! These functions avoid async state, channels, and external I/O.

use std::collections::VecDeque;

/// Sort peers by last seen time (descending)
pub fn sort_peers_by_last_seen(
    peers: &mut VecDeque<(String, String, String)>,
    current_selection: usize,
) -> usize {
    let selected_peer_id = peers.get(current_selection).map(|(id, _, _)| id.clone());

    let mut peers_vec: Vec<_> = peers.drain(..).collect();
    peers_vec.sort_by(|a, b| b.2.cmp(&a.2));
    *peers = peers_vec.into();

    match selected_peer_id {
        Some(sel_id) => peers
            .iter()
            .position(|(id, _, _)| id == &sel_id)
            .unwrap_or(0)
            .min(peers.len().saturating_sub(1)),
        None => current_selection.min(peers.len().saturating_sub(1)),
    }
}

/// Insert or update peer last seen time
pub fn upsert_peer_last_seen(
    peers: &mut VecDeque<(String, String, String)>,
    current_selection: usize,
    peer_id: &str,
    seen_at: &str,
) -> usize {
    if let Some((_, _, last_seen)) = peers.iter_mut().find(|(id, _, _)| id == peer_id) {
        *last_seen = seen_at.to_string();
    } else {
        peers.push_back((
            peer_id.to_string(),
            seen_at.to_string(),
            seen_at.to_string(),
        ));
    }
    sort_peers_by_last_seen(peers, current_selection)
}

/// Check if message content indicates a nickname-only update
pub fn is_nickname_update(content: &str, nickname: Option<&str>) -> bool {
    content.trim().is_empty() && nickname.is_some()
}

/// Calculate scroll offset for auto-scrolling
pub fn calculate_auto_scroll(total_messages: usize, visible_count: usize) -> usize {
    total_messages.saturating_sub(visible_count)
}

/// Calculate first visible message index accounting for scroll
pub fn calculate_visible_range(
    total_messages: usize,
    scroll_offset: usize,
    visible_count: usize,
) -> (usize, usize) {
    let start = scroll_offset.min(total_messages.saturating_sub(1));
    let end = (start + visible_count).min(total_messages);
    (start, end)
}

/// Parse command string into command parts
pub fn parse_command(input: &str) -> Option<(&str, &str)> {
    let input = input.trim();
    if input.starts_with('/') {
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        Some((parts[0], parts.get(1).copied().unwrap_or("")))
    } else {
        None
    }
}

/// Check if input is a command
pub fn is_command(input: &str) -> bool {
    input.trim().starts_with('/')
}

/// Get command name from input
pub fn get_command_name(input: &str) -> Option<&str> {
    parse_command(input).map(|(cmd, _)| cmd)
}

/// Get command argument from input
pub fn get_command_arg(input: &str) -> Option<&str> {
    parse_command(input).and_then(|(_, arg)| if arg.is_empty() { None } else { Some(arg) })
}

/// Validate nickname (alphanumeric and dash only, max 20 chars)
pub fn validate_nickname(nick: &str) -> bool {
    !nick.is_empty() && nick.len() <= 20 && nick.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Truncate message for display
pub fn truncate_message(msg: &str, max_len: usize) -> String {
    if msg.len() <= max_len {
        msg.to_string()
    } else {
        format!("{}...", &msg[..max_len.saturating_sub(3)])
    }
}

/// Calculate how many lines a message will occupy given terminal width
pub fn message_line_count(message: &str, terminal_width: usize) -> usize {
    if terminal_width == 0 {
        return 1;
    }
    let mut lines = 0;
    for line in message.lines() {
        let line_len = line.len();
        if line_len == 0 {
            lines += 1;
        } else {
            lines += line_len.div_ceil(terminal_width);
        }
    }
    lines.max(1)
}

/// Get short peer ID (last 8 chars)
pub fn short_peer_id(id: &str) -> String {
    id.chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

/// Parse latency string to milliseconds
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
pub fn is_at_bottom(scroll_offset: usize, total: usize, visible: usize) -> bool {
    scroll_offset >= total.saturating_sub(visible)
}

/// Format peer list item
pub fn format_peer_list_item(
    peer_id: &str,
    local_nickname: Option<&str>,
    last_seen: &str,
) -> String {
    match local_nickname {
        Some(nick) => format!(
            "{} ({}) - {}",
            nick,
            &peer_id[..8.min(peer_id.len())],
            last_seen
        ),
        None => format!("{} - {}", &peer_id[..8.min(peer_id.len())], last_seen),
    }
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

/// Maximum scroll offset calculation
pub fn calc_max_scroll(total_items: usize, visible_count: usize) -> usize {
    total_items.saturating_sub(visible_count)
}

/// Handle scroll key for a section - returns new (scroll_offset, auto_scroll)
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

/// Handle mouse wheel scroll
pub fn handle_mouse_wheel_scroll(
    direction: &str,
    scroll_offset: usize,
    max_offset: usize,
) -> usize {
    match direction {
        "up" => scroll_offset.saturating_sub(WHEEL_SCROLL_LINES),
        "down" => (scroll_offset + WHEEL_SCROLL_LINES).min(max_offset),
        _ => scroll_offset,
    }
}

/// Calculate tab index from current + delta (wrapping)
pub fn next_tab_index(current: usize, delta: isize, max_tabs: usize) -> usize {
    if max_tabs == 0 {
        return 0;
    }
    let sum = current as isize + delta;
    ((sum % max_tabs as isize) + max_tabs as isize) as usize % max_tabs
}
