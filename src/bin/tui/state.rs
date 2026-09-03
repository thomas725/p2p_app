use super::{DynamicTabs, TextArea};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

/// Channel capacity for inter-task communication (events per task)
pub const CHANNEL_CAPACITY: usize = 100;
/// Maximum direct messages to keep per peer conversation.
pub use p2p_app::types::MAX_DM_HISTORY;
/// Maximum messages to keep in memory (older messages are dropped).
pub use p2p_app::types::MAX_MESSAGE_HISTORY;
/// Input poll interval in milliseconds
pub const FRAME_TIME_MS: u64 = 16;

/// Trim a `VecDeque` to a maximum length, removing oldest (front) items.
pub fn trim_history<T>(queue: &mut VecDeque<T>, limit: usize) {
    while queue.len() > limit {
        queue.pop_front();
    }
}

pub type SharedState = Arc<tokio::sync::Mutex<AppState>>;

/// Shared application state for all tasks
///
/// This struct centralizes all mutable state needed by the TUI.
/// Only the `CommandProcessor` task directly mutates this state.
/// Other tasks:
/// - **`RenderLoop`**: Read-only access to render current state
/// - **`InputHandler`**: No direct access, sends `InputEvent` to `CommandProcessor`
/// - **`SwarmHandler`**: No direct access, sends `SwarmEvent` to `CommandProcessor`
///
/// This single-writer pattern prevents race conditions and simplifies reasoning about state changes.
#[allow(clippy::struct_excessive_bools)]
pub struct AppState {
    // Messages & Chat
    pub messages: VecDeque<p2p_app::DisplayMessage>,
    // Message IDs aligned with `messages` (used for receipts / click actions).
    pub message_ids: VecDeque<Option<String>>,
    // Broadcast receipts: msg_id -> (peer_id -> received_at epoch seconds).
    pub broadcast_receipts: HashMap<String, HashMap<String, f64>>,
    pub dm_messages: HashMap<String, VecDeque<String>>,
    // DM message IDs aligned with dm_messages[peer_id].
    pub dm_message_ids: HashMap<String, VecDeque<Option<String>>>,
    // DM receipts: msg_id -> (peer_id, received_at epoch seconds).
    pub dm_receipts: HashMap<String, (String, f64)>,

    // Peer Management
    pub peers: VecDeque<p2p_app::PeerRecord>,
    pub concurrent_peers: usize,
    // How many broadcasts *we* sent to each peer (bulk-loaded from the
    // `broadcast_recipients` table via `messages::get_all_peer_stats`, refreshed
    // whenever peers change). Drives the peers-table "Broadcast" column.
    pub broadcast_sent_to_peer: HashMap<String, usize>,
    // Currently connected peers (shared connected-tracking logic with the
    // Flutter backend). Mirror of Flutter's "Connected peers" on Settings.
    pub connected: p2p_app::connected::ConnectedTracker,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,
    // Per-peer self nickname override: peer_id -> nickname we present to that peer.
    pub self_nicknames_for_peers: HashMap<String, String>,

    // UI State (TUI-specific)
    pub active_tab: usize,
    pub dynamic_tabs: DynamicTabs,
    pub chat_input: TextArea<'static>,
    pub peer_selection: usize,     // For navigating peer list
    pub peer_sort_column: usize, // Active sort column (0=Name,1=DM,2=Broadcast,3=Last Seen,4=First Seen)
    pub peer_sort_ascending: bool, // Whether the peer list is sorted ascending on the active column
    pub peer_table_offset: usize, // First visible row of the peer table (set by render loop)
    // One-shot flag: display names have been resolved for every known peer the
    // first time the Peers tab is opened (warms the library's name cache).
    pub peer_names_warmed: bool,
    // Whether the terminal genuinely delivers kitty-keyboard encoding (from
    // crossterm's `supports_keyboard_enhancement()`). On WezTerm (kitty active)
    // Ctrl+I arrives as `Char('i')`+CONTROL and opens PeerInfo on a Direct tab.
    // On collapsed terminals (Konsole) Ctrl+I (and Ctrl+Shift+I) collapse to
    // the bare `0x09` Tab byte, so Ctrl+I opens PeerInfo only on kitty
    // terminals. Ctrl+P (`Char('p')`+CONTROL from 0x10 DLE) works on every
    // terminal as a universal fallback. Bare Tab always cycles tabs either way.
    pub kitty_keyboard_active: bool,
    pub mouse_capture: bool,
    pub last_mouse_row: u16, // For mouse-targeted scroll behavior in split layouts

    // Scroll State (Chat tab)
    pub chat_scroll_offset: usize,
    pub chat_auto_scroll: bool,
    pub chat_unread_count: usize, // Unread broadcast messages while scrolled up
    pub chat_area_height: usize,  // Height of message area in rows (set by render loop)
    pub terminal_width: usize,    // Terminal width in columns (set by render loop)

    // Scroll State (Log tab)
    pub log_scroll_offset: usize,
    pub log_auto_scroll: bool,

    // Per-DM scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_scroll_state: HashMap<String, (usize, bool)>,
    // Per-DM broadcast scroll state: peer_id -> (scroll_offset, auto_scroll)
    pub dm_broadcast_scroll_state: HashMap<String, (usize, bool)>,
    // Selected broadcast message in broadcast chat tab
    pub broadcast_selection: Option<usize>,

