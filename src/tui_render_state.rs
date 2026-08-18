//! TUI rendering state abstraction for testing
//!
//! This module provides a render state that can be used by both the binary
//! and integration tests. The binary uses `AppState`, tests use this abstraction.

use crate::PeerRecord;
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Minimum state needed to render a TUI frame
#[derive(Clone, Debug)]
pub struct TuiRenderState {
    /// Names of tabs for display (Chat, Peers, Log, plus DM tabs)
    pub tab_titles: Vec<String>,
    /// Currently active tab index
    pub active_tab: usize,
    /// Broadcast messages in chat tab
    pub messages: VecDeque<String>,
    /// Peer ID of each message sender (None = sent by local user)
    pub message_peer_ids: VecDeque<Option<String>>,
    /// Message IDs for sent messages (None if from other peers)
    pub message_ids: VecDeque<Option<String>>,
    /// Receipt status for broadcast messages: `peer_id` -> (`msg_id` -> timestamp)
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    /// Known peers
    pub peers: Vec<PeerRecord>,
    /// Direct messages per peer
    pub dm_messages: BTreeMap<String, VecDeque<String>>,
    /// Message IDs for DMs per peer
    pub dm_message_ids: BTreeMap<String, VecDeque<Option<String>>>,
    /// DM receipt status per peer: (`source_peer`, timestamp)
    pub dm_receipts: HashMap<String, (String, f64)>,
    /// Current input text in message box
    pub input_text: String,
    /// Logs shown in the debug tab
    pub log_messages: VecDeque<String>,
    /// Whether we're editing a peer's nickname
    pub editing_nickname: bool,
    /// Peer ID being edited (if `editing_nickname` is true)
    pub nickname_peer_id: String,
    /// Whether connected to the network
    pub connected: bool,
    /// Number of connected peers
    pub peer_count: usize,
    /// Whether mouse is captured (for text input)
    pub mouse_capture: bool,
    /// Optional popup message to display
    pub popup: Option<String>,
    /// Scroll offset for chat tab
    pub chat_scroll_offset: usize,
    /// Whether chat tab is auto-scrolling to bottom
    pub chat_auto_scroll: bool,
    /// Scroll offset for the log tab
    pub log_scroll_offset: usize,
    /// Whether log tab is auto-scrolling to bottom
    pub log_auto_scroll: bool,
    /// Scroll state per DM peer: (offset, `auto_scroll`)
    pub dm_scroll_state: BTreeMap<String, (usize, bool)>,
    /// Scroll state for DM+broadcast view per peer
    pub dm_broadcast_scroll_state: BTreeMap<String, (usize, bool)>,
    /// Index of selected broadcast message (for receipt popup)
    pub broadcast_selection: Option<usize>,
    /// Index of selected peer in peer list
    pub peer_selection: usize,
}

impl Default for TuiRenderState {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiRenderState {
    /// Creates a new empty [`TuiRenderState`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            tab_titles: vec!["Chat".into(), "Peers".into(), "Log".into()],
            active_tab: 0,
            messages: VecDeque::new(),
            message_peer_ids: VecDeque::new(),
            message_ids: VecDeque::new(),
            broadcast_receipts: HashMap::new(),
            peers: Vec::new(),
            dm_messages: BTreeMap::new(),
            dm_message_ids: BTreeMap::new(),
            dm_receipts: HashMap::new(),
            input_text: String::new(),
            log_messages: VecDeque::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 0,
            mouse_capture: false,
            popup: None,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            log_scroll_offset: 0,
            log_auto_scroll: true,
            dm_scroll_state: BTreeMap::new(),
            dm_broadcast_scroll_state: BTreeMap::new(),
            broadcast_selection: None,
            peer_selection: 0,
        }
    }

    /// Create with sample data for testing
    #[must_use]
    pub fn with_sample_data() -> Self {
        let mut messages = VecDeque::new();
        messages.push_back("[You] Hello world".into());
        messages.push_back("[Peer1] How are you?".into());
        messages.push_back("[You] I'm good!".into());

        let peers = vec![
            PeerRecord {
                peer_id: "12D3KooWH123456".into(),
                first_seen: "Alice".into(),
                last_seen: "Online".into(),
            },
            PeerRecord {
                peer_id: "12D3KooWH789012".into(),
                first_seen: "Bob".into(),
                last_seen: "Online".into(),
            },
        ];

        let mut dm_messages = BTreeMap::new();
        dm_messages.insert("Alice".into(), VecDeque::new());
        let mut dm_message_ids = BTreeMap::new();
        dm_message_ids.insert("Alice".into(), VecDeque::new());

        Self {
            tab_titles: vec![
                "Chat".into(),
                "Peers".into(),
                "Log".into(),
                "DM: Alice".into(),
            ],
            active_tab: 0,
            messages,
            message_peer_ids: VecDeque::new(),
            message_ids: VecDeque::new(),
            broadcast_receipts: HashMap::new(),
            peers,
            dm_messages,
            dm_message_ids,
            dm_receipts: HashMap::new(),
            input_text: String::new(),
            log_messages: VecDeque::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 2,
            mouse_capture: false,
            popup: None,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            log_scroll_offset: 0,
            log_auto_scroll: true,
            dm_scroll_state: BTreeMap::new(),
            dm_broadcast_scroll_state: BTreeMap::new(),
            broadcast_selection: None,
            peer_selection: 0,
        }
    }

    /// Add a message to chat (peer_id of None means local user)
    pub fn add_message(&mut self, msg: impl Into<String>) {
        self.messages.push_back(msg.into());
        self.message_peer_ids.push_back(None);
    }

    /// Add a peer
    pub fn add_peer(
        &mut self,
        peer_id: impl Into<String>,
        first_seen: impl Into<String>,
        last_seen: impl Into<String>,
    ) {
        self.peers.push(PeerRecord {
            peer_id: peer_id.into(),
            first_seen: first_seen.into(),
            last_seen: last_seen.into(),
        });
    }

    /// Add a DM message for a peer
    pub fn add_dm_message(&mut self, peer: impl Into<String>, msg: impl Into<String>) {
        let peer = peer.into();
        let entry = self.dm_messages.entry(peer).or_default();
        entry.push_back(msg.into());
    }
}

