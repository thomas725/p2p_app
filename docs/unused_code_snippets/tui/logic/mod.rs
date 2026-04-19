//! State management for the P2P chat TUI application.

use crate::tui::shared::{MAX_LOGS, MAX_MESSAGES, TuiState};
use crate::tui::tab::{Tab, TabContent};
use std::collections::{BTreeMap, VecDeque};

/// Initializes a new TUI state with default values
pub fn new_state() -> TuiState {
    TuiState::default()
}

/// Initializes a new TUI state with custom messages
pub fn new_state_with_messages(messages: VecDeque<String>) -> TuiState {
    let mut state = TuiState::default();
    state.messages = messages;
    state.chat_message_peers = messages
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
    state
}

/// Loads messages from database
pub fn load_messages(topic: &str, limit: usize) -> Result<VecDeque<String>, String> {
    // Placeholder for actual database loading
    // In real implementation, this would query SQLite
    Ok(VecDeque::new())
}

/// Loads peers from database
pub fn load_peers() -> Result<Vec<(String, String, String)>, String> {
    // Placeholder for actual database loading
    // In real implementation, this would query SQLite
    Ok(Vec::new())
}

/// Saves a message to the database
pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
) -> Result<(), String> {
    // Placeholder for actual database saving
    Ok(())
}

/// Marks a message as sent
pub fn mark_message_sent(message_id: i32) -> Result<(), String> {
    // Placeholder for actual database update
    Ok(())
}

/// Saves a peer to the database
pub fn save_peer(peer_id: &str, addresses: &[String]) -> Result<(), String> {
    // Placeholder for actual database saving
    Ok(())
}

/// Saves the current session
pub fn save_session(concurrent_peers: usize) -> Result<(), String> {
    // Placeholder for actual session saving
    Ok(())
}

/// Gets the current network size classification
pub fn get_network_size() -> Result<crate::network::NetworkSize, String> {
    // Placeholder for actual network size calculation
    Ok(crate::network::NetworkSize::Medium)
}

/// Handles user input for chat
pub fn handle_chat_input(input: &str, state: &mut TuiState, logs: &mut VecDeque<String>) {
    if input.starts_with('/') {
        handle_command(input, state, logs);
    } else {
        // Handle regular message
        let message = crate::ChatMessage {
            content: input.to_string(),
            peer_id: None,
            nickname: Some(state.own_nickname.clone()),
            timestamp: format_current_time(),
        };
        state.messages.push_back(message);
    }
}

fn handle_command(input: &str, state: &mut TuiState, logs: &mut VecDeque<String>) {
    let parts: Vec<&str> = input.splitn(2, ' ').collect();
    match parts[0] {
        "/nick" => {
            if let Some(new_nick) = parts.get(1) {
                let new_nick = new_nick.trim();
                if !new_nick.is_empty() {
                    state.own_nickname = new_nick.to_string();
                    add_log(logs, &format!("Nickname set to: {}", new_nick));
                }
            }
        }
        "/help" | "/h" => {
            add_log(logs, "Available commands:");
            add_log(logs, "  /nick [name] - Set your nickname");
            add_log(logs, "  /help - Show this help");
        }
        _ => add_log(logs, &format!("Unknown command: {}", parts[0])),
    }
}

fn add_log(logs: &mut VecDeque<String>, message: &str) {
    let timestamp = format_current_time();
    logs.push_back(format!("[{}] {}", timestamp, message));
    if logs.len() > MAX_LOGS {
        logs.pop_front();
    }
}

fn format_current_time() -> String {
    // Placeholder for time formatting
    "Now".to_string()
}

/// Handles user input for direct messages
pub fn handle_dm_input(peer_id: &str, input: &str, state: &mut TuiState) {
    let message = crate::ChatMessage {
        content: input.to_string(),
        peer_id: Some(peer_id.to_string()),
        nickname: Some(state.own_nickname.clone()),
        timestamp: format_current_time(),
    };

    state
        .dm_messages
        .entry(peer_id.to_string())
        .or_insert_with(VecDeque::new)
        .push_back(message);
}

/// Gets messages for a specific DM conversation
pub fn get_dm_messages(peer_id: &str, state: &TuiState) -> Vec<&crate::ChatMessage> {
    state
        .dm_messages
        .get(peer_id)
        .map(|messages| messages.iter().collect())
        .unwrap_or_default()
}
