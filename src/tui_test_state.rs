//! TUI test state and mouse event handling

use std::collections::{BTreeMap, VecDeque};

pub const TEST_MESSAGES: &[&str] = &[
    "[You] Hello world",
    "[Peer1] How are you?",
    "[You] I'm good, thanks!",
    "[Peer2] Welcome to the chat",
    "[You] Thanks!",
];

/// Test state for TUI rendering tests
#[derive(Clone, Debug)]
pub struct TuiTestState {
    pub messages: VecDeque<String>,
    pub chat_message_peers: Vec<String>,
    pub active_tab: usize,
    pub chat_list_state_offset: usize,
    pub unread_broadcasts: u32,
    pub unread_dms: BTreeMap<String, u32>,
    pub terminal_width: usize,
}

impl TuiTestState {
    /// Create default test state
    pub fn new() -> Self {
        Self::with_messages(TEST_MESSAGES.iter().map(|s| s.to_string()).collect())
    }

    /// Create with custom messages
    pub fn with_messages(messages: VecDeque<String>) -> Self {
        Self::with_messages_and_width(messages, 80)
    }

    /// Create with messages and terminal width
    pub fn with_messages_and_width(messages: VecDeque<String>, width: usize) -> Self {
        let chat_message_peers: Vec<String> = messages
            .iter()
            .map(|m| {
                if m.starts_with("[You]") {
                    "You".to_string()
                } else if m.contains('[') {
                    m.split('[')
                        .nth(1)
                        .map(|s| s.split(']').next().unwrap_or("").to_string())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            })
            .collect();

        Self {
            messages,
            chat_message_peers,
            active_tab: 0,
            chat_list_state_offset: 0,
            unread_broadcasts: 0,
            unread_dms: BTreeMap::new(),
            terminal_width: width,
        }
    }

    /// Handle mouse click event and return clicked peer if applicable
    pub fn handle_mouse_click(&self, row: u16, _col: u16) -> Option<String> {
        let first_msg_row = self.first_message_row();
        if row < first_msg_row {
            return None;
        }

        let content_width = (self.terminal_width as u16).saturating_sub(4);
        let clicked_row_in_list = row - first_msg_row;

        let mut current_row: u16 = 0;
        for msg_idx in self.chat_list_state_offset..self.messages.len() {
            let msg = &self.messages[msg_idx];
            let manual_breaks = msg.matches('\n').count() as u16;
            let wrapped_lines = ((msg.len() as u16) / content_width)
                .saturating_add(1)
                .max(1);
            let msg_lines = manual_breaks + wrapped_lines;

            if clicked_row_in_list >= current_row && clicked_row_in_list < current_row + msg_lines {
                return self.chat_message_peers.get(msg_idx).cloned();
            }

            current_row += msg_lines;
        }

        None
    }

    /// Get row where list header starts (tabs + notifications)
    pub fn list_header_start_row(&self) -> u16 {
        let tabs_rows = 3;
        let notification_rows = if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() {
            1
        } else {
            0
        };
        tabs_rows + notification_rows
    }

    /// Get row where first message appears
    pub fn first_message_row(&self) -> u16 {
        self.list_header_start_row() + 2
    }

    /// Get starting row for message content
    pub fn calculate_content_start_row(&self) -> u16 {
        self.first_message_row()
    }

    /// Handle tab click event
    pub fn handle_tab_click(&mut self, row: u16) {
        self.active_tab = match row {
            0..=2 => (row / 3) as usize,
            _ => self.active_tab,
        };
    }

    /// Handle notification click event
    pub fn handle_notification_click(&self, col: u16) -> Option<NotificationTarget> {
        if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() {
            if col < 20 {
                Some(NotificationTarget::Broadcasts)
            } else {
                self.unread_dms
                    .keys()
                    .next()
                    .cloned()
                    .map(NotificationTarget::Dm)
            }
        } else {
            None
        }
    }
}

impl Default for TuiTestState {
    fn default() -> Self {
        Self::new()
    }
}

impl TuiTestState {
    /// Get tab titles for rendering Tabs widget
    pub fn tab_titles(&self) -> Vec<&str> {
        vec!["Chat", "Peers", "Log"]
    }

    /// Get peer info text
    pub fn peer_info(&self) -> String {
        format!("Peers: {} | Network: test-net", self.messages.len())
    }

    /// Get tab content
    pub fn tab_content(&self) -> crate::tui_tabs::TabContent {
        match self.active_tab {
            0 => crate::tui_tabs::TabContent::Chat,
            1 => crate::tui_tabs::TabContent::Peers,
            _ => crate::tui_tabs::TabContent::Log,
        }
    }

    /// Get formatted messages for chat tab
    pub fn formatted_messages(&self) -> Vec<String> {
        self.messages.iter().cloned().collect()
    }

    /// Get formatted peer list
    pub fn formatted_peers(&self) -> Vec<String> {
        vec![
            "Alice (12D3) - Online".to_string(),
            "Bob (12D4) - Online".to_string(),
        ]
    }

    /// Get formatted DM messages
    pub fn formatted_dm_messages(&self, _peer_id: &str) -> Vec<String> {
        self.messages.iter().cloned().collect()
    }

    /// Get formatted logs
    pub fn formatted_logs(&self) -> Vec<String> {
        vec![
            "[12:00:00] Connected to test-net".to_string(),
            "[12:00:01] Discovered 2 peers".to_string(),
        ]
    }

    /// Get input text
    pub fn input_text(&self) -> &str {
        ""
    }

    /// Get status bar text
    pub fn status_text(&self) -> String {
        "Ready | 0 peers connected".to_string()
    }
}

/// Notification click target
#[derive(Clone, Debug)]
pub enum NotificationTarget {
    Broadcasts,
    Dm(String),
}
