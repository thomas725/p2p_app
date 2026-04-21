//! TUI tab management and navigation

use std::collections::VecDeque;

/// Tab identifier enumeration
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TabId {
    #[default]
    Chat,
    Peers,
    Direct,
    Log,
}

impl TabId {
    /// Convert to numeric index
    pub fn index(&self) -> usize {
        match self {
            TabId::Chat => 0,
            TabId::Peers => 1,
            TabId::Direct => 2,
            TabId::Log => 3,
        }
    }

    /// Convert from numeric index
    pub fn from_index(idx: usize) -> Self {
        match idx {
            0 => TabId::Chat,
            1 => TabId::Peers,
            2 => TabId::Direct,
            3 => TabId::Log,
            _ => TabId::Chat,
        }
    }
}

/// Direct message tab with peer ID and message history
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmTab {
    pub peer_id: String,
    pub messages: VecDeque<String>,
}

impl DmTab {
    /// Create a new DM tab for a peer
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            messages: VecDeque::new(),
        }
    }

    /// Create DM tab with initial messages
    pub fn with_messages(peer_id: String, messages: VecDeque<String>) -> Self {
        Self { peer_id, messages }
    }

    /// Get last 8 characters of peer ID for display
    #[must_use]
    pub fn short_id(&self) -> String {
        self.peer_id
            .chars()
            .rev()
            .take(8)
            .collect::<String>()
            .chars()
            .rev()
            .collect()
    }
}

/// Dynamic tab management for direct message conversations
#[derive(Clone, Debug, Default)]
pub struct DynamicTabs {
    pub dm_tabs: Vec<DmTab>,
}

impl DynamicTabs {
    /// Create new empty dynamic tabs
    pub fn new() -> Self {
        Self {
            dm_tabs: Vec::new(),
        }
    }

    /// Add or retrieve index of DM tab for peer
    pub fn add_dm_tab(&mut self, peer_id: String) -> usize {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            return pos + 2;
        }
        self.dm_tabs.push(DmTab::new(peer_id));
        self.dm_tabs.len() + 1
    }

    /// Remove DM tab for peer, return its previous index
    pub fn remove_dm_tab(&mut self, peer_id: &str) -> Option<usize> {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            self.dm_tabs.remove(pos);
            return Some(pos + 2);
        }
        None
    }

    /// Get DM tab by peer ID (read-only)
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
    pub fn dm_tab_titles(&self) -> Vec<String> {
        self.dm_tabs
            .iter()
            .map(|t| format!("{} (X)", t.short_id()))
            .collect()
    }

    /// Get display titles for all tabs (Chat, Peers, DMs..., Log)
    pub fn all_titles(&self) -> Vec<String> {
        let mut titles = vec!["Chat".to_string(), "Peers".to_string()];
        titles.extend(self.dm_tab_titles());
        titles.push("Log".to_string());
        titles
    }

    /// Convert tab index to content type
    pub fn tab_index_to_content(&self, tab_idx: usize) -> TabContent {
        let log_index = 2 + self.dm_tabs.len();
        match tab_idx {
            0 => TabContent::Chat,
            1 => TabContent::Peers,
            idx if idx == log_index => TabContent::Log,
            idx if idx >= 2 && idx < log_index => {
                let dm_idx = idx - 2;
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
    Chat,
    Peers,
    Direct(String),
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
