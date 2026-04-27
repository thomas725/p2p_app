use super::{DynamicTabs, TextArea};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub type SharedState = Arc<tokio::sync::Mutex<AppState>>;

/// Shared application state for all tasks
///
/// This struct centralizes all mutable state needed by the TUI.
///
/// Only the CommandProcessor task directly mutates this state.
/// Other tasks:
/// - **RenderLoop**: Read-only access to render current state
/// - **InputHandler**: No direct access, sends InputEvent to CommandProcessor
/// - **SwarmHandler**: No direct access, sends SwarmEvent to CommandProcessor
///
/// This single-writer pattern prevents race conditions and simplifies reasoning about state changes.
pub struct AppState {
    // Messages & Chat
    pub messages: VecDeque<(String, Option<String>)>,
    pub dm_messages: HashMap<String, VecDeque<String>>,

    // Peer Management
    pub peers: VecDeque<(String, String, String)>, // (id, first_seen, last_seen)
    pub concurrent_peers: usize,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,

    // UI State (TUI-specific)
    pub active_tab: usize,
    pub dynamic_tabs: DynamicTabs,
    pub chat_input: TextArea<'static>,
    pub peer_selection: usize, // For navigating peer list
    pub mouse_capture: bool,

    // Scroll State (Chat tab)
    pub chat_scroll_offset: usize,
    pub chat_auto_scroll: bool,
    pub visible_message_count: usize,
    pub chat_message_offset: usize, // Actual starting index for visible messages (set by render loop)
    pub chat_area_height: usize, // Height of message area in rows (set by render loop)

    // Per-DM scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_scroll_state: HashMap<String, (usize, bool)>,

    // Unread Counts
    pub unread_broadcasts: u32,
    pub unread_dms: HashMap<String, u32>,

    // Runtime Context
    pub own_nickname: String,
    pub topic_str: String,

    // Edit Mode
    pub editing_nickname: bool,
}

impl AppState {
    pub fn new(
        topic_str: String,
        own_nickname: String,
        local_nicknames: HashMap<String, String>,
        received_nicknames: HashMap<String, String>,
        initial_messages: VecDeque<(String, Option<String>)>,
        initial_peers: VecDeque<(String, String, String)>,
    ) -> Self {
        Self {
            messages: initial_messages,
            dm_messages: HashMap::new(),
            peers: initial_peers,
            dynamic_tabs: DynamicTabs::new(),
            active_tab: 0,
            chat_input: TextArea::default(),
            peer_selection: 0,
            concurrent_peers: 0,
            mouse_capture: true,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            visible_message_count: 1,
            chat_message_offset: 0,
            chat_area_height: 0,
            dm_scroll_state: HashMap::new(),
            own_nickname,
            local_nicknames,
            received_nicknames,
            unread_broadcasts: 0,
            unread_dms: HashMap::new(),
            topic_str,
            editing_nickname: false,
        }
    }
}

pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> VecDeque<(String, Option<String>)> {
    let mut messages = VecDeque::new();
    if let Ok(db_messages) = p2p_app::load_messages(topic_str, max_messages) {
        for msg in db_messages.iter().rev() {
            let ts = p2p_app::format_peer_datetime(msg.created_at);
            let sender = msg
                .peer_id
                .as_ref()
                .map(|p| {
                    let display =
                        p2p_app::peer_display_name(p, local_nicknames, received_nicknames);
                    format!("[{}]", display)
                })
                .unwrap_or_else(|| format!("[{}]", own_nickname));
            messages.push_back((
                format!("{} {} {}", ts, sender, msg.content),
                msg.peer_id.clone(),
            ));
        }
    } else {
        p2p_app::p2plog_debug("Failed to load messages from database");
    }
    messages
}
