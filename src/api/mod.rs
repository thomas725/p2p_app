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

/// Live network diagnostics: interfaces (name/MAC/IPv4/IPv6), reachability,
/// relay reservations, and DHT connectivity/announcement state.
///
/// Both frontends poll this single snapshot; see [`crate::net_diag`].
#[must_use]
pub fn get_network_details() -> crate::net_diag::NetworkDetails {
    crate::mobile_api::get_network_details()
}

/// Live ping-based connection quality for one peer (round-trip time, ping
/// successes/failures, loss fraction, canonical display labels).
///
/// Zeroed with `n/a` labels when the peer is not connected; both frontends
/// show the same Rust-formatted strings on the Peer Info page.
#[must_use]
pub fn get_peer_quality(peer_id: &str) -> crate::net_diag::PeerQuality {
    crate::mobile_api::get_peer_quality(peer_id)
}

/// Persisted network peers (bootstrap + relay multiaddrs): the Settings-editable
/// replacement for the `P2P_BOOTSTRAP` / `P2P_RELAY` env vars.
///
/// Both frontends render the list; the node reads it once at start.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_network_peers() -> Result<Vec<crate::network_config::NetworkPeer>, String> {
    crate::network_config::list_network_peers().map_err(|e| e.to_string())
}

/// Add one persisted network peer (`kind` = `bootstrap` or `relay`).
///
/// The address is validated by parsing it back into a libp2p multiaddr; relay
/// entries must carry `/p2p/<peer-id>`. Adding an already-stored address is a
/// no-op that returns the existing row. Takes effect on the next node start.
///
/// # Errors
/// Returns a human-readable validation/persistence error.
pub fn add_network_peer(
    kind: &str,
    multiaddr: &str,
) -> Result<crate::network_config::NetworkPeer, String> {
    crate::network_config::add_network_peer(kind, multiaddr)
}

/// Remove the persisted network peer with `id`; true when a row was deleted.
///
/// # Errors
/// Returns an error if the database delete fails.
pub fn remove_network_peer(id: i32) -> Result<bool, String> {
    crate::network_config::remove_network_peer(id).map_err(|e| e.to_string())
}

/// Whether LAN-only mode is enabled (US-051).
///
/// LAN-only disables the DHT, relays and internet bootstrap; mDNS and direct
/// dials still work. Both frontends show this on Settings.
#[must_use]
pub fn get_lan_only() -> bool {
    crate::settings::lan_only()
}

/// Enable or disable LAN-only mode (US-051).
///
/// The change is persisted and applies on the **next node start** — the swarm's
/// discovery/relay wiring is fixed when the node is built.
///
/// # Errors
/// Returns an error if the database write fails.
pub fn set_lan_only(enabled: bool) -> Result<(), String> {
    crate::settings::set_lan_only(enabled)
}

/// Resolved theme snapshot for the active palette (US-025/US-026).
///
/// Carries the accent swatch list so a frontend renders exactly the options
/// Rust accepts — no colour values or valid names are duplicated in Dart.
#[must_use]
pub fn get_theme() -> crate::theme::AppTheme {
    crate::theme::current()
}

/// Persist the dark/light + accent theme choice (US-025/US-026).
///
/// The accent name is validated before any write; an unknown colour is an
/// error rather than a silently ignored value.
///
/// # Errors
/// Returns an error for an unknown accent name or a failed database write.
pub fn set_theme(dark: bool, accent_name: &str) -> Result<(), String> {
    crate::theme::set(dark, accent_name)
}

/// Current notification delivery channels (US-027/US-028).
#[must_use]
pub fn get_notification_settings() -> crate::notifications::NotificationSettings {
    crate::notifications::settings()
}

