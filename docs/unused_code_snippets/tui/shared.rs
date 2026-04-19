//! Shared types and utilities for the P2P chat TUI application.

use std::collections::VecDeque;

/// Maximum number of messages to keep in memory
pub const MAX_MESSAGES: usize = 1000;
/// Maximum number of log messages to keep in memory
pub const MAX_LOGS: usize = 1000;

/// Represents a chat message with optional sender information.
#[derive(Clone, Debug)]
pub struct ChatMessage {
    /// The message content
    pub content: String,
    /// Optional peer ID for messages from other users
    pub peer_id: Option<String>,
    /// Optional sender nickname
    pub nickname: Option<String>,
    /// Timestamp of when the message was sent
    pub timestamp: String,
}

/// Represents a notification about unread messages
#[derive(Clone, Debug, Default)]
pub struct NotificationState {
    /// Count of unread broadcast messages
    pub unread_broadcasts: u32,
    /// Map of peer IDs to count of unread direct messages
    pub unread_dms: std::collections::BTreeMap<String, u32>,
}

/// Represents the current state of the TUI application
#[derive(Clone, Debug)]
pub struct TuiState {
    /// The chat messages displayed in the chat view
    pub messages: VecDeque<ChatMessage>,
    /// Mapping of message indices to peer IDs for quick lookup
    pub chat_message_peers: Vec<String>,
    /// Currently active tab index
    pub active_tab: usize,
    /// Scroll offset for the chat message list
    pub chat_scroll_offset: usize,
    /// Auto-scroll enabled for chat view
    pub chat_auto_scroll: bool,
    /// Current peer selection in the peers list
    pub peer_selection: usize,
    /// Debug log scroll offset
    pub debug_scroll_offset: usize,
    /// Debug log auto-scroll enabled
    pub debug_auto_scroll: bool,
    /// Terminal width in characters
    pub terminal_width: usize,
    /// Current nickname of the local user
    pub own_nickname: String,
    /// Mapping of peer IDs to their local nicknames
    pub local_nicknames: std::collections::HashMap<String, String>,
    /// Mapping of peer IDs to their received nicknames
    pub received_nicknames: std::collections::HashMap<String, String>,
}

impl Default for TuiState {
    fn default() -> Self {
        Self {
            messages: VecDeque::new(),
            chat_message_peers: Vec::new(),
            active_tab: 0,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            peer_selection: 0,
            debug_scroll_offset: 0,
            debug_auto_scroll: true,
            terminal_width: 80,
            own_nickname: String::new(),
            local_nicknames: std::collections::HashMap::new(),
            received_nicknames: std::collections::HashMap::new(),
        }
    }
}
