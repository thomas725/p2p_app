//! Flutter Rust Bridge API surface.
//!
//! This module defines the functions and types exposed to Dart via FRB.

use crate::mobile_api::{
    MobileContactInfo, MobileContactRow, MobileDeviceInfo, MobileInitStatus, MobilePeerStatus,
};
use crate::mobile_node::{
    ChatMessage, MobileGroup, MobileGroupMessage, MobilePeerRecord, SwarmEventJson,
};
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

/// Clear the in-memory TUI log buffer (Log tab "Clear" action).
pub fn clear_logs() {
    crate::logging::clear_tui_logs();
}

/// Poll the next swarm event (non-blocking).
///
/// # Errors
/// Returns an error if polling the swarm event fails.
pub fn poll_event() -> Result<Option<SwarmEventJson>, String> {
    crate::mobile_node::poll_event()
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

// --- Groups ---

/// Create (or join) a public group by name and subscribe to its topic.
///
/// # Errors
/// Returns an error if the group cannot be created.
pub fn create_group(name: &str) -> Result<MobileGroup, String> {
    crate::mobile_node::create_group(name)
}

/// List all known groups with their member counts.
///
/// # Errors
/// Returns an error if the groups cannot be loaded.
pub fn list_groups() -> Result<Vec<MobileGroup>, String> {
    crate::mobile_node::list_groups()
}

/// Load a group's message history (chronological order).
///
/// # Errors
/// Returns an error if the messages cannot be loaded.
pub fn load_group_messages(
    group_id: String,
    limit: i64,
) -> Result<Vec<MobileGroupMessage>, String> {
    crate::mobile_node::load_group_messages(group_id, limit)
}

/// Save an outgoing group message to DB and publish via swarm.
///
/// # Errors
/// Returns an error if the message cannot be saved or sent.
pub fn save_outgoing_group(
    group_id: String,
    content: String,
) -> Result<MobileGroupMessage, String> {
    crate::mobile_node::save_outgoing_group(group_id, content)
}

/// Accept a pending group invite: materialize the group locally, persist
/// membership, and subscribe to the group's topic so the swarm starts receiving
/// its traffic.
///
/// # Errors
/// Returns an error if the group row cannot be materialized/loaded.
pub fn accept_group_invite(group_id: String) -> Result<MobileGroup, String> {
    crate::mobile_node::accept_group_invite(group_id)
}

/// Set the local user's nickname.
///
/// # Errors
/// Returns an error if the nickname cannot be persisted.
pub fn set_self_nickname(nickname: &str) -> Result<(), String> {
    crate::set_self_nickname(nickname).map_err(|e| e.to_string())
}

/// Validate a nickname before persisting it.
///
/// Single source of truth shared with the desktop/TUI path
/// ([`crate::nickname::validate_nickname`]). Exposed so the Flutter UI can
/// reject invalid input without re-implementing the rules in Dart.
#[must_use]
pub fn validate_nickname(nickname: &str) -> bool {
    crate::nickname::validate_nickname(nickname)
}

/// Human-readable network-size class ("Small" / "Medium" / "Large") for a peer
/// count. Mirrors [`crate::network::NetworkSize`] so the Flutter UI doesn't
/// re-derive the thresholds in Dart.
#[must_use]
pub fn network_size_label(peer_count: i64) -> String {
    match peer_count {
        0..=3 => NetworkSize::Small,
        4..=15 => NetworkSize::Medium,
        _ => NetworkSize::Large,
    }
    .to_string()
}

// --- Identity lane ---

/// Resolve this install's user ID from its stored identity and binding.
///
/// Returns `None` until this install's peer ID is bound into some user's
/// device-set ([`crate::identity::DeviceBinding`]).
///
/// # Errors
/// Returns an error if the local peer ID cannot be resolved or the DB lookup
/// fails.
pub fn get_my_user_id() -> Result<Option<String>, String> {
    crate::mobile_api::get_my_user_id()
}

/// Provision this install as a user (account keypair + self-binding claim).
/// The returned string is the new root `user_id`.
///
/// # Errors
/// Returns an error if provisioning fails.
pub fn provision_my_user(
    self_username: Option<String>,
    self_nickname: Option<String>,
) -> Result<String, String> {
    crate::mobile_api::provision_my_user(self_username, self_nickname)
}

/// Send a device-binding `intro` request to a peer over the swarm.
///
/// # Errors
/// Returns an error if the request cannot be queued.
pub fn send_device_binding_intro(peer_id: String) -> Result<(), String> {
    crate::mobile_node::send_device_binding_intro(peer_id)
}

/// Mint a signed claim binding `device_pubkey` at `peer_id` into this
/// install's user, persist the local binding, and send the claim to the device.
///
/// # Errors
/// Returns an error if this install has no user, the claim cannot be minted,
/// or the request cannot be queued.
#[allow(clippy::needless_pass_by_value)]
pub fn accept_device_binding(peer_id: String, device_pubkey: Vec<u8>) -> Result<(), String> {
    // FFI boundary: FRB's Dart `List<int>` only deserializes into an owned
    // `Vec<u8>`; `&[u8]` is unsupported by codegen 2.13.0.
    crate::mobile_node::accept_device_binding(peer_id, &device_pubkey)
}

/// Verify a raw device-binding claim against the user's stored account key.
///
/// The claim is the raw signature over `user_id ‖ device_pubkey ‖ claim_id`;
/// all three message parts are required to recompute and verify it.
///
/// # Errors
/// Returns an error if the DB lookup fails.
pub fn verify_device_binding_claim(
    user_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
) -> Result<bool, String> {
    crate::mobile_api::verify_device_binding_claim(user_id, device_pubkey, claim_id, claim)
}

/// Verify a claim against the user's account key, then bind the device.
///
/// # Errors
/// Returns an error if the claim fails verification or the insert fails.
pub fn add_device(
    user_id: String,
    peer_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
) -> Result<MobileDeviceInfo, String> {
    crate::mobile_api::add_device(user_id, peer_id, device_pubkey, claim_id, claim)
}

/// List the live devices bound to a user, most recent first.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn get_user_devices(user_id: String) -> Result<Vec<MobileDeviceInfo>, String> {
    crate::mobile_api::get_user_devices(user_id)
}

/// List this install's user's contacts.
///
/// Returns an empty list before the install is bound to a user.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn get_contacts() -> Result<Vec<MobileContactInfo>, String> {
    crate::mobile_api::get_contacts()
}

/// Add a contact for this install's user.
///
/// # Errors
/// Returns an error if the install has no bound user or the insert fails.
pub fn add_contact(
    contact_user_id: String,
    nickname_override: Option<String>,
) -> Result<(), String> {
    crate::mobile_api::add_contact(contact_user_id, nickname_override)
}

/// Remove a contact for this install's user.
///
/// # Errors
/// Returns an error if the install has no bound user or the delete fails.
pub fn remove_contact(contact_user_id: String) -> Result<(), String> {
    crate::mobile_api::remove_contact(contact_user_id)
}

/// Load the canonical Contacts-tab roster in a single round trip.
///
/// # Errors
/// Returns an error if the roster cannot be loaded.
pub fn get_contacts_with_stats() -> Result<Vec<MobileContactRow>, String> {
    crate::mobile_api::get_contacts_with_stats()
}
