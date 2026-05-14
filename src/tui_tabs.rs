//! TUI tab management and navigation

use std::collections::VecDeque;

/// Tab identifier enumeration
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum TabId {
    /// Broadcast chat tab (default)
    #[default]
    Chat,
    /// Peer list tab
    Peers,
    /// Direct message tab
    Direct,
    /// Debug/log tab
    Log,
}

impl TabId {
    /// Convert to numeric index
    #[must_use]
    pub fn index(&self) -> usize {
        match self {
            TabId::Chat => 0,
            TabId::Peers => 1,
            TabId::Direct => 2,
            TabId::Log => 3,
        }
    }

    /// Convert from numeric index
    #[must_use]
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

    /// Create DM tab with initial messages
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
mod tests {
    use super::*;

    #[test]
    fn test_tab_id_index() {
        assert_eq!(TabId::Chat.index(), 0);
        assert_eq!(TabId::Peers.index(), 1);
        assert_eq!(TabId::Direct.index(), 2);
        assert_eq!(TabId::Log.index(), 3);
    }

    #[test]
    fn test_tab_id_from_index() {
        assert_eq!(TabId::from_index(0), TabId::Chat);
        assert_eq!(TabId::from_index(1), TabId::Peers);
        assert_eq!(TabId::from_index(2), TabId::Direct);
        assert_eq!(TabId::from_index(3), TabId::Log);
        assert_eq!(TabId::from_index(99), TabId::Chat);
    }

    #[test]
    fn test_dm_tab_new() {
        let dm = DmTab::new("12D3KooWABCDEFGH".to_string());
        assert_eq!(dm.peer_id, "12D3KooWABCDEFGH");
        assert!(dm.messages.is_empty());
    }

    #[test]
    fn test_dm_tab_with_messages() {
        let msgs = VecDeque::from(vec!["msg1".to_string(), "msg2".to_string()]);
        let dm = DmTab::with_messages("peer1".to_string(), msgs.clone());
        assert_eq!(dm.peer_id, "peer1");
        assert_eq!(dm.messages.len(), 2);
    }

    #[test]
    fn test_dm_tab_short_id() {
        let dm = DmTab::new("12D3KooWABCDEFGH".to_string());
        assert_eq!(dm.short_id(), "ABCDEFGH");
    }

    #[test]
    fn test_dm_tab_short_id_short_peer() {
        let dm = DmTab::new("short".to_string());
        assert_eq!(dm.short_id(), "short");
    }

    #[test]
    fn test_dynamic_tabs_new() {
        let tabs = DynamicTabs::new();
        assert_eq!(tabs.dm_tab_count(), 0);
        assert_eq!(tabs.total_tab_count(), 3);
    }

    #[test]
    fn test_dynamic_tabs_add_dm_tab() {
        let mut tabs = DynamicTabs::new();
        let idx = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);
    }

    #[test]
    fn test_dynamic_tabs_remove_dm_tab() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let idx = tabs.remove_dm_tab("peer1");
        assert_eq!(idx, Some(2));
        assert_eq!(tabs.dm_tab_count(), 0);
    }

    #[test]
    fn test_dynamic_tabs_get_dm_tab() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let dm = tabs.get_dm_tab("peer1");
        assert!(dm.is_some());
        assert_eq!(dm.unwrap().peer_id, "peer1");
    }

    #[test]
    fn test_dynamic_tabs_dm_tab_titles() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        tabs.add_dm_tab("peer2".to_string());
        let titles = tabs.dm_tab_titles();
        assert_eq!(titles.len(), 2);
    }

    #[test]
    fn test_tab_content_peer_id() {
        assert_eq!(
            TabContent::Direct("peer1".to_string()).peer_id(),
            Some("peer1")
        );
        assert_eq!(TabContent::Chat.peer_id(), None);
        assert_eq!(TabContent::Peers.peer_id(), None);
        assert_eq!(TabContent::Log.peer_id(), None);
    }

    #[test]
    fn test_tab_content_is_input_enabled() {
        assert!(TabContent::Chat.is_input_enabled());
        assert!(TabContent::Direct("peer1".to_string()).is_input_enabled());
        assert!(!TabContent::Peers.is_input_enabled());
        assert!(!TabContent::Log.is_input_enabled());
    }

    #[test]
    fn test_dynamic_tabs_get_dm_tab_mut() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let dm = tabs.get_dm_tab_mut("peer1");
        assert!(dm.is_some());
        dm.unwrap().messages.push_back("Hello".to_string());
        let dm2 = tabs.get_dm_tab("peer1");
        assert_eq!(dm2.unwrap().messages.len(), 1);
    }

    #[test]
    fn test_dynamic_tabs_total_tab_count() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.total_tab_count(), 3);
        tabs.add_dm_tab("peer1".to_string());
        assert_eq!(tabs.total_tab_count(), 4);
    }

    #[test]
    fn test_dynamic_tabs_tab_index_to_content() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.tab_index_to_content(0), TabContent::Chat);
        assert_eq!(tabs.tab_index_to_content(1), TabContent::Peers);
        tabs.add_dm_tab("peer1".to_string());
        assert_eq!(
            tabs.tab_index_to_content(2),
            TabContent::Direct("peer1".to_string())
        );
        assert_eq!(tabs.tab_index_to_content(3), TabContent::Log);
    }

    #[test]
    fn test_remove_dm_tab_nonexistent() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.remove_dm_tab("nobody"), None);
    }

    #[test]
    fn test_get_dm_tab_nonexistent() {
        let tabs = DynamicTabs::new();
        assert_eq!(tabs.get_dm_tab("nobody"), None);
    }

    #[test]
    fn test_get_dm_tab_mut_nonexistent() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.get_dm_tab_mut("nobody"), None);
    }

    #[test]
    fn test_add_dm_tab_existing_peer() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let idx = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);
    }

    #[test]
    fn test_tab_index_to_content_multiple_dms() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        tabs.add_dm_tab("peer2".to_string());
        assert_eq!(
            tabs.tab_index_to_content(2),
            TabContent::Direct("peer1".to_string())
        );
        assert_eq!(
            tabs.tab_index_to_content(3),
            TabContent::Direct("peer2".to_string())
        );
        assert_eq!(tabs.tab_index_to_content(4), TabContent::Log);
    }

    #[test]
    fn test_tab_index_to_content_out_of_bounds_dm() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        // Index 3 is Log (2 + 1 DM = 3), index 4+ should be Chat
        assert_eq!(tabs.tab_index_to_content(5), TabContent::Chat);
    }

    #[test]
    fn test_all_titles_with_dms() {
        let mut tabs = DynamicTabs::new();
        let titles = tabs.all_titles();
        assert_eq!(titles, vec!["Chat", "Peers", "Log"]);
        tabs.add_dm_tab("peer1".to_string());
        let titles = tabs.all_titles();
        assert_eq!(titles.len(), 4);
        assert_eq!(titles[0], "Chat");
        assert_eq!(titles[1], "Peers");
        assert!(titles[2].contains("peer1"));
        assert_eq!(titles[3], "Log");
    }

    #[test]
    fn test_dynamic_tabs_default_is_empty() {
        let tabs = DynamicTabs::default();
        assert_eq!(tabs.dm_tab_count(), 0);
        assert_eq!(tabs.total_tab_count(), 3);
    }
}
