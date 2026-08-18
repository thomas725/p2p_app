//! Flutter Rust Bridge API surface.
//!
//! This module defines the functions and types exposed to Dart via FRB.

use crate::mobile_api::{MobileInitStatus, MobilePeerStatus};
use crate::mobile_node::SwarmEventJson;
use crate::PeerRecord;

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

/// Stop the p2p node.
pub fn stop_node() -> Result<(), String> {
    crate::mobile_node::stop_node()
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

/// Get all known peers.
pub fn get_known_peers() -> Result<Vec<PeerRecord>, String> {
    crate::mobile_node::get_known_peers()
}
