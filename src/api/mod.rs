//! Flutter Rust Bridge API surface.
//!
//! This module defines the functions and types exposed to Dart via FRB.

use crate::mobile_api::{MobileInitStatus, MobilePeerStatus};
use crate::mobile_node::{ChatMessage, MobilePeerRecord, SwarmEventJson};
use crate::network::NetworkSize;

/// Initialize the mobile database at the given path and return peer info.
pub fn init_mobile_database(db_path: String) -> Result<MobileInitStatus, String> {
    crate::mobile_api::init_mobile_database(db_path)
}

/// Get current peer status (DB URL, peer ID, nickname).
pub fn get_mobile_peer_status() -> Result<MobilePeerStatus, String> {
    crate::mobile_api::get_mobile_peer_status()
}

/// Start the p2p node: init DB, build swarm, begin listening.
pub fn start_node(db_path: String) -> Result<String, String> {
    crate::mobile_node::start_node(db_path)
}

/// Start the p2p node with automatic DB selection (lock-based, same as TUI).
/// Scans CWD for unlocked .db files, picks the first one, or creates a new one.
pub fn start_node_auto() -> Result<String, String> {
    crate::mobile_node::start_node_auto()
}

/// Stop the p2p node.
pub fn stop_node() -> Result<(), String> {
    crate::mobile_node::stop_node()
}

/// Get all stored TUI log messages (for Log tab display).
pub fn get_logs() -> Vec<String> {
    crate::logging::get_tui_logs()
}

/// Poll the next swarm event (non-blocking).
pub fn poll_event() -> Result<Option<SwarmEventJson>, String> {
    crate::mobile_node::poll_event()
}

/// Send a broadcast message.
pub fn send_broadcast(content: String) -> Result<(), String> {
    crate::mobile_node::send_broadcast(content)
}

/// Send a direct message to a peer.
pub fn send_dm(peer_id: String, content: String) -> Result<(), String> {
    crate::mobile_node::send_dm(peer_id, content)
}

/// Get all known peers with nicknames.
pub fn get_known_peers() -> Result<Vec<MobilePeerRecord>, String> {
    crate::mobile_node::get_known_peers()
}

// --- Message history ---

/// Load broadcast chat messages (chronological order).
pub fn load_broadcast_messages(limit: i64) -> Result<Vec<ChatMessage>, String> {
    crate::mobile_node::load_broadcast_messages(limit)
}

/// Load DM history with a specific peer (chronological order).
pub fn load_dm_messages(peer_id: String, limit: i64) -> Result<Vec<ChatMessage>, String> {
    crate::mobile_node::load_dm_messages(peer_id, limit)
}

/// Save an outgoing broadcast message to DB and send via swarm.
pub fn save_outgoing_broadcast(content: String) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_broadcast(content)
}

/// Save an outgoing DM to DB and send via swarm.
pub fn save_outgoing_dm(peer_id: String, content: String) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_dm(peer_id, content)
}

/// Save an incoming message (broadcast or DM) to the database.
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
) -> Result<ChatMessage, String> {
    crate::mobile_node::save_incoming_message(content, peer_id, is_direct, nickname)
}

/// Set the local user's nickname.
pub fn set_self_nickname(nickname: String) -> Result<(), String> {
    crate::set_self_nickname(&nickname).map_err(|e| e.to_string())
}

/// Validate a nickname before persisting it.
///
/// Single source of truth shared with the desktop/TUI path
/// ([`crate::nickname::validate_nickname`]). Exposed so the Flutter UI can
/// reject invalid input without re-implementing the rules in Dart.
pub fn validate_nickname(nickname: String) -> bool {
    crate::nickname::validate_nickname(&nickname)
}

/// Human-readable network-size class ("Small" / "Medium" / "Large") for a peer
/// count. Mirrors [`crate::network::NetworkSize`] so the Flutter UI doesn't
/// re-derive the thresholds in Dart.
pub fn network_size_label(peer_count: i64) -> String {
    NetworkSize::from_peer_count(peer_count as f64).to_string()
}
