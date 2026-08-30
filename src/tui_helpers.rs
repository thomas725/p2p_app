//! Pure helper functions for TUI modules that can be unit tested.
//! These functions avoid async state, channels, and external I/O.

use chrono::NaiveDateTime;
use std::collections::{HashMap, VecDeque};
use unicode_width::UnicodeWidthStr;

use crate::PeerRecord;

/// Sort peers by last seen time (descending)
pub fn sort_peers_by_last_seen(
    peers: &mut VecDeque<PeerRecord>,
    current_selection: usize,
) -> usize {
    let selected_peer_id = peers.get(current_selection).map(|p| p.peer_id.clone());

    let mut peers_vec: Vec<_> = peers.drain(..).collect();
    peers_vec.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    *peers = peers_vec.into();

    selected_peer_id.map_or_else(
        || current_selection.min(peers.len().saturating_sub(1)),
        |sel_id| {
            peers
                .iter()
                .position(|p| p.peer_id == sel_id)
                .unwrap_or(0)
                .min(peers.len().saturating_sub(1))
        },
    )
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
    let end = start.saturating_add(visible_count).min(total_messages);
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
        format!("{}...", msg.get(..max_len.saturating_sub(3)).unwrap_or(msg))
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
#[allow(clippy::missing_const_for_fn)]
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
#[allow(clippy::missing_const_for_fn)]
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
#[allow(clippy::missing_const_for_fn)]
pub fn scroll_up_lines(scroll_offset: &mut usize, lines: usize) {
    *scroll_offset = scroll_offset.saturating_sub(lines);
}

/// Scroll down to target, enabling auto-scroll if reaching max
#[allow(clippy::missing_const_for_fn)]
pub fn scroll_down_lines(
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    lines: usize,
    max_offset: usize,
) {
    *scroll_offset = scroll_offset.saturating_add(lines).min(max_offset);
    if *scroll_offset >= max_offset {
        *auto_scroll = true;
    }
}

/// Convert crossterm `KeyCode` to scroll action string
#[must_use]
#[allow(clippy::missing_const_for_fn)]
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
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]
pub const fn next_tab_index(current: usize, delta: isize, max_tabs: usize) -> usize {
    if max_tabs == 0 {
        return 0;
    }
    let sum = current as isize + delta;
    ((sum % max_tabs as isize).wrapping_add(max_tabs as isize)) as usize % max_tabs
}

/// Abstraction over the per-peer direct-message map so the peers table can be
/// built from either a `HashMap` (the app state) or a `BTreeMap` (the render state).
pub trait PeerMessageMap {
    /// Number of direct messages stored for `peer_id`.
    fn dm_count_for(&self, peer_id: &str) -> usize;
}

#[allow(clippy::implicit_hasher)]
impl PeerMessageMap for HashMap<String, VecDeque<String>> {
    fn dm_count_for(&self, peer_id: &str) -> usize {
        self.get(peer_id).map_or(0, VecDeque::len)
    }
}

#[allow(clippy::implicit_hasher)]
impl PeerMessageMap for std::collections::BTreeMap<String, VecDeque<String>> {
    fn dm_count_for(&self, peer_id: &str) -> usize {
        self.get(peer_id).map_or(0, VecDeque::len)
    }
}

/// A single row of the peers table, mirroring Flutter's peer list.
pub struct PeerTableRow {
    pub peer_id: String,
    pub display_name: String,
    pub dm_count: usize,
    pub broadcast_count: usize,
    pub last_seen: String,
}

impl PeerTableRow {
    #[must_use]
    pub fn new(
        peer_id: &str,
        display_name: &str,
        dm_count: usize,
        broadcast_count: usize,
        last_seen: &str,
    ) -> Self {
        Self {
            peer_id: peer_id.to_string(),
            display_name: display_name.to_string(),
            dm_count,
            broadcast_count,
            last_seen: last_seen.to_string(),
        }
    }
}

/// Parse a `YYYY-MM-DD HH:MM:SS` (or `...T...`) timestamp into milliseconds since epoch.
#[must_use]
pub fn parse_last_seen_ms(last_seen: &str) -> u64 {
    let norm = last_seen.replace(' ', "T");
    NaiveDateTime::parse_from_str(&norm, "%Y-%m-%dT%H:%M:%S").map_or(0, |dt| {
        let millis = dt.and_utc().timestamp_millis().max(0);
        u64::try_from(millis).unwrap_or(0)
    })
}

/// Count how many broadcast messages each peer has sent, given the list of
/// sender peer ids (`None` entries are ignored).
#[allow(clippy::arithmetic_side_effects)]
#[must_use]
pub fn compute_broadcast_counts(messages: &VecDeque<Option<String>>) -> HashMap<String, usize> {
    let mut map: HashMap<String, usize> = HashMap::new();
    for id in messages.iter().flatten() {
        *map.entry(id.clone()).or_insert(0) += 1;
    }
    map
}

