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
    // Message IDs aligned with `messages` (used for receipts / click actions).
    pub message_ids: VecDeque<Option<String>>,
    // Broadcast receipts: msg_id -> (peer_id -> received_at epoch seconds).
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    // Outgoing message send times (epoch seconds) for receipt timing.
    pub sent_at_by_msg_id: HashMap<String, f64>,
    pub dm_messages: HashMap<String, VecDeque<String>>,
    // DM message IDs aligned with dm_messages[peer_id].
    pub dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    // DM receipts: msg_id -> (peer_id, received_at epoch seconds).
    pub dm_receipts: HashMap<String, (String, f64)>,

    // Peer Management
    pub peers: VecDeque<(String, String, String)>, // (id, first_seen, last_seen)
    pub concurrent_peers: usize,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,
    // Per-peer self nickname override: peer_id -> nickname we present to that peer.
    pub self_nicknames_for_peers: HashMap<String, String>,

    // UI State (TUI-specific)
    pub active_tab: usize,
    pub dynamic_tabs: DynamicTabs,
    pub chat_input: TextArea<'static>,
    pub peer_selection: usize, // For navigating peer list
    pub mouse_capture: bool,
    pub last_mouse_row: u16, // For hover-based scroll targeting in split layouts

    // Scroll State (Chat tab)
    pub chat_scroll_offset: usize,
    pub chat_auto_scroll: bool,
    pub visible_message_count: usize,
    pub chat_message_offset: usize, // Actual starting index for visible messages (set by render loop)
    pub chat_area_height: usize,    // Height of message area in rows (set by render loop)
    pub chat_message_lines: Vec<usize>, // Line count for each visible message (set by render loop)

    // Scroll State (Log tab)
    pub log_scroll_offset: usize,
    pub log_auto_scroll: bool,
    pub visible_log_count: usize,

    // Per-DM scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_scroll_state: HashMap<String, (usize, bool)>,
    // Per-DM broadcast scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_broadcast_scroll_state: HashMap<String, (usize, bool)>,
    // Visible message counts: peer_id -> (broadcast_count, dm_count)
    pub dm_visible_counts: HashMap<String, (usize, usize)>,
    // Line counts for broadcast messages in DM tab: peer_id -> Vec of line counts
    pub dm_broadcast_message_lines: HashMap<String, Vec<usize>>,
    // Line counts for DM messages in DM tab: peer_id -> Vec of line counts
    pub dm_message_lines: HashMap<String, Vec<usize>>,
    // Broadcast scroll offset for DM tab: peer_id -> offset (for recalculating visible range)
    pub dm_broadcast_offset: HashMap<String, usize>,
    // DM scroll offset for DM tab: peer_id -> offset (for recalculating visible range)
    pub dm_offset: HashMap<String, usize>,
    // DM pane Y position for click mapping: peer_id -> dm_area.y
    pub dm_area_y: HashMap<String, u16>,
    // Selected broadcast message in broadcast chat tab
    pub broadcast_selection: Option<usize>,

    // Unread Counts
    pub unread_broadcasts: u32,
    pub unread_dms: HashMap<String, u32>,

    // Runtime Context
    pub own_nickname: String,
    pub local_peer_id: String,
    pub topic_str: String,

    // Edit Mode
    pub editing_nickname: bool,
    pub editing_nickname_peer: Option<String>,

    // Ad-hoc UI popup (used for receipt timing details, etc.)
    pub popup: Option<String>,
}

