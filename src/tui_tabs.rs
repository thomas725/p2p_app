//! TUI tab management and navigation

use std::collections::VecDeque;

/// Number of fixed tabs before DM tabs (Chat, Peers)
pub(crate) const FIXED_TAB_COUNT: usize = 2;

/// Direct message tab with peer ID and message history
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmTab {
    /// The full peer ID this DM tab is associated with
    pub peer_id: String,
    /// Scrollback buffer of messages in this conversation
    pub messages: VecDeque<String>,
}

impl DmTab {
    /// Create a new DM tab for a peer
    #[must_use]
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            messages: VecDeque::new(),
        }
    }

    /// Test utility: Create a DmTab with initial test messages.
    #[cfg(any(test, feature = "test-utils"))]
    #[must_use]
    pub fn with_messages(peer_id: String, messages: VecDeque<String>) -> Self {
        Self { peer_id, messages }
    }

    /// Get last 8 characters of peer ID for display
    #[must_use]
    pub fn short_id(&self) -> String {
        crate::fmt::short_peer_id(&self.peer_id)
    }
}

/// Dynamic tab management for direct message conversations
#[derive(Clone, Debug, Default)]
pub struct DynamicTabs {
    /// Active DM tabs, one per open conversation
    pub dm_tabs: Vec<DmTab>,
}

impl DynamicTabs {
    /// Create new empty dynamic tabs
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or retrieve index of DM tab for peer
    pub fn add_dm_tab(&mut self, peer_id: String) -> usize {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            return pos + FIXED_TAB_COUNT;
        }
        let idx = self.dm_tabs.len() + FIXED_TAB_COUNT;
        self.dm_tabs.push(DmTab::new(peer_id));
        idx
    }

    /// Remove DM tab for peer, return its previous index
    pub fn remove_dm_tab(&mut self, peer_id: &str) -> Option<usize> {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            self.dm_tabs.remove(pos);
            return Some(pos + FIXED_TAB_COUNT);
        }
        None
    }

    /// Get DM tab by peer ID (read-only)
    #[must_use]
    pub fn get_dm_tab(&self, peer_id: &str) -> Option<&DmTab> {
        self.dm_tabs.iter().find(|t| t.peer_id == peer_id)
    }

    /// Get DM tab by peer ID (mutable)
    pub fn get_dm_tab_mut(&mut self, peer_id: &str) -> Option<&mut DmTab> {
        self.dm_tabs.iter_mut().find(|t| t.peer_id == peer_id)
    }

    /// Count of active DM tabs
    #[must_use]
    pub fn dm_tab_count(&self) -> usize {
        self.dm_tabs.len()
    }

    /// Get display titles for all DM tabs
    #[must_use]
    pub fn dm_tab_titles(&self) -> Vec<String> {
        self.dm_tabs
            .iter()
            .map(|t| format!("{} (X)", t.short_id()))
            .collect()
    }

    /// Get display titles for all tabs (Chat, Peers, DMs..., Log)
    #[must_use]
    pub fn all_titles(&self) -> Vec<String> {
        let mut titles = vec!["Chat".to_string(), "Peers".to_string()];
        titles.extend(self.dm_tab_titles());
        titles.push("Log".to_string());
        titles
    }

    /// Convert tab index to content type
    #[must_use]
    pub fn tab_index_to_content(&self, tab_idx: usize) -> TabContent {
        let log_index = FIXED_TAB_COUNT + self.dm_tabs.len();
        match tab_idx {
            0 => TabContent::Chat,
            1 => TabContent::Peers,
            idx if idx == log_index => TabContent::Log,
            idx if idx >= FIXED_TAB_COUNT && idx < log_index => {
                let dm_idx = idx - FIXED_TAB_COUNT;
                if let Some(tab) = self.dm_tabs.get(dm_idx) {
                    TabContent::Direct(tab.peer_id.clone())
                } else {
                    TabContent::Chat
                }
            }
            _ => TabContent::Chat,
        }
    }

    /// Total count of tabs including Chat, Peers, DMs, and Log
    #[must_use]
    pub fn total_tab_count(&self) -> usize {
        3 + self.dm_tabs.len()
    }
}

/// Content type for active tab
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabContent {
    /// Broadcast chat view
    Chat,
    /// Peer list view
    Peers,
    /// Direct message view for the given peer ID
    Direct(String),
    /// Debug/log view
    Log,
}

impl TabContent {
    /// Extract peer ID if this is a Direct tab
    #[must_use]
    pub fn peer_id(&self) -> Option<&str> {
        match self {
            TabContent::Direct(id) => Some(id),
            _ => None,
        }
    }

    /// Check if this tab allows text input
    #[must_use]
    pub fn is_input_enabled(&self) -> bool {
        matches!(self, TabContent::Chat | TabContent::Direct(_))
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_tabs.rs"]
mod tests;
