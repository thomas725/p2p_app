//! TUI tab management and navigation

/// Flutter-style peer label: nickname (or generated petname) followed by the
/// first 3 characters of the short peer id, e.g. `"Alice (Ab3)"`. Falls back to
/// the short id when the database lookup fails (e.g. in tests).
fn peer_display_label(peer_id: &str) -> String {
    crate::get_peer_display_name(peer_id).unwrap_or_else(|_| crate::fmt::short_peer_id(peer_id))
}

/// Number of fixed tabs before dynamic tabs (Chat, Peers, Groups)
pub(crate) const FIXED_TAB_COUNT: usize = 3;

/// Fixed tabs that always appear after any dynamic tabs (Log, Settings)
const SUFFIX_TAB_COUNT: usize = 2;
const LOG_TITLE: &str = "Log";
const SETTINGS_TITLE: &str = "Settings";
const GROUPS_TITLE: &str = "Groups";

/// Group chat tab: a marker for an open group conversation.
///
/// The conversation history lives in the app state's per-group message map;
/// this type only tracks which group chats are open and how to label them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupTab {
    /// The stable group ID this chat tab is associated with
    pub group_id: String,
    /// Human-readable group name shown in the tab title
    pub display_name: String,
}

impl GroupTab {
    /// Create a new group chat tab for a group
    #[must_use]
    pub const fn new(group_id: String, display_name: String) -> Self {
        Self {
            group_id,
            display_name,
        }
    }
}

/// Direct message tab: a marker for an open DM conversation with a peer.
///
/// The actual conversation history lives in the app state's per-peer message
/// map; this type only tracks which DM tabs are open and how to label them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DmTab {
    /// The full peer ID this DM tab is associated with
    pub peer_id: String,
}

impl DmTab {
    /// Create a new DM tab for a peer
    #[must_use]
    pub const fn new(peer_id: String) -> Self {
        Self { peer_id }
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
    /// Active group-chat tabs, one per open conversation
    pub group_tabs: Vec<GroupTab>,
    /// Active DM tabs, one per open conversation
    pub dm_tabs: Vec<DmTab>,
    /// Active peer-info tabs, one per inspected peer
    pub peer_info_tabs: Vec<String>,
}

impl DynamicTabs {
    /// Create new empty dynamic tabs
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or retrieve index of a group-chat tab for a group.
    ///
    /// Deduplicates on `group_id` (a group has a single chat window, regardless
    /// of separate creaters using different spelling of its name).
    pub fn add_group_tab(&mut self, group_id: String, display_name: String) -> usize {
        if let Some(pos) = self.group_tabs.iter().position(|t| t.group_id == group_id) {
            return pos.saturating_add(FIXED_TAB_COUNT);
        }
        let idx = self.group_tabs.len().saturating_add(FIXED_TAB_COUNT);
        self.group_tabs.push(GroupTab::new(group_id, display_name));
        idx
    }

    /// Remove a group-chat tab, return its previous index
    pub fn remove_group_tab(&mut self, group_id: &str) -> Option<usize> {
        if let Some(pos) = self.group_tabs.iter().position(|t| t.group_id == group_id) {
            self.group_tabs.remove(pos);
            return Some(pos.saturating_add(FIXED_TAB_COUNT));
        }
        None
    }

    /// Get a group-chat tab by group ID (read-only)
    #[must_use]
    pub fn get_group_tab(&self, group_id: &str) -> Option<&GroupTab> {
        self.group_tabs.iter().find(|t| t.group_id == group_id)
    }

    /// Count of active group-chat tabs
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn group_tab_count(&self) -> usize {
        self.group_tabs.len()
    }

    /// Get display titles for all group-chat tabs
    #[must_use]
    pub fn group_tab_titles(&self) -> Vec<String> {
        self.group_tabs
            .iter()
            .map(|t| format!("Group: {} [X]", t.display_name))
            .collect()
    }

    /// Add or retrieve index of DM tab for peer
    pub fn add_dm_tab(&mut self, peer_id: String) -> usize {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            return pos
                .saturating_add(FIXED_TAB_COUNT)
                .saturating_add(self.group_tabs.len());
        }
        let idx = self
            .dm_tabs
            .len()
            .saturating_add(FIXED_TAB_COUNT)
            .saturating_add(self.group_tabs.len());
        self.dm_tabs.push(DmTab::new(peer_id));
        idx
    }