/// Persist the notification sound + desktop channels (US-027/US-028).
///
/// # Errors
/// Returns an error if the database write fails.
pub fn set_notification_settings(sound: bool, desktop: bool) -> Result<(), String> {
    crate::notifications::set_settings(sound, desktop)
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

/// Locally hide one sent message (broadcast, DM, or group message).
///
/// Writes a tombstone only when the row was sent by us and is not already
/// hidden; incoming rows are never deletable. Nothing leaves this device:
/// `is_group` just picks which local table to stamp. History loaders and
/// group backfill skip tombstones, while the group watermark still includes
/// them so peers do not resend the row.
///
/// # Errors
/// Returns an error if the database write fails or no deletable sent row with
/// that id exists.
pub fn delete_message(message_id: i32, is_group: bool) -> Result<(), String> {
    let changed = if is_group {
        crate::groups::mark_group_message_deleted(message_id)?
    } else {
        crate::messages::mark_message_deleted(message_id)?
    };
    if changed {
        Ok(())
    } else {
        Err(format!("no deletable sent message with id {message_id}"))
    }
}

/// Locally replace the body of one sent message (broadcast, DM, or group).
///
/// Only a sent, non-deleted row is editable; incoming rows keep their copy.
/// The new body must contain non-whitespace text. This is device-only: no edit
/// is sent over the swarm, so peers still see the original message. `is_group`
/// just picks which local table to update.
///
/// # Errors
/// Returns an error if the body is blank, the database write fails, or no
/// editable sent row with that id exists.
pub fn edit_message(message_id: i32, is_group: bool, content: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("message body cannot be empty".to_string());
    }
    let changed = if is_group {
        crate::groups::edit_group_message(message_id, content)?
    } else {
        crate::messages::edit_message(message_id, content)?
    };
    if changed {
        Ok(())
    } else {
        Err(format!("no editable sent message with id {message_id}"))
    }
}

/// Save an outgoing broadcast message to DB and send via swarm.
/// `attachment_path`, when present, is stored content-addressedly (files lane
/// §12) and sent as an attachment descriptor on the message.
///
/// # Errors
/// Returns an error if the message cannot be saved, sent, or the attachment
/// stored/read.
pub fn save_outgoing_broadcast(
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_broadcast(content, attachment_path, reply_to)
}

/// Save an outgoing DM to DB and send via swarm. `attachment_path`, when
/// present, is stored content-addressedly (files lane §12) and sent as an
/// attachment descriptor on the message.
///
/// # Errors
/// Returns an error if the message cannot be saved, sent, or the attachment
/// stored/read.
pub fn save_outgoing_dm(
    peer_id: String,
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    crate::mobile_node::save_outgoing_dm(peer_id, content, attachment_path, reply_to)
}

/// Save an incoming message (broadcast or DM) to the database.
///
/// `latency` is the live event's measured round-trip time; it only decorates
/// the returned message (never persisted), mirroring the TUI. `encrypted`
/// records that the DM arrived sealed (US-042) so the flag survives a reload.
///
/// # Errors
/// Returns an error if the message cannot be saved.
// FRB 2.13 emits one Dart named argument per Rust parameter; grouping these
// into a struct would change the generated `saveIncomingMessage` signature and
// every Dart/web caller. The internal impl already takes a struct (see
// `mobile_node::IncomingMessageData`), so the extra argument is irreducible at
// this boundary.
#[allow(clippy::too_many_arguments)]
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
    latency: Option<String>,
    msg_id: Option<String>,
    reply_to: Option<crate::ReplyRef>,
    encrypted: bool,
) -> Result<ChatMessage, String> {
    crate::mobile_node::save_incoming_message(crate::mobile_node::IncomingMessageData {
        content,
        peer_id,
        is_direct,
        nickname,
        latency,
        msg_id,
        reply_to,
        encrypted,
    })
}

/// Dial a peer by multiaddr (manual peer address entry, US-053).
///
/// # Errors
/// Returns an error if the address is invalid or the node is not running.
pub fn dial_peer(address: String) -> Result<(), String> {
    crate::mobile_node::dial_peer(address)
}

// --- Peer blocklist / mute (US-054) ---

