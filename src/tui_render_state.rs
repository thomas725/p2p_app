//! TUI rendering state abstraction for testing
//!
//! This module provides a render state that can be used by both the binary
//! and integration tests. The binary uses AppState, tests use this abstraction.

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
    /// Message IDs for sent messages (None if from other peers)
    pub message_ids: VecDeque<Option<String>>,
    /// Receipt status for broadcast messages: peer_id -> (msg_id -> timestamp)
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    /// Known peers: (peer_id, first_seen, last_seen)
    pub peers: Vec<(String, String, String)>,
    /// Direct messages per peer
    pub dm_messages: BTreeMap<String, VecDeque<String>>,
    /// Message IDs for DMs per peer
    pub dm_message_ids: BTreeMap<String, VecDeque<Option<String>>>,
    /// DM receipt status per peer: (source_peer, timestamp)
    pub dm_receipts: HashMap<String, (String, f64)>,
    /// Current input text in message box
    pub input_text: String,
    /// Whether we're editing a peer's nickname
    pub editing_nickname: bool,
    /// Peer ID being edited (if editing_nickname is true)
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
    /// Scroll state per DM peer: (offset, auto_scroll)
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
    pub fn new() -> Self {
        Self {
            tab_titles: vec!["Chat".into(), "Peers".into(), "Log".into()],
            active_tab: 0,
            messages: VecDeque::new(),
            message_ids: VecDeque::new(),
            broadcast_receipts: HashMap::new(),
            peers: Vec::new(),
            dm_messages: BTreeMap::new(),
            dm_message_ids: BTreeMap::new(),
            dm_receipts: HashMap::new(),
            input_text: String::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 0,
            mouse_capture: false,
            popup: None,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            dm_scroll_state: BTreeMap::new(),
            dm_broadcast_scroll_state: BTreeMap::new(),
            broadcast_selection: None,
            peer_selection: 0,
        }
    }

    /// Create with sample data for testing
    pub fn with_sample_data() -> Self {
        let mut messages = VecDeque::new();
        messages.push_back("[You] Hello world".into());
        messages.push_back("[Peer1] How are you?".into());
        messages.push_back("[You] I'm good!".into());

        let peers = vec![
            ("12D3KooWH123456".into(), "Alice".into(), "Online".into()),
            ("12D3KooWH789012".into(), "Bob".into(), "Online".into()),
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
            message_ids: VecDeque::new(),
            broadcast_receipts: HashMap::new(),
            peers,
            dm_messages,
            dm_message_ids,
            dm_receipts: HashMap::new(),
            input_text: String::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 2,
            mouse_capture: false,
            popup: None,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            dm_scroll_state: BTreeMap::new(),
            dm_broadcast_scroll_state: BTreeMap::new(),
            broadcast_selection: None,
            peer_selection: 0,
        }
    }

    /// Add a message to chat
    pub fn add_message(&mut self, msg: impl Into<String>) {
        self.messages.push_back(msg.into());
    }

    /// Add a peer
    pub fn add_peer(
        &mut self,
        id: impl Into<String>,
        name: impl Into<String>,
        status: impl Into<String>,
    ) {
        self.peers.push((id.into(), name.into(), status.into()));
    }

    /// Add a DM message for a peer
    pub fn add_dm_message(&mut self, peer: impl Into<String>, msg: impl Into<String>) {
        let peer = peer.into();
        let entry = self.dm_messages.entry(peer).or_default();
        entry.push_back(msg.into());
    }
}

/// Calculate visible items and effective offset for string-based messages with auto/manual scroll
pub fn calc_visible_strings(
    messages: &VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let msgs: Vec<String> = messages.iter().cloned().collect();
    calc_visible_impl(
        &msgs,
        auto_scroll,
        scroll_offset,
        text_width,
        usable_height,
        |m| count_lines(m, text_width),
    )
}

/// Count wrapped lines of text, accounting for ANSI codes and terminal width
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
    _text_width: usize,
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

pub fn broadcast_receipt_prefix(
    msg_id: Option<&str>,
    broadcast_receipts: &HashMap<String, HashMap<String, f64>>,
) -> &'static str {
    match msg_id {
        Some(msg_id) => {
            let confirmed = broadcast_receipts.get(msg_id).map(|m| m.len()).unwrap_or(0);
            if confirmed == 0 { "  " } else { "v " }
        }
        _ => "  ",
    }
}

