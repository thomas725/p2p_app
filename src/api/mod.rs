//! Flutter Rust Bridge API surface.
//!
//! This module defines the functions and types exposed to Dart via FRB.

use crate::mobile_api::{MobileInitStatus, MobilePeerStatus};
use crate::mobile_node::{ChatMessage, MobilePeerRecord, SwarmEventJson};
use crate::network::NetworkSize;

/// Initialize the mobile database at the given path and return peer info.
///
/// # Errors
/// Returns an error if the database cannot be opened or initialized.
pub fn init_mobile_database(db_path: String) -> Result<MobileInitStatus, String> {
    crate::mobile_api::init_mobile_database(db_path)
}

/// Get current peer status (DB URL, peer ID, nickname).
///
/// # Errors
/// Returns an error if the status cannot be retrieved.
pub fn get_mobile_peer_status() -> Result<MobilePeerStatus, String> {
    crate::mobile_api::get_mobile_peer_status()
}

/// Start the p2p node: init DB, build swarm, begin listening.
///
/// # Errors
/// Returns an error if the node fails to start.
pub fn start_node(db_path: String) -> Result<String, String> {
    crate::mobile_node::start_node(db_path)
}

/// Start the p2p node with automatic DB selection (lock-based, same as TUI).
/// Scans CWD for unlocked .db files, picks the first one, or creates a new one.
///
/// # Errors
/// Returns an error if the node fails to start.
pub fn start_node_auto() -> Result<String, String> {
    crate::mobile_node::start_node_auto()
}

/// Stop the p2p node.
///
/// # Errors
/// Returns an error if the node fails to stop.
pub fn stop_node() -> Result<(), String> {
    crate::mobile_node::stop_node()
}

/// Get all stored TUI log messages (for Log tab display).
#[must_use]
pub fn get_logs() -> Vec<String> {
    crate::logging::get_tui_logs()
}

/// Poll the next swarm event (non-blocking).
///
/// # Errors
/// Returns an error if polling the swarm event fails.
pub fn poll_event() -> Result<Option<SwarmEventJson>, String> {
    crate::mobile_node::poll_event()
}

/// Send a broadcast message.
///
/// # Errors
/// Returns an error if the message fails to send.
pub fn send_broadcast(content: String) -> Result<(), String> {
    crate::mobile_node::send_broadcast(content)
}

/// Send a direct message to a peer.
///
/// # Errors
/// Returns an error if the message fails to send.
pub fn send_dm(peer_id: String, content: String) -> Result<(), String> {
    crate::mobile_node::send_dm(peer_id, content)
}

/// Get all known peers with nicknames.
///
/// This is retained as the home of the generated [`MobilePeerRecord`] Dart
/// type; Flutter's peer table prefers the single round trip
/// [`crate::mobile_api::get_peers_with_stats`].
///
/// # Errors
/// Returns an error if the peers cannot be loaded.
pub fn get_known_peers() -> Result<Vec<MobilePeerRecord>, String> {
    crate::mobile_node::get_known_peers()
}

// --- Message history ---

/// Load broadcast chat messages (chronological order).
///
/// # Errors
/// Returns an error if the messages cannot be loaded.
pub fn load_broadcast_messages(limit: i64) -> Result<Vec<ChatMessage>, String> {
    crate::mobile_node::load_broadcast_messages(limit)
}

/// Load DM history with a specific peer (chronological order).
///
/// # Errors
/// Returns an error if the messages cannot be loaded.
pub fn load_dm_messages(peer_id: String, limit: i64) -> Result<Vec<ChatMessage>, String> {
    crate::mobile_node::load_dm_messages(peer_id, limit)
}

/// Save an outgoing broadcast message to DB and send via swarm.
///
/// # Errors
/// Returns an error if the message cannot be saved or sent.
pub fn save_outgoing_broadcast(content: String) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_broadcast(content)
}

/// Save an outgoing DM to DB and send via swarm.
///
/// # Errors
/// Returns an error if the message cannot be saved or sent.
pub fn save_outgoing_dm(peer_id: String, content: String) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_dm(peer_id, content)
}

/// Save an incoming message (broadcast or DM) to the database.
///
/// # Errors
/// Returns an error if the message cannot be saved.
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
) -> Result<ChatMessage, String> {
    crate::mobile_node::save_incoming_message(content, peer_id, is_direct, nickname)
}

/// Set the local user's nickname.
///
/// # Errors
/// Returns an error if the nickname cannot be persisted.
#[allow(clippy::needless_pass_by_value)]
pub fn set_self_nickname(nickname: String) -> Result<(), String> {
    crate::set_self_nickname(&nickname).map_err(|e| e.to_string())
}

/// Validate a nickname before persisting it.
///
/// Single source of truth shared with the desktop/TUI path
/// ([`crate::nickname::validate_nickname`]). Exposed so the Flutter UI can
/// reject invalid input without re-implementing the rules in Dart.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn validate_nickname(nickname: String) -> bool {
    crate::nickname::validate_nickname(&nickname)
}

/// Human-readable network-size class ("Small" / "Medium" / "Large") for a peer
/// count. Mirrors [`crate::network::NetworkSize`] so the Flutter UI doesn't
/// re-derive the thresholds in Dart.
#[must_use]
pub fn network_size_label(peer_count: i64) -> String {
    // SAFETY: peer counts are small, non-negative values; widening `i64` to `f64`
    // is exact and cannot lose precision for the range used here.
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    NetworkSize::from_peer_count(peer_count as f64).to_string()
}
