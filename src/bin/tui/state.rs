use super::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Shared application state for all tasks
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
    pub dm_inputs: HashMap<String, TextArea<'static>>,
    pub peer_selection: usize,
    pub mouse_capture: bool,

    // Scroll State
    pub debug_scroll_offset: usize,
    pub debug_auto_scroll: bool,
    pub chat_scroll_offset: usize,
    pub chat_auto_scroll: bool,

    // Unread Counts
    pub unread_broadcasts: u32,
    pub unread_dms: HashMap<String, u32>,

    // Runtime Context
    pub own_nickname: String,
    pub topic_str: String,
    pub logs: Arc<Mutex<VecDeque<String>>>,
}

impl AppState {
    pub fn new(
        topic_str: String,
        logs: Arc<Mutex<VecDeque<String>>>,
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
            dm_inputs: HashMap::new(),
            chat_input: TextArea::default(),
            concurrent_peers: 0,
            peer_selection: 0,
            mouse_capture: false,
            debug_scroll_offset: 0,
            debug_auto_scroll: true,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            own_nickname,
            local_nicknames,
            received_nicknames,
            unread_broadcasts: 0,
            unread_dms: HashMap::new(),
            topic_str,
            logs,
        }
    }
}

pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    logs: &Arc<Mutex<VecDeque<String>>>,
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
        let _ = p2p_app::log_debug(
            logs,
            format!("Loaded {} messages from database", db_messages.len()),
        );
    } else {
        let _ = p2p_app::log_debug(logs, "Failed to load messages from database".to_string());
    }
    messages
}