    /// Remove DM tab for peer, return its previous index
    pub fn remove_dm_tab(&mut self, peer_id: &str) -> Option<usize> {
        if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
            self.dm_tabs.remove(pos);
            return Some(
                pos.saturating_add(FIXED_TAB_COUNT)
                    .saturating_add(self.group_tabs.len()),
            );
        }
        None
    }

    /// Get DM tab by peer ID (read-only)
    #[must_use]
    pub fn get_dm_tab(&self, peer_id: &str) -> Option<&DmTab> {
        self.dm_tabs.iter().find(|t| t.peer_id == peer_id)
    }

    /// Count of active DM tabs
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn dm_tab_count(&self) -> usize {
        self.dm_tabs.len()
    }

    /// Get display titles for all DM tabs
    #[must_use]
    pub fn dm_tab_titles(&self) -> Vec<String> {
        self.dm_tabs
            .iter()
            .map(|t| format!("{} [X]", peer_display_label(&t.peer_id)))
            .collect()
    }

    /// Add or retrieve index of a peer-info tab for peer
    pub fn add_peer_info_tab(&mut self, peer_id: String) -> usize {
        let prefix = FIXED_TAB_COUNT
            .saturating_add(self.group_tabs.len())
            .saturating_add(self.dm_tabs.len());
        if let Some(pos) = self.peer_info_tabs.iter().position(|p| p == &peer_id) {
            return pos.saturating_add(prefix);
        }
        let idx = self.peer_info_tabs.len().saturating_add(prefix);
        self.peer_info_tabs.push(peer_id);
        idx
    }

    /// Remove peer-info tab for peer, return its previous index
    pub fn remove_peer_info_tab(&mut self, peer_id: &str) -> Option<usize> {
        let prefix = FIXED_TAB_COUNT
            .saturating_add(self.group_tabs.len())
            .saturating_add(self.dm_tabs.len());
        if let Some(pos) = self.peer_info_tabs.iter().position(|p| p == peer_id) {
            self.peer_info_tabs.remove(pos);
            return Some(pos.saturating_add(prefix));
        }
        None
    }

    /// Remove the tab matching `content` (`Direct`, `PeerInfo`, or `GroupChat`),
    /// returning its previous index. Other tab kinds are never removable.
    pub fn remove_tab(&mut self, content: &TabContent) -> Option<usize> {
        match content {
            TabContent::Direct(peer_id) => self.remove_dm_tab(peer_id),
            TabContent::PeerInfo(peer_id) => self.remove_peer_info_tab(peer_id),
            TabContent::GroupChat(group_id) => self.remove_group_tab(group_id),
            _ => None,
        }
    }

    /// Get peer-info tab by peer ID (read-only)
    #[must_use]
    pub fn get_peer_info_tab(&self, peer_id: &str) -> Option<&String> {
        self.peer_info_tabs.iter().find(|p| p.as_str() == peer_id)
    }

    /// Count of active peer-info tabs
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn peer_info_tab_count(&self) -> usize {
        self.peer_info_tabs.len()
    }

    /// Get display titles for all peer-info tabs
    #[must_use]
    pub fn peer_info_tab_titles(&self) -> Vec<String> {
        self.peer_info_tabs
            .iter()
            .map(|p| format!("Info: {} [X]", peer_display_label(p)))
            .collect()
    }

    /// Get display titles for all tabs (Chat, Peers, Groups, Group chats,
    /// DMs..., Info..., Log, Settings)
    #[must_use]
    pub fn all_titles(&self) -> Vec<String> {
        let mut titles = vec![
            "Chat".to_string(),
            "Peers".to_string(),
            GROUPS_TITLE.to_string(),
        ];
        titles.extend(self.group_tab_titles());
        titles.extend(self.dm_tab_titles());
        titles.extend(self.peer_info_tab_titles());
        titles.push(LOG_TITLE.to_string());
        titles.push(SETTINGS_TITLE.to_string());
        titles
    }

    /// Convert tab index to content type
    #[must_use]
    pub fn tab_index_to_content(&self, tab_idx: usize) -> TabContent {
        let group_count = self.group_tabs.len();
        let dm_count = self.dm_tabs.len();
        let info_count = self.peer_info_tabs.len();
        let log_index = FIXED_TAB_COUNT
            .saturating_add(group_count)
            .saturating_add(dm_count)
            .saturating_add(info_count);
        let settings_index = log_index.saturating_add(1);
        let dm_start = FIXED_TAB_COUNT.saturating_add(group_count);
        let info_start = dm_start.saturating_add(dm_count);
        match tab_idx {
            0 => TabContent::Chat,
            1 => TabContent::Peers,
            2 => TabContent::Groups,
            idx if idx == log_index => TabContent::Log,
            idx if idx == settings_index => TabContent::Settings,
            idx if idx >= FIXED_TAB_COUNT && idx < dm_start => {
                let group_idx = idx.saturating_sub(FIXED_TAB_COUNT);
                self.group_tabs
                    .get(group_idx)
                    .map_or(TabContent::Chat, |t| {
                        TabContent::GroupChat(t.group_id.clone())
                    })
            }
            idx if idx >= dm_start && idx < info_start => {
                let dm_idx = idx.saturating_sub(dm_start);
                self.dm_tabs.get(dm_idx).map_or(TabContent::Chat, |tab| {
                    TabContent::Direct(tab.peer_id.clone())
                })
            }
            idx if idx >= info_start && idx < log_index => {
                let info_idx = idx.saturating_sub(info_start);
                self.peer_info_tabs
                    .get(info_idx)
                    .map_or(TabContent::Chat, |p| TabContent::PeerInfo(p.clone()))
            }
            _ => TabContent::Chat,
        }
    }

    /// Total count of tabs including Chat, Peers, Groups, group chats, DMs,
    /// Info, Log, and Settings
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn total_tab_count(&self) -> usize {
        self.group_tabs
            .len()
            .saturating_add(self.dm_tabs.len())
            .saturating_add(self.peer_info_tabs.len())
            .saturating_add(FIXED_TAB_COUNT)
            .saturating_add(SUFFIX_TAB_COUNT)
    }
}

/// Content type for active tab
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TabContent {
    /// Broadcast chat view
    Chat,
    /// Peer list view
    Peers,
    /// Group list and create/join view
    Groups,
    /// Group chat view for the given group ID
    GroupChat(String),
    /// Direct message view for the given peer ID
    Direct(String),
    /// Debug/log view
    Log,
    /// Settings view
    Settings,
    /// Peer info view for the given peer ID
    PeerInfo(String),
}

impl TabContent {
    /// Extract peer ID if this is a Direct or `PeerInfo` tab
    #[must_use]
    pub fn peer_id(&self) -> Option<&str> {
        match self {
            Self::Direct(id) | Self::PeerInfo(id) => Some(id),
            _ => None,
        }
    }

    /// Check if this tab allows text input
    #[must_use]
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_input_enabled(&self) -> bool {
        matches!(self, Self::Chat | Self::Direct(_) | Self::GroupChat(_))
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_tui_tabs.rs"]
mod tests;