/// Whether a peer is muted. A blocked peer is ignored at the swarm boundary:
/// its traffic is dropped, it is never dialed, and an incoming connection is
/// refused — not merely hidden in the UI.
#[must_use]
pub fn is_peer_blocked(peer_id: &str) -> bool {
    crate::blocklist::is_blocked(peer_id)
}

/// Block one peer; `true` when it was not already blocked.
///
/// # Errors
/// Returns a human-readable error when the peer id does not parse or the
/// database write fails.
pub fn block_peer(peer_id: &str) -> Result<bool, String> {
    let changed = crate::blocklist::block_peer(peer_id)?;
    // Close any live connection: the swarm boundary refuses new ones, but this
    // one predates the block.
    crate::mobile_node::disconnect_peer(peer_id);
    Ok(changed)
}

/// Unblock one peer; `true` when a row was removed.
///
/// # Errors
/// Returns a human-readable error when the peer id does not parse or the
/// database delete fails.
pub fn unblock_peer(peer_id: &str) -> Result<bool, String> {
    crate::blocklist::unblock_peer(peer_id)
}

/// All blocked peer ids, oldest first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_blocked_peers() -> Result<Vec<String>, String> {
    crate::blocklist::list_blocked_peers().map_err(|e| e.to_string())
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
/// `attachment_path`, when present, is stored content-addressedly (files lane
/// §12) and sent as an attachment descriptor on the message.
///
/// # Errors
/// Returns an error if the message cannot be saved, sent, or the attachment
/// stored/read.
pub fn save_outgoing_group(
    group_id: String,
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<MobileGroupMessage, String> {
    crate::mobile_node::save_outgoing_group(group_id, content, attachment_path, reply_to)
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

/// Leave a group: delete the local rows (messages, membership, the group
/// itself) and unsubscribe from its gossipsub topic. Local-only — public
/// groups have no server-side membership to notify.
///
/// # Errors
/// Returns an error if the group rows cannot be deleted.
pub fn delete_group(group_id: String) -> Result<(), String> {
    crate::mobile_node::delete_group(group_id)
}

/// Set the local user's nickname.
///
/// # Errors
/// Returns an error if the nickname cannot be persisted.
pub fn set_self_nickname(nickname: &str) -> Result<(), String> {
    crate::set_self_nickname(nickname).map_err(|e| e.to_string())
}

/// Set a per-peer local nickname (the name only this install shows for that
/// peer). Validated against the canonical nickname rules
/// ([`validate_nickname`]) so every frontend accepts the same input.
///
/// # Errors
/// Returns an error if the nickname is invalid or cannot be persisted.
pub fn set_peer_local_nickname(peer_id: &str, nickname: &str) -> Result<(), String> {
    if !crate::nickname::validate_nickname(nickname) {
        return Err(format!("invalid nickname: {nickname}"));
    }
    crate::nickname::set_peer_local_nickname(peer_id, nickname).map_err(|e| e.to_string())
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
/// device-set ([`crate::generated::models_queryable::DeviceBinding`]).
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
pub fn accept_device_binding(peer_id: String, device_pubkey: Vec<u8>) -> Result<(), String> {
    // FFI boundary: FRB's Dart `List<int>` only deserializes into an owned
    // `Vec<u8>`; `&[u8]` is unsupported by codegen 2.13.0. The owned bytes are
    // moved all the way into `mint_device_binding`, so nothing needs an allow.
    crate::mobile_node::accept_device_binding(peer_id, device_pubkey)
}

/// Place an outgoing call: mint a session id, queue its `invite`.
///
/// Returns the fresh `call_id` so the caller can track the session end-to-end
/// (a session ends via [`send_call_signal`] with kind `end`).
///
/// # Errors
/// Returns an error if the request is refused by the running node.
pub fn start_call(peer_id: String) -> Result<String, String> {
    crate::mobile_node::start_call(peer_id)
}

/// Queue a call signal (`accept`/`reject`/`end`) over the calls lane; `call_id`
/// is echoed by every kind and pins the session (mint a fresh one with
/// [`start_call`]).
///
/// # Errors
/// Returns an error if the request is refused by the running node.
pub fn send_call_signal(peer_id: String, call_id: String, kind: String) -> Result<(), String> {
    crate::mobile_node::send_call_signal(peer_id, call_id, kind)
}

/// Send a transient DM typing state (`true` = started, `false` = stopped) on the
/// dedicated typing lane (US-009). Fire-and-forget: no message is persisted or
/// receipt expected by the caller.
///
/// # Errors
/// Returns an error if the request cannot be queued by the running node.
pub fn send_typing(peer_id: String, is_typing: bool) -> Result<(), String> {
    crate::mobile_node::send_typing(peer_id, is_typing)
}

/// Toggle this install's reaction on a message and return the fresh aggregated
/// chips (US-008).
///
/// The change is applied locally first, then synced to the peers holding the
/// message on the dedicated reaction lane.
///
/// # Errors
/// Returns an error if the emoji is invalid or the local change cannot be saved.
pub fn toggle_reaction(
    msg_id: String,
    emoji: String,
) -> Result<Vec<crate::reactions::ReactionSummary>, String> {
    crate::mobile_node::toggle_reaction(msg_id, emoji)
}

/// Load the aggregated reactions for one message (US-008).
#[must_use]
pub fn get_reactions(msg_id: &str) -> Vec<crate::reactions::ReactionSummary> {
    crate::mobile_node::get_reactions(msg_id)
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    fn db_lock() -> std::sync::MutexGuard<'static, ()> {
        crate::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[test]
    #[serial(db)]
    fn set_peer_local_nickname_validates_and_persists() {
        let _guard = db_lock();
        let dir = tempfile::tempdir().expect("tempdir");
        crate::db::reset_db_url();
        crate::mobile_api::init_mobile_database(
            dir.path().join("api.sqlite").to_string_lossy().into_owned(),
        )
        .expect("mobile init");

        // Valid: persisted and readable back through the canonical getter.
        set_peer_local_nickname("peer-1", "alice-1").expect("valid nickname");
        assert_eq!(
            crate::nickname::get_peer_local_nickname("peer-1")
                .expect("read back")
                .as_deref(),
            Some("alice-1")
        );

        // Invalid input is rejected by the same canonical rules everywhere
        // (empty, over the length cap, non-alphanumeric/dash characters).
        for bad in ["", "a-very-long-nickname-over-20", "has space", "bad!"] {
            assert!(
                set_peer_local_nickname("peer-1", bad).is_err(),
                "expected rejection for {bad:?}"
            );
        }
        // A rejected name never overwrites the stored one.
        assert_eq!(
            crate::nickname::get_peer_local_nickname("peer-1")
                .expect("read back")
                .as_deref(),
            Some("alice-1")
        );
    }

    #[test]
    #[serial(db)]
    fn delete_group_removes_group_and_membership() {
        let _guard = db_lock();
        let dir = tempfile::tempdir().expect("tempdir");
        crate::db::reset_db_url();
        crate::mobile_api::init_mobile_database(
            dir.path().join("api.sqlite").to_string_lossy().into_owned(),
        )
        .expect("mobile init");

        let group = crate::groups::create_public_group("Team Rocket").expect("create group");
        crate::groups::record_group_member(&group.group_id, "peer-1").expect("member");
        assert_eq!(
            crate::groups::get_group_member_count(&group.group_id).expect("count"),
            1
        );

        delete_group(group.group_id.clone()).expect("leave group");

        assert!(
            crate::groups::find_group(&group.group_id)
                .expect("lookup")
                .is_none(),
            "group row must be gone"
        );
        assert_eq!(
            crate::groups::get_group_member_count(&group.group_id).expect("count"),
            0,
            "membership must be gone"
        );

        // Unknown group: idempotent success (nothing to delete).
        delete_group("does-not-exist".to_string()).expect("idempotent leave");
    }
}