pub fn dm_receipt_prefix(
    msg_id: Option<&str>,
    dm_receipts: &HashMap<String, (String, f64)>,
) -> &'static str {
    match msg_id {
        Some(msg_id) if dm_receipts.contains_key(msg_id) => "v ",
        _ => "  ",
    }
}

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
mod tests {
    use super::*;
    use crate::tui_tabs::TabContent;

    #[test]
    fn test_tui_render_state_default() {
        let state = TuiRenderState::default();
        assert_eq!(state.active_tab, 0);
        assert_eq!(state.tab_titles.len(), 3);
        assert!(state.messages.is_empty());
        assert!(state.peers.is_empty());
    }

    #[test]
    fn test_tui_render_state_new() {
        let state = TuiRenderState::new();
        assert!(state.connected);
        assert!(!state.mouse_capture);
        assert!(state.popup.is_none());
    }

    #[test]
    fn test_tui_render_state_with_sample_data() {
        let state = TuiRenderState::with_sample_data();
        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.peers.len(), 2);
        assert_eq!(state.peer_count, 2);
    }

    #[test]
    fn test_tui_render_state_add_message() {
        let mut state = TuiRenderState::new();
        state.add_message("Hello");
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0], "Hello");
    }

    #[test]
    fn test_tui_render_state_add_peer() {
        let mut state = TuiRenderState::new();
        state.add_peer("peer1", "Alice", "Online");
        assert_eq!(state.peers.len(), 1);
        assert_eq!(state.peers[0].0, "peer1");
        assert_eq!(state.peers[0].1, "Alice");
    }

    #[test]
    fn test_tui_render_state_add_dm_message() {
        let mut state = TuiRenderState::new();
        state.add_dm_message("peer1", "Hello there");
        assert!(state.dm_messages.contains_key("peer1"));
        assert_eq!(state.dm_messages["peer1"].len(), 1);
    }

    #[test]
    fn test_get_tab_content_chat() {
        let mut state = TuiRenderState::new();
        state.active_tab = 0;
        assert_eq!(get_tab_content(&state), TabContent::Chat);
    }

    #[test]
    fn test_get_tab_content_peers() {
        let mut state = TuiRenderState::new();
        state.active_tab = 1;
        assert_eq!(get_tab_content(&state), TabContent::Peers);
    }

    #[test]
    fn test_get_tab_content_log() {
        let mut state = TuiRenderState::new();
        state.active_tab = 2;
        assert_eq!(get_tab_content(&state), TabContent::Log);
    }

    #[test]
    fn test_get_tab_content_direct() {
        let mut state = TuiRenderState::new();
        state.tab_titles.push("DM: Alice".to_string());
        state.active_tab = 3;
        if let TabContent::Direct(peer) = get_tab_content(&state) {
            assert_eq!(peer, "Alice");
        } else {
            panic!("expected Direct");
        }
    }

    #[test]
    fn test_tui_tab_content_is_input_enabled() {
        assert!(TabContent::Chat.is_input_enabled());
        assert!(TabContent::Direct("peer".to_string()).is_input_enabled());
        assert!(!TabContent::Peers.is_input_enabled());
        assert!(!TabContent::Log.is_input_enabled());
    }

    #[test]
    fn test_tui_render_state_clone() {
        let state = TuiRenderState::new();
        let cloned = state.clone();
        assert_eq!(cloned.active_tab, state.active_tab);
        assert_eq!(cloned.messages.len(), state.messages.len());
    }

    #[test]
    fn test_count_lines_single_short_line() {
        assert_eq!(count_lines("hello", 80), 1);
    }

    #[test]
    fn test_count_lines_wraps_long_line() {
        assert_eq!(count_lines("hello world foo bar baz quux", 10), 3);
    }

    #[test]
    fn test_count_lines_empty_string() {
        assert_eq!(count_lines("", 80), 1);
    }

    #[test]
    fn test_count_lines_newlines() {
        assert_eq!(count_lines("line1\nline2\nline3", 80), 3);
    }

    #[test]
    fn test_calc_visible_strings_empty() {
        let msgs: VecDeque<String> = VecDeque::new();
        let (visible, offset) = calc_visible_strings(&msgs, true, 0, 80, 10);
        assert_eq!(visible, 0);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_calc_visible_strings_auto_scroll_shows_all() {
        let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string()]);
        let (visible, offset) = calc_visible_strings(&msgs, true, 0, 80, 10);
        assert_eq!(visible, 2);
        assert_eq!(offset, 0);
    }

    #[test]
    fn test_calc_visible_strings_manual_scroll() {
        let msgs = VecDeque::from(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        let (visible, offset) = calc_visible_strings(&msgs, false, 1, 80, 10);
        assert_eq!(visible, 2);
        assert_eq!(offset, 1);
    }

    #[test]
    fn test_broadcast_receipt_prefix_no_receipts() {
        let receipts = HashMap::new();
        assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "  ");
    }

    #[test]
    fn test_broadcast_receipt_prefix_with_receipt() {
        let mut receipts = HashMap::new();
        receipts.insert(
            "msg-1".to_string(),
            HashMap::from([("peer-1".to_string(), 1.0)]),
        );
        assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "v ");
    }

    #[test]
    fn test_broadcast_receipt_prefix_no_msg_id() {
        let receipts = HashMap::new();
        assert_eq!(broadcast_receipt_prefix(None, &receipts), "  ");
    }

    #[test]
    fn test_dm_receipt_prefix_no_receipt() {
        let receipts = HashMap::new();
        assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "  ");
    }

    #[test]
    fn test_dm_receipt_prefix_with_receipt() {
        let receipts = HashMap::from([("msg-1".to_string(), ("peer-1".to_string(), 1.0))]);
        assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "v ");
    }

    #[test]
    fn test_tui_render_state_debug_format() {
        let state = TuiRenderState::new();
        let debug_str = format!("{:?}", state);
        assert!(debug_str.contains("TuiRenderState"));
    }

    #[test]
    fn test_tui_tab_content_clone() {
        let content = TabContent::Direct("peer1".to_string());
        let cloned = content.clone();
        assert_eq!(cloned, content);
    }

    #[test]
    fn test_tui_tab_content_debug_format() {
        let content = TabContent::Chat;
        let debug_str = format!("{:?}", content);
        assert!(debug_str.contains("Chat"));
    }

    #[test]
    fn test_row_to_visible_index_before_first_content() {
        assert_eq!(row_to_visible_index(&[1, 1], 2, 1), None);
    }

    #[test]
    fn test_row_to_visible_index_at_first_content_row() {
        assert_eq!(row_to_visible_index(&[1, 1], 2, 2), Some(0));
    }

    #[test]
    fn test_row_to_visible_index_within_first_message() {
        let line_counts = vec![3, 1, 2];
        assert_eq!(row_to_visible_index(&line_counts, 0, 0), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 0, 1), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 0, 2), Some(0));
    }

    #[test]
    fn test_row_to_visible_index_within_second_message() {
        let line_counts = vec![3, 1, 2];
        assert_eq!(row_to_visible_index(&line_counts, 0, 3), Some(1));
    }

    #[test]
    fn test_row_to_visible_index_within_third_message() {
        let line_counts = vec![3, 1, 2];
        assert_eq!(row_to_visible_index(&line_counts, 0, 4), Some(2));
        assert_eq!(row_to_visible_index(&line_counts, 0, 5), Some(2));
    }

    #[test]
    fn test_row_to_visible_index_beyond_all_messages() {
        let line_counts = vec![3, 1, 2];
        let total_lines: usize = line_counts.iter().sum();
        assert_eq!(row_to_visible_index(&line_counts, 0, total_lines), None);
    }

    #[test]
    fn test_row_to_visible_index_empty() {
        assert_eq!(row_to_visible_index(&[], 0, 0), None);
    }

    #[test]
    fn test_row_to_visible_index_first_content_row_nonzero() {
        let line_counts = vec![5, 3];
        assert_eq!(row_to_visible_index(&line_counts, 3, 3), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 3, 7), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 3, 8), Some(1));
        assert_eq!(row_to_visible_index(&line_counts, 3, 10), Some(1));
        assert_eq!(row_to_visible_index(&line_counts, 3, 11), None);
    }
}
