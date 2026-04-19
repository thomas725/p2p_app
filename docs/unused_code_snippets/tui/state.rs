use std::collections::{BTreeMap, VecDeque};
use crate::{
    AppBehaviour, ChatMessageSource, RawChatMessage,
    format_latency, peer_display_name, short_peer_id,
};

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
    pub fn new() -> Self {
        Self::with_messages(crate::TEST_MESSAGES.iter().map(|s| s.to_string()).collect())
    }

    pub fn with_messages(messages: VecDeque<String>) -> Self {
        Self::with_messages_and_width(messages, 80)
    }

    pub fn with_messages_and_width(messages: VecDeque<String>, width: usize) -> Self {
        let chat_message_peers: Vec<String> = messages.iter()
            .map(|m| {
                if m.starts_with("[You]") {
                    "You".to_string()
                } else if m.contains('[') {
                    m.split('[').nth(1).map(|s| s.split(']').next().unwrap_or("").to_string()).unwrap_or_default()
                } else {
                    String::new()
                }
            }).collect();

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
}

// Re-export test messages
pub const TEST_MESSAGES: &[&str] = &[
    "[You] Hello world",
    "[Peer1] How are you?",
    "[You] I'm good, thanks!",
    "[Peer2] Welcome to the chat",
    "[You] Thanks!",
];
