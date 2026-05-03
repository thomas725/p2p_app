//! TUI rendering state abstraction for testing
//!
//! This module provides a render state that can be used by both the binary
//! and integration tests. The binary uses AppState, tests use this abstraction.

use std::collections::{BTreeMap, VecDeque};

/// Minimum state needed to render a TUI frame
#[derive(Clone, Debug)]
pub struct TuiRenderState {
    /// Tab titles
    pub tab_titles: Vec<String>,
    /// Active tab index
    pub active_tab: usize,
    /// Chat messages to display
    pub messages: VecDeque<String>,
    /// Peer list: (peer_id, nickname, status)
    pub peers: Vec<(String, String, String)>,
    /// DM conversations: peer_id -> messages
    pub dm_messages: BTreeMap<String, VecDeque<String>>,
    /// Current input text
    pub input_text: String,
    /// Whether editing nickname
    pub editing_nickname: bool,
    /// Peer ID for nickname editing
    pub nickname_peer_id: String,
    /// Connection status
    pub connected: bool,
    /// Concurrent peer count
    pub peer_count: usize,
    /// Mouse capture enabled
    pub mouse_capture: bool,
    /// Popup text if any
    pub popup: Option<String>,
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
            peers: Vec::new(),
            dm_messages: BTreeMap::new(),
            input_text: String::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 0,
            mouse_capture: false,
            popup: None,
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

        Self {
            tab_titles: vec![
                "Chat".into(),
                "Peers".into(),
                "Log".into(),
                "DM: Alice".into(),
            ],
            active_tab: 0,
            messages,
            peers,
            dm_messages,
            input_text: String::new(),
            editing_nickname: false,
            nickname_peer_id: String::new(),
            connected: true,
            peer_count: 2,
            mouse_capture: false,
            popup: None,
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

/// Get the tab content based on active tab index
pub fn get_tab_content(state: &TuiRenderState) -> TuiTabContent {
    let tab_title = state
        .tab_titles
        .get(state.active_tab)
        .cloned()
        .unwrap_or_default();

    if tab_title.starts_with("DM: ") {
        let peer = tab_title.trim_start_matches("DM: ").to_string();
        TuiTabContent::Direct(peer)
    } else if tab_title == "Peers" {
        TuiTabContent::Peers
    } else if tab_title == "Log" {
        TuiTabContent::Log
    } else {
        TuiTabContent::Chat
    }
}

/// Tab content enum
#[derive(Clone, Debug)]
pub enum TuiTabContent {
    /// Broadcast chat view
    Chat,
    /// Peer list view
    Peers,
    /// Direct message view for the given peer ID
    Direct(String),
    /// Debug/log view
    Log,
}

impl TuiTabContent {
    /// Returns `true` if the input box should be enabled for this tab.
    pub fn is_input_enabled(&self) -> bool {
        matches!(self, TuiTabContent::Chat | TuiTabContent::Direct(_))
    }
}
