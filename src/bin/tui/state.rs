use super::{DynamicTabs, TextArea};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

pub type SharedState = Arc<tokio::sync::Mutex<AppState>>;

/// Shared application state for all tasks
///
/// This struct centralizes all mutable state needed by the TUI.
///
/// Only the `CommandProcessor` task directly mutates this state.
/// Other tasks:
/// - **`RenderLoop`**: Read-only access to render current state
/// - **`InputHandler`**: No direct access, sends `InputEvent` to `CommandProcessor`
/// - **`SwarmHandler`**: No direct access, sends `SwarmEvent` to `CommandProcessor`
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
            topic_str,
            editing_nickname: false,
            editing_nickname_peer: None,
            popup: None,
        }
    }
}

type FormattedMessages = (
    VecDeque<(String, Option<String>)>,
    VecDeque<Option<String>>,
    HashMap<String, f64>,
);

/// Pure: formats DB messages into display-ready `(text, peer_id)` pairs.
///
/// Separated from the DB call so it can be unit-tested without a database.
#[allow(clippy::type_complexity)]
pub fn format_messages_from_db(
    db_messages: &[p2p_app::generated::models_queryable::Message],
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    let mut messages = VecDeque::new();
    let mut message_ids = VecDeque::new();
    let mut sent_at_by_msg_id = HashMap::new();
    for msg in db_messages.iter().rev() {
        let ts = p2p_app::format_peer_datetime(msg.created_at);
        let sender = if msg.peer_id.is_none() {
            msg.sender_nickname
                .as_ref()
                .map_or_else(|| format!("[{own_nickname}]"), |n| format!("[{n}]"))
        } else {
            msg.sender_nickname.as_ref().map_or_else(
                || {
                    let p = msg.peer_id.as_ref().unwrap();
                    let display =
                        p2p_app::peer_display_name(p, local_nicknames, received_nicknames);
                    format!("[{display}]")
                },
                |n| format!("[{n}]"),
            )
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
    (messages, message_ids, sent_at_by_msg_id)
}

#[allow(clippy::type_complexity)]
pub fn load_and_format_messages(
    topic_str: &str,
    max_messages: usize,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> FormattedMessages {
    if let Ok(db_messages) = p2p_app::load_messages(topic_str, max_messages) {
        format_messages_from_db(
            &db_messages,
            local_nicknames,
            received_nicknames,
            own_nickname,
        )
    } else {
        p2p_app::p2plog_debug("Failed to load messages from database");
        (VecDeque::new(), VecDeque::new(), HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use p2p_app::generated::models_queryable::Message;

    fn msg(
        content: &str,
        peer_id: Option<&str>,
        sender_nickname: Option<&str>,
        msg_id: Option<&str>,
        sent_at: Option<f64>,
        created_at: &str,
    ) -> Message {
        let dt = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S").unwrap();
        Message {
            id: 0,
            created_at: dt,
            content: content.to_string(),
            peer_id: peer_id.map(String::from),
            topic: "test".to_string(),
            sent: 0,
            is_direct: 0,
            target_peer: None,
            msg_id: msg_id.map(String::from),
            sent_at,
            sender_nickname: sender_nickname.map(String::from),
        }
    }

    #[test]
    fn test_format_messages_from_db_empty() {
        let (msgs, ids, sent_at) =
            format_messages_from_db(&[], &HashMap::new(), &HashMap::new(), "Me");
        assert!(msgs.is_empty());
        assert!(ids.is_empty());
        assert!(sent_at.is_empty());
    }

    #[test]
    fn test_format_outgoing_without_sender_nickname() {
        let messages = [msg(
            "hello",
            None,
            None,
            Some("m1"),
            Some(1.0),
            "2024-01-01 12:00:00",
        )];
        let (msgs, ids, sent_at) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].0.contains("[Me]"));
        assert!(msgs[0].0.contains("hello"));
        assert_eq!(ids[0], Some("m1".to_string()));
        assert_eq!(sent_at.get("m1"), Some(&1.0));
    }

    #[test]
    fn test_format_outgoing_with_sender_nickname() {
        let messages = [msg(
            "hello",
            None,
            Some("OldNick"),
            Some("m1"),
            None,
            "2024-01-01 12:00:00",
        )];
        let (msgs, _, _) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert!(msgs[0].0.contains("[OldNick]"));
        assert!(msgs[0].0.contains("hello"));
    }

    #[test]
    fn test_format_incoming_without_sender_nickname_falls_back_to_display_name() {
        let messages = [msg(
            "hi there",
            Some("peer-abc"),
            None,
            None,
            None,
            "2024-01-01 12:00:00",
        )];
        let local = HashMap::from([("peer-abc".to_string(), "Alice".to_string())]);
        let (msgs, _, _) = format_messages_from_db(&messages, &local, &HashMap::new(), "Me");
        assert!(msgs[0].0.contains("[Alice]"));
        assert!(msgs[0].0.contains("hi there"));
    }

    #[test]
    fn test_format_incoming_with_sender_nickname() {
        let messages = [msg(
            "hey",
            Some("peer-abc"),
            Some("Bob"),
            None,
            None,
            "2024-01-01 12:00:00",
        )];
        let (msgs, _, _) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert!(msgs[0].0.contains("[Bob]"));
        assert!(msgs[0].0.contains("hey"));
    }

    #[test]
    fn test_format_messages_reverses_newest_first_to_oldest_first() {
        let messages = vec![
            msg(
                "second",
                Some("p1"),
                None,
                None,
                None,
                "2024-01-01 12:00:01",
            ),
            msg("first", Some("p1"), None, None, None, "2024-01-01 12:00:00"),
        ];
        let (msgs, _, _) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert_eq!(msgs.len(), 2);
        assert!(
            msgs[0].0.contains("first"),
            "first msg should be oldest after rev"
        );
        assert!(
            msgs[1].0.contains("second"),
            "second msg should be newest after rev"
        );
    }

    #[test]
    fn test_format_messages_sent_at_skipped_when_none() {
        let messages = [msg(
            "no sent_at",
            None,
            None,
            Some("m1"),
            None,
            "2024-01-01 12:00:00",
        )];
        let (_, _, sent_at) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert!(sent_at.is_empty());
    }

    #[test]
    fn test_format_messages_peer_id_maps_correctly() {
        let messages = [msg(
            "incoming",
            Some("p1"),
            None,
            None,
            None,
            "2024-01-01 12:00:00",
        )];
        let (msgs, _, _) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert_eq!(msgs[0].1, Some("p1".to_string()));
    }

    #[test]
    fn test_format_messages_outgoing_peer_id_is_none() {
        let messages = [msg(
            "outgoing",
            None,
            None,
            None,
            None,
            "2024-01-01 12:00:00",
        )];
        let (msgs, _, _) =
            format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
        assert_eq!(msgs[0].1, None);
    }
}