/// Build and sort the peers table according to the active column and order.
///
/// Mirrors Flutter's peer-list sort. Columns: name (0), DM count (1),
/// broadcast count (2), last seen (3). The tie-breaker is always the peer id
/// to keep the ordering stable.
#[must_use]
pub fn sort_peers_table(
    peers: &[PeerRecord],
    dm_messages: &impl PeerMessageMap,
    messages: &VecDeque<Option<String>>,
    sort_column: usize,
    ascending: bool,
) -> Vec<PeerTableRow> {
    let broadcast_map = compute_broadcast_counts(messages);
    let mut rows: Vec<PeerTableRow> = peers
        .iter()
        .map(|p| {
            let dm_count = dm_messages.dm_count_for(&p.peer_id);
            let broadcast_count = broadcast_map.get(&p.peer_id).copied().unwrap_or(0);
            let display_name = crate::get_peer_display_name(&p.peer_id)
                .unwrap_or_else(|_| crate::fmt::short_peer_id(&p.peer_id));
            PeerTableRow::new(
                &p.peer_id,
                &display_name,
                dm_count,
                broadcast_count,
                &p.last_seen,
            )
        })
        .collect();

    rows.sort_by(|a, b| {
        let ord = match sort_column {
            0 => {
                let an = a.display_name.to_lowercase();
                let bn = b.display_name.to_lowercase();
                if an == bn {
                    a.peer_id.cmp(&b.peer_id)
                } else {
                    an.cmp(&bn)
                }
            }
            1 => a.dm_count.cmp(&b.dm_count).then_with(|| a.peer_id.cmp(&b.peer_id)),
            2 => a
                .broadcast_count
                .cmp(&b.broadcast_count)
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
            _ => parse_last_seen_ms(&a.last_seen)
                .cmp(&parse_last_seen_ms(&b.last_seen))
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
        };
        if ascending {
            ord
        } else {
            ord.reverse()
        }
    });
    rows
}

/// Reorder `peers` in place according to the active column/order, preserving
/// the currently selected peer (by id) and returning its new index.
#[must_use]
pub fn sort_peers_by_column(
    peers: &mut VecDeque<PeerRecord>,
    dm_messages: &impl PeerMessageMap,
    messages: &VecDeque<Option<String>>,
    sort_column: usize,
    ascending: bool,
    selection: usize,
) -> usize {
    let selected_id = peers.get(selection).map(|p| p.peer_id.clone());
    let rows = sort_peers_table(peers.as_slices().0, dm_messages, messages, sort_column, ascending);
    let ordered: VecDeque<PeerRecord> = rows
        .iter()
        .filter_map(|r| peers.iter().find(|p| p.peer_id == r.peer_id).cloned())
        .collect();
    *peers = ordered;
    selected_id
        .and_then(|id| peers.iter().position(|p| p.peer_id == id))
        .unwrap_or(0)
}

/// Build table rows in the order the peers are currently stored (no re-sort).
#[must_use]
pub fn peer_table_rows_ordered(
    peers: &[PeerRecord],
    dm_messages: &impl PeerMessageMap,
    messages: &VecDeque<Option<String>>,
) -> Vec<PeerTableRow> {
    let broadcast_map = compute_broadcast_counts(messages);
    peers
        .iter()
        .map(|p| {
            let dm_count = dm_messages.dm_count_for(&p.peer_id);
            let broadcast_count = broadcast_map.get(&p.peer_id).copied().unwrap_or(0);
            let display_name = crate::get_peer_display_name(&p.peer_id)
                .unwrap_or_else(|_| crate::fmt::short_peer_id(&p.peer_id));
            PeerTableRow::new(
                &p.peer_id,
                &display_name,
                dm_count,
                broadcast_count,
                &p.last_seen,
            )
        })
        .collect()
}

/// Sort-direction indicator shown on the active peers-table column header.
#[must_use]
#[allow(clippy::missing_const_for_fn)]
pub const fn peer_sort_indicator(ascending: bool) -> &'static str {
    if ascending { " ▲" } else { " ▼" }
}

/// Header labels for the peers table; the sort indicator marks the active column.
#[must_use]
pub fn peer_table_header_labels(sort_column: usize, sort_ascending: bool) -> Vec<String> {
    const COLUMNS: [&str; 4] = ["Name", "DM", "Broadcast", "Last Seen"];
    let indicator = peer_sort_indicator(sort_ascending);
    COLUMNS
        .iter()
        .enumerate()
        .map(|(i, name)| {
            if i == sort_column {
                format!("{name}{indicator}")
            } else {
                (*name).to_string()
            }
        })
        .collect()
}

/// Display width of each cell of a peers-table row (CJK/wide chars = 2 cols).
#[must_use]
fn peer_cell_widths(row: &PeerTableRow) -> [usize; 4] {
    let dm = row.dm_count.to_string();
    let broadcast = row.broadcast_count.to_string();
    [
        row.display_name.width(),
        dm.width(),
        broadcast.width(),
        row.last_seen.width(),
    ]
}

/// Render widths for the peers-table columns.
///
/// Each column is exactly wide enough for its longest header (including the
/// sort indicator) or cell, so the table only spans its content instead of
/// filling the whole area.
#[must_use]
pub fn peer_table_column_widths(
    rows: &[PeerTableRow],
    sort_column: usize,
    sort_ascending: bool,
) -> Vec<usize> {
    let mut widths: Vec<usize> = peer_table_header_labels(sort_column, sort_ascending)
        .iter()
        .map(|h| h.width())
        .collect();
    for row in rows {
        for (slot, cell) in widths.iter_mut().zip(peer_cell_widths(row)) {
            *slot = (*slot).max(cell);
        }
    }
    widths
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_helpers.rs"]
mod tests;