/// Calculate visible items and effective offset for string-based messages with auto/manual scroll
#[must_use]
pub fn calc_visible_strings(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().cloned().collect();
    calc_visible_impl(&msgs, auto_scroll, scroll_offset, usable_height, |m| {
        count_lines(m, text_width)
    })
}

/// Count wrapped lines of text, accounting for ANSI codes and terminal width
#[must_use]
pub fn count_lines(text: &str, text_width: usize) -> usize {
    if text_width == 0 || text.is_empty() {
        return 1;
    }
    let clean_text = crate::logging::strip_ansi_codes(text);
    let lines: Vec<&str> = clean_text.split('\n').collect();

    let mut total = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            if i < lines.len() - 1 {
                total += 1;
            }
        } else {
            total += line.len().div_ceil(text_width);
        }
    }
    total.max(1)
}

const MIN_VISIBLE: usize = 1;

fn calc_visible_impl<F>(
    messages: &[String],
    auto_scroll: bool,
    scroll_offset: usize,
    usable_height: usize,
    get_lines: F,
) -> (usize, usize)
where
    F: Fn(&str) -> usize,
{
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let (visible, _offset) = calc_auto_scroll(messages, usable_height, get_lines);
        (visible, total_items.saturating_sub(visible))
    } else {
        let visible = calc_manual_scroll(messages, scroll_offset, usable_height, get_lines);
        (visible, scroll_offset)
    }
}

fn calc_auto_scroll<F>(messages: &[String], usable_height: usize, get_lines: F) -> (usize, usize)
where
    F: Fn(&str) -> usize,
{
    let mut used = 0;
    let mut count = 0;
    for msg in messages.iter().rev() {
        let msg_lines = get_lines(msg);
        if used > 0 && used + msg_lines > usable_height {
            break;
        }
        used += msg_lines;
        count += 1;
    }
    (count, 0)
}

fn calc_manual_scroll<F>(
    messages: &[String],
    scroll_offset: usize,
    usable_height: usize,
    get_lines: F,
) -> usize
where
    F: Fn(&str) -> usize,
{
    if scroll_offset >= messages.len() {
        return MIN_VISIBLE;
    }
    let mut used = 0;
    let mut count = 0;
    for msg in messages.iter().skip(scroll_offset) {
        let msg_lines = get_lines(msg);
        if used > 0 && used + msg_lines > usable_height {
            break;
        }
        used += msg_lines;
        count += 1;
    }
    count.max(MIN_VISIBLE)
}

/// Return a checkmark prefix (`"v "`) if any peer has acknowledged the given
/// broadcast message, or two spaces otherwise.
#[must_use]
pub fn broadcast_receipt_prefix(
    msg_id: Option<&str>,
    broadcast_receipts: &HashMap<String, HashMap<String, f64>>,
) -> &'static str {
    match msg_id {
        Some(msg_id) => {
            let confirmed = broadcast_receipts
                .get(msg_id)
                .map_or(0, std::collections::HashMap::len);
            if confirmed == 0 { "  " } else { "v " }
        }
        _ => "  ",
    }
}

/// Return a checkmark prefix (`"v "`) if the recipient has acknowledged the
/// given direct message, or two spaces otherwise.
#[must_use]
pub fn dm_receipt_prefix(
    msg_id: Option<&str>,
    dm_receipts: &HashMap<String, (String, f64)>,
) -> &'static str {
    match msg_id {
        Some(msg_id) if dm_receipts.contains_key(msg_id) => "v ",
        _ => "  ",
    }
}

/// Map a clicked terminal row to the index of the message it falls within,
/// given each message's rendered line count and the row where content starts.
/// Returns `None` if the click was above the content area or past the last message.
#[must_use]
pub fn row_to_visible_index(
    line_counts: &[usize],
    first_content_row: usize,
    click_row: usize,
) -> Option<usize> {
    if click_row < first_content_row {
        return None;
    }
    let mut current_row = first_content_row;

    for (idx, line_count) in line_counts.iter().copied().enumerate() {
        let message_end_row = current_row + line_count;
        if click_row < message_end_row {
            return Some(idx);
        }
        current_row = message_end_row;
    }

    None
}

/// Get the tab content based on active tab index
#[must_use]
pub fn get_tab_content(state: &TuiRenderState) -> crate::tui_tabs::TabContent {
    let tab_title = state
        .tab_titles
        .get(state.active_tab)
        .cloned()
        .unwrap_or_default();

    if tab_title.starts_with("DM: ") {
        let peer = tab_title.trim_start_matches("DM: ").to_string();
        crate::tui_tabs::TabContent::Direct(peer)
    } else if tab_title == "Peers" {
        crate::tui_tabs::TabContent::Peers
    } else if tab_title == "Log" {
        crate::tui_tabs::TabContent::Log
    } else {
        crate::tui_tabs::TabContent::Chat
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_render_state.rs"]
mod tests;