    // Runtime Context
    pub own_nickname: String,
    pub local_peer_id: String,
    pub topic_str: String,

    // Edit Mode
    pub editing_nickname: bool,
    pub editing_nickname_peer: Option<String>,

    // Ad-hoc UI popup (used for receipt timing details, etc.)
    pub popup: Option<String>,

    // Settings tab (read-only node/network diagnostics)
    pub db_url: String,
    pub platform: String,
    pub network_size: String,
    pub listen_addrs: Vec<String>,
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

    /// Re-sort the peer list by the currently active column/order, keeping the
    /// selected peer (by id) selected. Also refreshes the per-peer
    /// `broadcast_sent_to_peer` counts (a bulk DB call) so the sort and the
    /// rendered "Broadcast" column never show stale sent-to-peer numbers.
    pub fn resort_peers(&mut self) {
        self.broadcast_sent_to_peer = p2p_app::messages::get_all_peer_stats()
            .map(|stats| {
                stats
                    .into_iter()
                    .map(|(pid, s)| (pid, usize::try_from(s.broadcast_sent_to_peer).unwrap_or(0)))
                    .collect()
            })
            .unwrap_or_default();
        self.peer_selection = p2p_app::tui_helpers::sort_peers_by_column(
            &mut self.peers,
            &self.dm_messages,
            &self.broadcast_sent_to_peer,
            self.peer_sort_column,
            self.peer_sort_ascending,
            self.peer_selection,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        topic_str: String,
        own_nickname: String,
        local_peer_id: String,
        local_nicknames: HashMap<String, String>,
        received_nicknames: HashMap<String, String>,
        self_nicknames_for_peers: HashMap<String, String>,
        initial_messages: VecDeque<p2p_app::DisplayMessage>,
        initial_message_ids: VecDeque<Option<String>>,
        initial_peers: VecDeque<p2p_app::PeerRecord>,
        initial_broadcast_receipts: HashMap<String, HashMap<String, f64>>,
        initial_dm_receipts: HashMap<String, (String, f64)>,
    ) -> Self {
        Self {
            messages: initial_messages,
            message_ids: initial_message_ids,
            broadcast_receipts: initial_broadcast_receipts,
            dm_messages: HashMap::new(),
            dm_message_ids: HashMap::new(),
            dm_receipts: initial_dm_receipts,
            peers: initial_peers,
            broadcast_sent_to_peer: HashMap::new(),
            dynamic_tabs: DynamicTabs::new(),
            active_tab: 0,
            chat_input: TextArea::default(),
            peer_selection: 0,
            peer_sort_column: 3,
            peer_sort_ascending: false,
            peer_table_offset: 0,
            peer_names_warmed: false,
            kitty_keyboard_active: true,
            concurrent_peers: 0,
            connected: p2p_app::connected::ConnectedTracker::new(),
            mouse_capture: true,
            last_mouse_row: 0,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            chat_unread_count: 0,
            chat_area_height: 0,
            terminal_width: 0,
            log_scroll_offset: 0,
            log_auto_scroll: true,
            dm_scroll_state: HashMap::new(),
            dm_broadcast_scroll_state: HashMap::new(),
            broadcast_selection: None,
            own_nickname,
            local_peer_id,
            local_nicknames,
            received_nicknames,
            self_nicknames_for_peers,
            topic_str,
            editing_nickname: false,
            editing_nickname_peer: None,
            popup: None,
            db_url: String::new(),
            platform: String::new(),
            network_size: String::new(),
            listen_addrs: Vec::new(),
        }
    }
}

type FormattedMessages = (VecDeque<p2p_app::DisplayMessage>, VecDeque<Option<String>>);

/// Pure: formats DB messages into display-ready `(text, peer_id)` pairs.
///
/// Separated from the DB call so it can be unit-tested without a database.
#[allow(clippy::type_complexity)]
#[allow(clippy::option_if_let_else)]
fn format_messages_from_db(
    db_messages: &[p2p_app::generated::models_queryable::Message],
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    let mut messages = VecDeque::new();
    let mut message_ids = VecDeque::new();
    for msg in db_messages.iter().rev() {
        let ts = p2p_app::format_peer_datetime(msg.created_at);
        let sender = if let Some(ref nick) = msg.sender_nickname {
            format!("[{nick}]")
        } else if let Some(ref pid) = msg.peer_id {
            let display = p2p_app::peer_display_name(pid, local_nicknames, received_nicknames);
            format!("[{display}]")
        } else {
            format!("[{own_nickname}]")
        };
        messages.push_back(p2p_app::DisplayMessage {
            text: format!("{ts} {sender} {}", msg.content),
            sender_peer_id: msg.peer_id.clone(),
        });
        message_ids.push_back(msg.msg_id.clone());
    }
    (messages, message_ids)
}

#[allow(clippy::type_complexity)]
pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    p2p_app::load_messages(topic_str, max_messages).map_or_else(
        |_| {
            p2p_app::p2plog_debug("Failed to load messages from database");
            (VecDeque::new(), VecDeque::new())
        },
        |db_messages| {
            format_messages_from_db(
                &db_messages,
                local_nicknames,
                received_nicknames,
                own_nickname,
            )
        },
    )
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_state.rs"]
mod tests;