impl AppState {
    pub fn cancel_nickname_edit(&mut self) {
        if !self.editing_nickname {
            return;
        }
        self.editing_nickname = false;
        self.editing_nickname_peer = None;
        self.chat_input = TextArea::default();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        topic_str: String,
        own_nickname: String,
        local_peer_id: String,
        local_nicknames: HashMap<String, String>,
        received_nicknames: HashMap<String, String>,
        self_nicknames_for_peers: HashMap<String, String>,
        initial_messages: VecDeque<(String, Option<String>)>,
        initial_message_ids: VecDeque<Option<String>>,
        initial_sent_at: HashMap<String, f64>,
        initial_peers: VecDeque<(String, String, String)>,
        initial_broadcast_receipts: HashMap<String, HashMap<String, f64>>,
        initial_dm_receipts: HashMap<String, (String, f64)>,
    ) -> Self {
        Self {
            messages: initial_messages,
            message_ids: initial_message_ids,
            broadcast_receipts: initial_broadcast_receipts,
            sent_at_by_msg_id: initial_sent_at,
            dm_messages: HashMap::new(),
            dm_message_ids: HashMap::new(),
            dm_receipts: initial_dm_receipts,
            peers: initial_peers,
            dynamic_tabs: DynamicTabs::new(),
            active_tab: 0,
            chat_input: TextArea::default(),
            peer_selection: 0,
            concurrent_peers: 0,
            mouse_capture: true,
            last_mouse_row: 0,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            visible_message_count: 1,
            chat_message_offset: 0,
            chat_area_height: 0,
            chat_message_lines: Vec::new(),
            log_scroll_offset: 0,
            log_auto_scroll: true,
            visible_log_count: 1,
            dm_scroll_state: HashMap::new(),
            dm_broadcast_scroll_state: HashMap::new(),
            dm_visible_counts: HashMap::new(),
            dm_broadcast_message_lines: HashMap::new(),
            dm_message_lines: HashMap::new(),
            dm_broadcast_offset: HashMap::new(),
            dm_offset: HashMap::new(),
            dm_area_y: HashMap::new(),
            broadcast_selection: None,
            own_nickname,
            local_peer_id,
            local_nicknames,
            received_nicknames,
            self_nicknames_for_peers,
            unread_broadcasts: 0,
            unread_dms: HashMap::new(),
            topic_str,
            editing_nickname: false,
            editing_nickname_peer: None,
            popup: None,
        }
    }
}

pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> (
    VecDeque<(String, Option<String>)>,
    VecDeque<Option<String>>,
    HashMap<String, f64>,
) {
    let mut messages = VecDeque::new();
    let mut message_ids = VecDeque::new();
    let mut sent_at_by_msg_id = HashMap::new();
    if let Ok(db_messages) = p2p_app::load_messages(topic_str, max_messages) {
        for msg in db_messages.iter().rev() {
            let ts = p2p_app::format_peer_datetime(msg.created_at);
            // For outgoing messages (peer_id = None), use sender_nickname if stored, else own_nickname
            // For incoming messages, use stored sender_nickname or fallback to lookup
            let sender = if msg.peer_id.is_none() {
                // Outgoing message - use stored sender nickname or current own nickname
                msg.sender_nickname
                    .as_ref()
                    .map(|n| format!("[{}]", n))
                    .unwrap_or_else(|| format!("[{}]", own_nickname))
            } else {
                // Incoming message - prefer stored sender_nickname, fallback to lookup
                msg.sender_nickname
                    .as_ref()
                    .map(|n| format!("[{}]", n))
                    .unwrap_or_else(|| {
                        let p = msg.peer_id.as_ref().unwrap();
                        let display =
                            p2p_app::peer_display_name(p, local_nicknames, received_nicknames);
                        format!("[{}]", display)
                    })
            };
            messages.push_back((
                format!("{} {} {}", ts, sender, msg.content),
                msg.peer_id.clone(),
            ));
            message_ids.push_back(msg.msg_id.clone());
            if let Some(msg_id) = &msg.msg_id
                && let Some(sent_at) = msg.sent_at
            {
                sent_at_by_msg_id.insert(msg_id.clone(), sent_at);
            }
        }
    } else {
        p2p_app::p2plog_debug("Failed to load messages from database");
    }
    (messages, message_ids, sent_at_by_msg_id)
}
