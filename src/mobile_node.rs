//! Mobile node: manages the libp2p swarm lifecycle for Android/iOS.
//!
//! The swarm runs on a background tokio task. Commands are sent via MPSC,
//! events are polled by Dart via `poll_event()`.

use crate::messages::MessageMeta;
use crate::types::{SwarmCommand, SwarmEvent};
use crate::{
    CHAT_TOPIC, build_swarm, current_timestamp, ensure_self_nickname, gen_msg_id,
    get_local_peer_id, get_network_size, get_self_nickname, load_direct_messages, load_messages,
    mark_message_sent, p2plog_debug, record_peer_received_name_change, save_message_with_meta,
    save_peer, spawn_swarm_handler,
};
use chrono::TimeZone;
use libp2p::gossipsub;
use num_traits::ToPrimitive;
use serde::Serialize;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex, MutexGuard, OnceLock};
use tokio::sync::mpsc;

static NODE: OnceLock<Mutex<MobileNode>> = OnceLock::new();

/// Live set of peers currently connected to this node. Used to attribute
/// broadcasts we send to the peers that were online to receive them, and to
/// remember the most recent disconnection. Shared with the TUI via
/// [`crate::connected::ConnectedTracker`] so both frontends agree on what
/// counts as "currently connected".
static CONNECTED_PEERS: LazyLock<Mutex<crate::connected::ConnectedTracker>> =
    LazyLock::new(|| Mutex::new(crate::connected::ConnectedTracker::new()));

struct MobileNode {
    /// Broadcast fan-out of swarm events so multiple frontends (web poller,
    /// TUI, ...) can all receive every event instead of fighting over a
    /// single `mpsc` receiver.
    event_broadcast: Option<tokio::sync::broadcast::Sender<SwarmEvent>>,
    /// Receiver handed to [`poll_event`] (the web/Flutter consumer).
    poll_receiver: Option<tokio::sync::broadcast::Receiver<SwarmEvent>>,
    cmd_tx: Option<mpsc::Sender<SwarmCommand>>,
    peer_id: String,
    runtime: Option<tokio::runtime::Runtime>,
}

/// Start the p2p node with an explicit DB path.
/// Returns the local peer ID.
#[flutter_rust_bridge::frb(ignore)]
pub fn start_node(db_path: String) -> Result<String, String> {
    start_node_impl(Some(db_path))
}

/// Start the p2p node using automatic DB selection (lock-based, same as TUI).
/// Scans CWD for unlocked .db files, picks the first one, or creates a new one.
/// Returns the local peer ID.
#[flutter_rust_bridge::frb(ignore)]
pub fn start_node_auto() -> Result<String, String> {
    start_node_impl(None)
}

fn lock_node_mutex(mu: &Mutex<MobileNode>) -> MutexGuard<'_, MobileNode> {
    match mu.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Run `f` with the running node's command sender, or return `None` when the
/// node has not started or its command lane has closed (during shutdown).
///
/// Every command-queueing surface used to repeat the `NODE.get()` → lock →
/// `cmd_tx.as_ref()` dance; this keeps that lifecycle check in one place and
/// holds the node lock only for the duration of `f`.
fn with_cmd_tx<T>(f: impl FnOnce(&mpsc::Sender<SwarmCommand>) -> T) -> Option<T> {
    let tx = {
        let node = lock_node_mutex(NODE.get()?);
        node.cmd_tx.clone()?
    }; // release the node lock before a potentially blocking send
    Some(f(&tx))
}

/// Queue a command without blocking, returning whether it was accepted.
/// Fire-and-forget callers can ignore the bool; `send_typing` uses it to report
/// a stopped node.
fn try_send_cmd(cmd: SwarmCommand) -> bool {
    with_cmd_tx(|tx| tx.try_send(cmd).is_ok()).unwrap_or(false)
}

/// Queue a command that must reach the swarm, returning whether it was accepted.
fn blocking_send_cmd(cmd: SwarmCommand) -> bool {
    with_cmd_tx(|tx| tx.blocking_send(cmd).is_ok()).unwrap_or(false)
}

fn start_node_impl(db_path: Option<String>) -> Result<String, String> {
    if let Some(existing) = NODE.get() {
        let node = lock_node_mutex(existing);
        if node.cmd_tx.is_some() {
            return Ok(node.peer_id.clone());
        }
    }

    // Initialize logging - capture messages for the Flutter Log tab
    crate::logging::init_logging();
    // Register a no-op callback so logs go into the buffer (Flutter polls get_logs())
    // Previously buffered logs are replayed immediately.
    let cb: Arc<dyn Fn(String) + Send + Sync> = Arc::new(|_msg| {});
    crate::logging::register_log_callback(cb);

    if let Some(path) = db_path {
        crate::mobile_api::init_mobile_database(path)?;
    } else {
        // No path: use lock-based DB selection (same as TUI)
        crate::init_database().map_err(|e| e.to_string())?;
    }

    // Generate and persist a random nickname on first start (same as TUI)
    if let Err(e) = ensure_self_nickname() {
        p2plog_debug(format!("Failed to ensure self nickname: {e}"));
    }

    // Provision a user identity for this install (idempotent): creates the
    // users row keyed by a generated account keypair and self-binds the local
    // device. Later starts just return the existing account and never change
    // the account root, so every install is a holder that can mint device
    // bindings and accept sibling devices. Seed the announced user-level
    // nickname from the install's self nickname ensured above.
    let self_nickname = crate::get_self_nickname().ok().flatten();
    if let Err(e) = crate::identity::provision_my_user(None, self_nickname.as_deref()) {
        p2plog_debug(format!("Failed to provision user identity: {e}"));
    }

    // Create a dedicated tokio runtime — FRB may call us from a non-tokio thread,
    // but mDNS/swarm need a reactor.
    crate::net_diag::reset();
    let runtime =
        tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    // Block on swarm setup: build_swarm (mDNS), listen, subscribe, spawn handler.
    // tokio::spawn inside block_on uses this runtime; keeping it alive keeps the task alive.
    let (event_rx, cmd_tx, peer_id) = runtime.block_on(async {
        let network_size = get_network_size().map_err(|e| e.to_string())?;
        let mut swarm =
            build_swarm(network_size).map_err(|e| format!("Failed to build swarm: {e}"))?;

        // Sweep dead attachment blobs (files lane §12): `.part` partials from
        // interrupted pulls plus orphaned blobs nothing references any more.
        // No-op unless the `attachment-gc` Cargo feature is on.
        if let Err(e) = crate::swarm_handler::gc_stale_attachments() {
            p2plog_debug(format!("attachment GC failed: {e}"));
        }

        let topic = gossipsub::IdentTopic::new(CHAT_TOPIC);
        swarm
            .listen_on(
                "/ip4/0.0.0.0/tcp/0"
                    .parse()
                    .map_err(|e| format!("Failed to listen: {e}"))?,
            )
            .map_err(|e| format!("Failed to listen: {e}"))?;
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| format!("Failed to subscribe: {e}"))?;

        let (_handle, event_rx, cmd_tx) = spawn_swarm_handler(swarm, CHAT_TOPIC.to_string());

        // Re-join every group persisted in the local database: the swarm only
        // receives group traffic for topics it is subscribed to, so saved groups
        // must be re-subscribed on every startup.
        for summary in crate::groups::list_groups_with_member_counts()
            .map_err(|e| format!("Failed to list saved groups: {e}"))?
        {
            p2plog_debug(format!(
                "Subscribing to saved group {} ({})",
                summary.group.display_name, summary.group.group_id
            ));
            let _ = cmd_tx
                .send(SwarmCommand::SubscribeGroup {
                    group_id: summary.group.group_id.clone(),
                })
                .await;
        }

        let pid = get_local_peer_id().map_err(|e| e.to_string())?.to_string();

        Ok::<_, String>((event_rx, cmd_tx, pid))
    })?;

    // Fan out swarm events to multiple consumers (web poller, TUI, ...) via a
    // broadcast channel. Side effects (saving peers, nickname exchange, ...)
    // run exactly once in the forwarder task so consumers stay read-only.
    let (broadcast_tx, _) = tokio::sync::broadcast::channel::<SwarmEvent>(256);
    let broadcast_for_forwarder = broadcast_tx.clone();
    runtime.spawn(forward_swarm_events(
        event_rx,
        cmd_tx.clone(),
        broadcast_for_forwarder,
    ));

    let new_node = MobileNode {
        event_broadcast: Some(broadcast_tx.clone()),
        poll_receiver: Some(broadcast_tx.subscribe()),
        cmd_tx: Some(cmd_tx),
        peer_id: peer_id.clone(),
        runtime: Some(runtime),
    };

    // NODE is an OnceLock, so it can only be initialized once. On the first
    // start it is empty; on a restart (after stop_node) it already holds the
    // previous, now-inactive node. Replacing the inner value keeps the new
    // runtime and swarm handler alive — calling NODE.set here would fail
    // (already set) and silently drop the new node, killing the relaunched
    // swarm so its listen addresses (and all events) never come back.
    match NODE.get() {
        Some(existing) => {
            let mut g = lock_node_mutex(existing);
            *g = new_node;
        }
        None => {
            let _ = NODE.set(Mutex::new(new_node));
        }
    }
    p2plog_debug(format!("Mobile node started: peer_id={peer_id}"));
    Ok(peer_id)
}

/// Stop the p2p node (drops the swarm task).
#[flutter_rust_bridge::frb(ignore)]
pub fn stop_node() -> Result<(), String> {
    if let Some(m) = NODE.get() {
        let mut node = lock_node_mutex(m);
        node.cmd_tx.take();
        node.event_broadcast.take();
        node.poll_receiver.take();
        // The swarm-handler task (and the swarm itself) lives on our runtime.
        // Shut it down so the old swarm stops listening and no longer keeps
        // routing/responding to traffic while the node is stopped; a fresh
        // runtime is created on the next start.
        if let Some(rt) = node.runtime.take() {
            rt.shutdown_background();
        }
        // Drop the connected-peer snapshot so a later restart doesn't attribute
        // broadcast sends to peers that only connected to the previous session.
        if let Ok(mut set) = CONNECTED_PEERS.lock() {
            *set = crate::connected::ConnectedTracker::new();
        }
        p2plog_debug("Mobile node stopped".to_string());
    }
    crate::net_diag::reset();
    // Release DB lock so subsequent starts can acquire the same database
    crate::logging::p2plog_debug("[stop_node] releasing database lock".to_string());
    crate::db::release_db_lock();
    Ok(())
}

/// Poll the next swarm event (non-blocking). Returns None if no event ready.
/// Also processes events: saves peers, sends nickname DMs, stores received nicknames.
#[flutter_rust_bridge::frb(ignore)]
pub fn poll_event() -> Result<Option<SwarmEventJson>, String> {
    let m = NODE.get().ok_or("Node not started")?;
    loop {
        let recv_result = {
            let mut node = lock_node_mutex(m);
            let mut rx = node.poll_receiver.take().ok_or("Node stopped")?;
            let r = rx.try_recv();
            node.poll_receiver = Some(rx);
            drop(node);
            r
        };
        let ev = match recv_result {
            Ok(ev) => ev,
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => return Ok(None),
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                return Err("Swarm task ended".into());
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {
                // Dropped some events under backpressure; keep going.
                continue;
            }
        };

        // Side effects already ran in the forwarder task.
        // A nickname-only DM is the name-exchange side channel, not a
        // message: skip it so it never reaches Dart.
        if is_nickname_only_dm(&ev) {
            continue;
        }
        return Ok(Some(event_to_json(ev)));
    }
}

/// Subscribe to the node's event broadcast. Multiple frontends can subscribe
/// and each receives every event (fan-out). Side effects have already run in
/// the forwarder task.
#[flutter_rust_bridge::frb(ignore)]
pub fn subscribe_events() -> Result<tokio::sync::broadcast::Receiver<SwarmEvent>, String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = lock_node_mutex(m);
    node.event_broadcast
        .as_ref()
        .ok_or_else(|| "Node stopped".to_string())
        .map(tokio::sync::broadcast::Sender::subscribe)
}

/// Clone of the node's command sender (for frontends that drive the node).
#[flutter_rust_bridge::frb(ignore)]
pub fn command_sender() -> Result<mpsc::Sender<SwarmCommand>, String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = lock_node_mutex(m);
    node.cmd_tx
        .clone()
        .ok_or_else(|| "Node stopped".to_string())
}

/// True for `DirectMessage` events whose empty content carries only a
/// nickname. These are the outgoing nickname-exchange DM we send on connect
/// (echoed back by the peer) and similar name-only messages: the nickname is
/// a peer-record update (handled by [`process_event_for_mobile`]), never a
/// chat message to persist or surface in the UI. A reply snapshot makes the
/// message real user traffic (US-006) even with an empty body.
const fn is_nickname_only_dm(ev: &SwarmEvent) -> bool {
    matches!(
        ev,
        SwarmEvent::DirectMessage(m)
            if m.content.is_empty()
                && m.nickname.is_some()
                && m.attachment.is_none()
                && m.reply_to.is_none()
    )
}

/// Fan out swarm events to consumers, running each event's synchronous side
/// effects exactly once. Kept as a named function so the async/sync boundary is
/// explicit: the nickname greeting is sent with `send().await` here, never with
/// `blocking_send` (which panics inside this Tokio task).
async fn forward_swarm_events(
    mut rx: mpsc::Receiver<SwarmEvent>,
    cmd_tx: mpsc::Sender<SwarmCommand>,
    broadcast_tx: tokio::sync::broadcast::Sender<SwarmEvent>,
) {
    while let Some(ev) = rx.recv().await {
        if let Some(greeting) = process_event_for_mobile(&ev) {
            let _ = cmd_tx.send(greeting).await;
        }
        let _ = broadcast_tx.send(ev);
    }
}

/// Perform the synchronous side effects for one swarm event (save peers, store
/// nicknames, persist receipts). Returns the nickname-greeting command to send
/// on a new connection; the caller owns *how* it is sent so this stays safe to
/// call from both sync FRB entry points and the async forwarder.
fn process_event_for_mobile(ev: &SwarmEvent) -> Option<SwarmCommand> {
    match ev {
        SwarmEvent::PeerConnected(peer_id) => {
            // Track connected peers for broadcast attribution.
            if let Ok(mut set) = CONNECTED_PEERS.lock() {
                set.on_peer_connected(peer_id.clone());
                // Feed the peer-session history used for `get_network_size()`.
                let _ =
                    crate::peers::record_peer_session(i32::try_from(set.len()).unwrap_or(i32::MAX));
            }
            // Save peer to DB (like TUI does)
            if let Err(e) = save_peer(peer_id, &[]) {
                p2plog_debug(format!("Failed to save peer: {e}"));
            }
            // Send our nickname to the peer (nickname exchange). Return the
            // command rather than sending it here: the async forwarder awaits
            // the send, and sync FRB callers can use `blocking_send` safely.
            let nickname = get_self_nickname().ok().flatten();
            let msg_id = gen_msg_id();
            Some(SwarmCommand::SendDm {
                peer_id: peer_id.clone(),
                content: String::new(),
                nickname,
                msg_id: Some(msg_id),
                ack_for: None,
                attachment: None,
                reply_to: None,
            })
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            if let Ok(mut set) = CONNECTED_PEERS.lock() {
                set.on_peer_disconnected(peer_id, crate::current_timestamp());
                // Feed the peer-session history used for `get_network_size()`.
                let _ =
                    crate::peers::record_peer_session(i32::try_from(set.len()).unwrap_or(i32::MAX));
            }
            None
        }
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => {
            // Persist the receipt like the TUI does, so delivery state survives
            // restart and the Broadcast column's "confirmed" counts are not lost.
            let at = crate::current_timestamp();
            let is_dm = match crate::messages::message_is_dm(ack_for) {
                Ok(Some(is_dm)) => is_dm,
                _ => false,
            };
            let kind = i32::from(is_dm);
            if let Err(e) = crate::messages::save_receipt(ack_for, peer_id, kind, at) {
                p2plog_debug(format!("Failed to save receipt: {e}"));
            }
            None
        }
        SwarmEvent::BroadcastMessage(m) | SwarmEvent::DirectMessage(m) => {
            // Store the sender's announced nickname
            if let Some(nick) = &m.nickname
                && !nick.is_empty()
            {
                let _ = record_peer_received_name_change(&m.peer_id, nick);
            }
            // Nickname-only DMs are fully handled here (nickname stored above)
            // and are filtered out of the event stream by `poll_event`
            // (`is_nickname_only_dm`) so they never reach Dart or the DB.
            None
        }
        SwarmEvent::GroupMessage(m) => {
            // Store the sender's announced nickname (the group message itself is
            // persisted by the swarm handler).
            if let Some(nick) = &m.nickname
                && !nick.is_empty()
            {
                let _ = record_peer_received_name_change(&m.peer_id, nick);
            }
            None
        }
        // Inbound attachment finished downloading (files lane §12). The blob is
        // content-addressed on disk, so there is nothing to persist here — the
        // Dart side surfaces it off the `file_received` event.
        //
        // Log it exactly as the TUI does so the Flutter/web Log tab shows the
        // same "file arrived" line the TUI binary shows.
        SwarmEvent::FileReceived {
            peer_id,
            file_id,
            name,
            path,
        } => {
            p2plog_debug(format!(
                "Received file {name} ({file_id}) from {peer_id} at {path}"
            ));
            None
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            // Mirrors the TUI's log line (mDNS's peer-expiry notice).
            p2plog_debug(format!("Peer expired: {peer_id}"));
            None
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, addresses } => {
            let addrs: Vec<String> = addresses.iter().map(ToString::to_string).collect();
            if let Err(e) = save_peer(peer_id, &addrs) {
                p2plog_debug(format!("Failed to save discovered peer: {e}"));
            }
            None
        }
        _ => None,
    }
}

/// Get the local peer ID.
///
/// # Errors
/// Returns an error if the node has not been started yet.
pub fn get_node_peer_id() -> Result<String, String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = lock_node_mutex(m);
    Ok(node.peer_id.clone())
}

// --- Mobile peer record with nicknames ---

/// A known peer with the nickname fields needed by the mobile Peer Info page.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobilePeerRecord {
    /// Stable libp2p peer ID.
    pub peer_id: String,
    /// First contact, formatted for display.
    pub first_seen: String,
    /// Last activity, formatted for display.
    pub last_seen: String,
    /// Nickname received from the peer, if any.
    pub nickname: Option<String>,
    /// Nickname assigned locally to this peer, if any.
    pub local_nickname: Option<String>,
    /// Resolved display name (local → received → generated petname).
    pub display_name: String,
}

/// Get all known peers from the database with nickname info.
#[flutter_rust_bridge::frb(ignore)]
pub fn get_known_peers() -> Result<Vec<MobilePeerRecord>, String> {
    let known = crate::load_peers().map_err(|e| e.to_string())?;
    Ok(known
        .into_iter()
        .map(|p| {
            let display_name =
                crate::get_peer_display_name(&p.peer_id).unwrap_or_else(|_| p.peer_id.clone());
            MobilePeerRecord {
                peer_id: p.peer_id,
                first_seen: chrono::Utc
                    .from_utc_datetime(&p.first_seen)
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                last_seen: chrono::Utc
                    .from_utc_datetime(&p.last_seen)
                    .with_timezone(&chrono::Local)
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                nickname: p.received_nickname,
                local_nickname: p.peer_local_nickname,
                display_name,
            }
        })
        .collect())
}

// --- Message history types ---

/// Channel and direction of a chat row.
///
/// Grouped so [`ChatMessage`] stays under clippy's bool limit; `serde(flatten)`
/// keeps the web JSON keys (`isBroadcast`, `sent`) exactly as before, and Dart
/// keeps its old property names through a hand-written extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessageOrigin {
    /// True for a gossipsub broadcast, false for DMs and group messages.
    pub is_broadcast: bool,
    /// True for messages we sent, false for incoming ones.
    pub sent: bool,
}

/// One chat message row shared by the broadcast, DM and group screens.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    /// Database row ID (unique per message).
    pub id: i32,
    /// Message body, or the canonical file chip for an attachment-only row.
    pub content: String,
    /// Peer ID of the sender (incoming) or partner (outgoing DM), if known.
    pub peer_id: Option<String>,
    /// Channel and direction of this row (flat in the serialized form).
    #[serde(flatten)]
    pub origin: ChatMessageOrigin,
    /// Intended recipient of an outgoing DM, if any.
    pub target_peer: Option<String>,
    /// Stable wire message ID, used for dedup and delivery receipts.
    pub msg_id: Option<String>,
    /// Immutable reply snapshot (US-006); `None` for an ordinary message.
    pub reply_to: Option<crate::ReplyRef>,
    /// Send time as `HH:MM` (from the Unix-seconds wire value), if recorded.
    pub sent_at: Option<String>,
    /// Time the row was stored, formatted for display.
    pub created_at: String,
    /// Display label for the sender, when the nickname is known.
    pub sender_nickname: Option<String>,
    /// Attachment descriptor, if the message carries a file.
    pub attachment: Option<MobileAttachment>,
    /// True when the body arrived sealed end to end (US-042) — surfaced in
    /// the UI as a lock marker. Always false for outgoing rows and broadcasts.
    pub encrypted: bool,
    /// True once a recipient acknowledged this (outgoing) message — a delivery
    /// tick in the UI. Always false for incoming messages and for the value
    /// returned right after sending; [`load_broadcast_messages`] /
    /// [`load_dm_messages`] fill it in from the receipts table.
    pub confirmed: bool,
    /// True when the body was edited locally after sending (US-005). The edit
    /// is device-only; peers keep the original body. Filled from the row's
    /// `edited_at` marker on history loads.
    pub edited: bool,
    /// Round-trip latency of a *live* incoming message (`5ms`, `<1ms`, `1.2s`),
    /// the value the TUI prefixes onto the message line. `None` for outgoing
    /// messages, group messages, and every row read back from the database —
    /// the TUI measures it in memory only and never persists it.
    pub latency: Option<String>,
    /// Aggregated reactions on this message (US-008). Empty on a freshly sent
    /// row; history loaders fill it in one batched query via the internal
    /// `hydrate_reactions` helper.
    pub reactions: Vec<crate::reactions::ReactionSummary>,
}

/// Dial a peer by multiaddr (manual peer address entry, US-053).
///
/// # Errors
/// Returns an error if the address is invalid or the node is not running.
#[flutter_rust_bridge::frb(ignore)]
pub fn dial_peer(address: String) -> Result<(), String> {
    if !blocking_send_cmd(SwarmCommand::Dial { address }) {
        return Err("Failed to queue dial: node not running".into());
    }
    Ok(())
}

/// Drop the live connection to a peer (blocklist toggle, US-054).
///
/// Best-effort: when the node is not running there is nothing to disconnect.
/// `try_send` keeps this callable from the sync FRB surface.
#[flutter_rust_bridge::frb(ignore)]
pub fn disconnect_peer(peer_id: &str) {
    let _ = try_send_cmd(SwarmCommand::DisconnectPeer {
        peer_id: peer_id.to_string(),
    });
}

/// Mobile mirror of [`crate::files::MessageAttachment`] (files lane §12).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileAttachment {
    /// SHA-256 hex of the file bytes (content address)
    pub file_id: String,
    /// Original filename the sender advertised
    pub name: String,
    /// Total byte length
    pub size: i64,
    /// Human-readable label for `size` (`2 KiB`, `64 MiB`), produced by the
    /// canonical `files::format_file_size` so the Flutter chip does no byte
    /// math of its own.
    pub size_label: String,
    /// Canonical `[file] name (size)` chip text, reused verbatim for
    /// attachment-only reply previews so no frontend assembles it.
    pub chip: String,
    /// Best-effort MIME type hint (`None` when unknown)
    pub mime: Option<String>,
}

fn mobile_attachment(att: crate::files::MessageAttachment) -> MobileAttachment {
    let chip = crate::files::file_chip(&att);
    MobileAttachment {
        file_id: att.file_id,
        name: att.name,
        size: i64::try_from(att.size).unwrap_or(i64::MAX),
        size_label: crate::files::format_file_size(att.size),
        chip,
        mime: att.mime,
    }
}

fn attachment_from_row(attachment_json: Option<&str>) -> Option<MobileAttachment> {
    attachment_json
        .and_then(crate::files::attachment_from_json)
        .map(mobile_attachment)
}

/// Resolve a message's sender label, shared by DMs/broadcasts and group chats.
///
/// Always appends a short peer-ID suffix to an announced nickname so the name
/// stays consistent across messages (announced nickname or generated petname).
/// Falls back to the resolved display name when no nickname was announced.
fn sender_nickname_label(
    sender_nickname: Option<String>,
    peer_id: Option<String>,
) -> Option<String> {
    match (sender_nickname, peer_id) {
        (Some(nick), Some(pid)) => {
            let suffix = crate::fmt::peer_id_suffix(&pid);
            Some(format!("{nick} ({suffix})"))
        }
        (Some(nick), None) => Some(nick),
        (None, Some(pid)) => crate::get_peer_display_name(&pid).ok(),
        (None, None) => None,
    }
}

fn message_to_chat(msg: crate::generated::models_queryable::Message) -> ChatMessage {
    let sender_nickname = sender_nickname_label(msg.sender_nickname.clone(), msg.peer_id.clone());
    ChatMessage {
        id: msg.id,
        content: msg.content,
        peer_id: msg.peer_id,
        origin: ChatMessageOrigin {
            is_broadcast: msg.is_direct == 0,
            sent: msg.sent == 1,
        },
        target_peer: msg.target_peer,
        msg_id: msg.msg_id,
        reply_to: msg.reply_to_json.as_deref().and_then(crate::reply::decode),
        sent_at: msg.sent_at.map(|t| {
            // `sent_at` is a Unix timestamp in seconds (well within i64 range);
            // `to_i64` truncates the fractional part exactly like `as i64` did.
            let secs = t.to_i64().unwrap_or_default();
            let dt = chrono::DateTime::from_timestamp(secs, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Local);
            dt.format("%H:%M").to_string()
        }),
        created_at: chrono::Utc
            .from_utc_datetime(&msg.created_at)
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        sender_nickname,
        attachment: attachment_from_row(msg.attachment_json.as_deref()),
        encrypted: msg.encrypted != 0,
        confirmed: false,
        edited: msg.edited_at.is_some(),
        latency: None,
        reactions: Vec::new(),
    }
}

/// Flag which of these outgoing messages a recipient has acknowledged, using a
/// single batched query over `message_receipts` (see
/// [`crate::messages::confirmed_message_ids`]). A DB failure leaves every flag
/// false and logs, so the history view still renders.
fn mark_confirmed(messages: &mut [ChatMessage]) {
    let ids: Vec<String> = messages
        .iter()
        .filter(|m| m.origin.sent)
        .filter_map(|m| m.msg_id.clone())
        .collect();
    if ids.is_empty() {
        return;
    }
    match crate::messages::confirmed_message_ids(&ids) {
        Ok(confirmed) => {
            for m in messages.iter_mut() {
                // Only our own sent rows can be acknowledged. Incoming rows now
                // persist `msg_id` too, so an echoed id must never tick an
                // inbound message.
                m.confirmed =
                    m.origin.sent && m.msg_id.as_ref().is_some_and(|id| confirmed.contains(id));
            }
        }
        Err(e) => p2plog_debug(format!("Failed to load message receipts: {e}")),
    }
}

/// Fill reaction chips for a batch of messages in one query (US-008).
fn hydrate_reactions(messages: &mut [ChatMessage]) {
    let ids: Vec<String> = messages.iter().filter_map(|m| m.msg_id.clone()).collect();
    let all = crate::reactions::summaries_for_many(&ids);
    for m in messages.iter_mut() {
        if let Some(id) = &m.msg_id {
            m.reactions = all.get(id).cloned().unwrap_or_default();
        }
    }
}

/// Fill reaction chips for a batch of group messages in one query (US-008).
fn hydrate_group_reactions(messages: &mut [MobileGroupMessage]) {
    let ids: Vec<String> = messages.iter().filter_map(|m| m.msg_id.clone()).collect();
    let all = crate::reactions::summaries_for_many(&ids);
    for m in messages.iter_mut() {
        if let Some(id) = &m.msg_id {
            m.reactions = all.get(id).cloned().unwrap_or_default();
        }
    }
}

/// Load broadcast messages (newest-first from DB, reversed to chronological).
#[flutter_rust_bridge::frb(ignore)]
pub fn load_broadcast_messages(limit: i64) -> Result<Vec<ChatMessage>, String> {
    // `limit` is a message count; clamp a negative/wrapping value to the
    // maximum so the loader simply returns every available row.
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let msgs = load_messages(CHAT_TOPIC, limit_usize).map_err(|e| e.to_string())?;
    let mut out: Vec<ChatMessage> = msgs.into_iter().rev().map(message_to_chat).collect();
    mark_confirmed(&mut out);
    hydrate_reactions(&mut out);
    Ok(out)
}

/// Load DM history with a specific peer (already oldest-first in DB).
#[flutter_rust_bridge::frb(ignore)]
pub fn load_dm_messages(peer_id: String, limit: i64) -> Result<Vec<ChatMessage>, String> {
    // `limit` is a message count; clamp a negative/wrapping value to the
    // maximum so the loader simply returns every available row.
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let msgs = load_direct_messages(&peer_id, limit_usize).map_err(|e| e.to_string())?;
    let mut out: Vec<ChatMessage> = msgs.into_iter().map(message_to_chat).collect();
    mark_confirmed(&mut out);
    hydrate_reactions(&mut out);
    Ok(out)
}

/// Store an outbound file (files lane §12) and return its delivery descriptor,
/// or `None` when the caller sent no path. Failures are returned as `Err` so the
/// frontend can surface them instead of silently sending a text-only message.
fn store_send_attachment(
    attachment_path: Option<String>,
) -> Result<Option<crate::files::MessageAttachment>, String> {
    attachment_path.map_or(Ok(None), |path| {
        crate::files::store_outgoing_file(&path)
            .map(Some)
            .map_err(|e| e.to_string())
    })
}

/// Save an outgoing broadcast message, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_outgoing_broadcast(
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_broadcast_attached(content, attachment, reply_to)
}

/// [`save_outgoing_broadcast`] with a pre-stored attachment (files lane §12).
///
/// The web upload route (`docs/web_frontend.md`) stores the blob from
/// raw bytes server-side and passes the descriptor straight through, so the
/// path-based step never happens on that flow.
pub(crate) fn save_outgoing_broadcast_attached(
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
        reply_to: reply_to.clone(),
        encrypted: false,
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, false, None, meta)
        .map_err(|e| e.to_string())?;

    // Send via swarm
    let queued = blocking_send_cmd(SwarmCommand::Publish {
        content,
        nickname,
        msg_id: Some(msg_id.clone()),
        attachment,
        reply_to,
    });

    let chat = message_to_chat(msg);
    // Only claim transmission if the swarm actually accepted the command; a
    // stopped node (closed channel) leaves the message unsent instead of
    // showing a false "sent" checkmark.
    if queued {
        let _ = mark_message_sent(chat.id);
    }

    // Attribute this broadcast to every peer that was online to receive it.
    let connected: Vec<String> = CONNECTED_PEERS
        .lock()
        .map_or_default(|t| t.connected_peer_ids().map(str::to_owned).collect());
    if !connected.is_empty() {
        let _ = crate::peers::record_broadcast_recipients(&msg_id, &connected);
    }

    Ok(chat)
}

/// Save an outgoing DM, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_outgoing_dm(
    peer_id: String,
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_dm_attached(peer_id, content, attachment, reply_to)
}

/// [`save_outgoing_dm`] with a pre-stored attachment (files lane §12); see
/// [`save_outgoing_broadcast_attached`].
pub(crate) fn save_outgoing_dm_attached(
    peer_id: String,
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
        reply_to: reply_to.clone(),
        encrypted: false,
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, true, Some(&peer_id), meta)
        .map_err(|e| e.to_string())?;

    // Send via swarm
    let queued = blocking_send_cmd(SwarmCommand::SendDm {
        peer_id,
        content,
        nickname,
        msg_id: Some(msg_id),
        ack_for: None,
        attachment,
        reply_to,
    });

    let chat = message_to_chat(msg);
    if queued {
        let _ = mark_message_sent(chat.id);
    }
    Ok(chat)
}

/// Fields for persisting one incoming broadcast or DM (internal; the FRB
/// wrapper in [`crate::api`] takes the same values as separate named args so
/// Dart keeps its generated signature).
pub struct IncomingMessageData {
    /// Message body (may be empty for an attachment-only message).
    pub content: String,
    /// Sender peer id.
    pub peer_id: String,
    /// `true` for a DM, `false` for a broadcast.
    pub is_direct: bool,
    /// Announced sender nickname, if any.
    pub nickname: Option<String>,
    /// Live round-trip time decoration (never persisted).
    pub latency: Option<String>,
    /// Stable message id (reply/receipt correlation).
    pub msg_id: Option<String>,
    /// Immutable reply snapshot (US-006).
    pub reply_to: Option<crate::ReplyRef>,
    /// Whether the DM arrived sealed (US-042); persisted.
    pub encrypted: bool,
}

/// Save an incoming message (broadcast or DM) to the database.
///
/// `latency` is the measured round-trip time of the live event; it decorates
/// the returned [`ChatMessage`] only (never written to the database), matching
/// the TUI's in-memory rendering. `encrypted` is the live US-042 flag and *is*
/// persisted, so a reloaded DM still shows the lock marker.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_incoming_message(data: IncomingMessageData) -> Result<ChatMessage, String> {
    let IncomingMessageData {
        content,
        peer_id,
        is_direct,
        nickname,
        latency,
        msg_id,
        reply_to,
        encrypted,
    } = data;
    let target = is_direct.then_some(peer_id.as_str());
    // Ensure the peer row exists (and gets a generated petname if it's a
    // silent peer seen for the first time / before this feature) so the
    // message's sender label can resolve to a name instead of a raw ID.
    if let Err(e) = crate::save_peer(&peer_id, &[]) {
        p2plog_debug(format!("Failed to ensure peer on incoming message: {e}"));
    }
    let meta = MessageMeta {
        sender_nickname: nickname,
        msg_id,
        sent_at: None,
        attachment: None,
        reply_to,
        encrypted,
    };
    let msg = save_message_with_meta(
        &content,
        Some(&peer_id),
        CHAT_TOPIC,
        is_direct,
        target,
        meta,
    )
    .map_err(|e| e.to_string())?;
    let mut chat = message_to_chat(msg);
    chat.latency = latency;
    Ok(chat)
}

// --- Device-binding exchange (identity lane §11) ---

/// Introduce this device (and its account if provisioned) to a peer by
/// sending a device-binding `intro` request over the swarm.
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn send_device_binding_intro(peer_id: String) -> Result<(), String> {
    let request = crate::identity::intro_device_binding_request().map_err(|e| e.to_string())?;
    if !blocking_send_cmd(SwarmCommand::SendDeviceBinding {
        peer_id,
        request: Box::new(request),
    }) {
        return Err("Failed to queue device-binding intro: node not running".into());
    }
    Ok(())
}

/// Accept a device's intro by minting a signed claim that binds that device
/// into **our** account's device-set, persisting the local binding and sending
/// the claim to the device over the swarm.
///
/// # Errors
/// Returns an error if the install has not been provisioned, the claim cannot
/// be minted, or the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn accept_device_binding(peer_id: String, device_pubkey: Vec<u8>) -> Result<(), String> {
    let request =
        crate::identity::mint_device_binding(&peer_id, device_pubkey).map_err(|e| e.to_string())?;
    if !blocking_send_cmd(SwarmCommand::SendDeviceBinding {
        peer_id,
        request: Box::new(request),
    }) {
        return Err("Failed to queue device-binding claim: node not running".into());
    }
    Ok(())
}

/// Place an outgoing call to `peer_id`: mint a fresh session id and queue its
/// `invite` over the calls lane. Returns the new `call_id` so the caller can
/// track the session end-to-end (a session ends via `send_call_signal` with
/// kind `end`).
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn start_call(peer_id: String) -> Result<String, String> {
    let call_id = crate::gen_msg_id();
    send_call_signal(peer_id, call_id.clone(), "invite".to_string())?;
    Ok(call_id)
}

/// Queue a call signal (`accept`/`reject`/`end`) over the calls lane; `call_id`
/// is echoed by every kind and pins the session (mint a fresh one with
/// [`Self::start_call`]).
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn send_call_signal(peer_id: String, call_id: String, kind: String) -> Result<(), String> {
    if !blocking_send_cmd(SwarmCommand::SendCallSignal {
        peer_id,
        signal: Box::new(crate::behavior::CallSignal::new(&kind, call_id)),
    }) {
        return Err("Failed to queue call signal: node not running".into());
    }
    Ok(())
}

/// Open the Rust-owned media session for an accepted call (calls lane, US-044).
/// Decoded peer audio then arrives as `call_media` swarm events.
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn start_call_media(call_id: String, peer_id: String) -> Result<(), String> {
    if !blocking_send_cmd(SwarmCommand::StartCallMedia { call_id, peer_id }) {
        return Err("Failed to queue call media start: node not running".into());
    }
    Ok(())
}

/// Close the active media session (hang-up, reject, or leaving the call screen).
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn stop_call_media() -> Result<(), String> {
    if !blocking_send_cmd(SwarmCommand::StopCallMedia) {
        return Err("Failed to queue call media stop: node not running".into());
    }
    Ok(())
}

/// Encode and send one 16 kHz mono PCM block on the active session.
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn send_call_media(call_id: String, pcm: Vec<i16>) -> Result<(), String> {
    if !blocking_send_cmd(SwarmCommand::SendCallMedia { call_id, pcm }) {
        return Err("Failed to queue call media frame: node not running".into());
    }
    Ok(())
}

/// Mute or unmute outgoing audio; inbound audio keeps playing.
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn set_call_media_mute(muted: bool) -> Result<(), String> {
    if !try_send_cmd(SwarmCommand::SetCallMediaMute { muted }) {
        return Err("Failed to queue call media mute: node not running".into());
    }
    Ok(())
}

/// Queue a transient typing state (`true` on first keystroke, `false` on send,
/// empty input, or leaving the DM) over the dedicated typing lane.
///
/// # Errors
/// Returns an error if the request is refused by the running node.
#[flutter_rust_bridge::frb(ignore)]
pub fn send_typing(peer_id: String, is_typing: bool) -> Result<(), String> {
    // Fire-and-forget: never block the UI if the command lane is busy.
    if !try_send_cmd(SwarmCommand::SendTyping { peer_id, is_typing }) {
        return Err("Failed to queue typing signal: node not running".into());
    }
    Ok(())
}

/// Toggle this install's reaction on a message and sync the change to the peers
/// that hold it (US-008). The local change is applied before queueing, so the UI
/// can refresh from the returned summaries even if the node is offline; a failed
/// queue only means peers will hear about it on a later action.
///
/// # Errors
/// Returns an error if the emoji is invalid, the DB write fails, or the local
/// identity is not yet available.
#[flutter_rust_bridge::frb(ignore)]
pub fn toggle_reaction(
    msg_id: String,
    emoji: String,
) -> Result<Vec<crate::reactions::ReactionSummary>, String> {
    let emoji = crate::reactions::validate_emoji(&emoji)?;
    let actor = crate::peers::local_peer_id_string()
        .ok_or_else(|| "local peer id is not available".to_string())?;
    // Compute recipients from the *current* row before applying the change:
    // the message lookup is independent of reactions, but doing it first keeps
    // the intent obvious and the send consistent with the message's scope.
    let recipients = crate::reactions::recipients_for_message(&msg_id);
    let remove = crate::reactions::has_reaction(&msg_id, &actor, &emoji)?;
    crate::reactions::apply(&msg_id, &actor, &emoji, remove)?;

    let _ = try_send_cmd(SwarmCommand::SendReaction {
        peer_ids: recipients,
        signal: Box::new(crate::behavior::ReactionSignal::new(
            msg_id.clone(),
            emoji,
            actor,
            remove,
        )),
    });
    crate::reactions::summaries_for(&msg_id)
}

/// Load the aggregated reactions for one message (US-008).
#[must_use]
pub fn get_reactions(msg_id: &str) -> Vec<crate::reactions::ReactionSummary> {
    crate::reactions::summaries_for(msg_id).unwrap_or_default()
}

// --- Group chat types and functions ---

/// A saved group shown in the group list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileGroup {
    /// Stable group topic identifier.
    pub group_id: String,
    /// Human-readable group name.
    pub display_name: String,
    /// Number of members recorded for this group.
    pub member_count: i64,
    /// True for a private (invite-only) group.
    pub is_private: bool,
}

/// One message in a group's persisted history.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileGroupMessage {
    /// Database row ID.
    pub id: i32,
    /// Message body, or the canonical file chip for an attachment-only row.
    pub content: String,
    /// Peer ID of the sender, when recorded.
    pub peer_id: Option<String>,
    /// True for a message we sent.
    pub sent: bool,
    /// True when the body was edited locally after sending (US-005). Device-only,
    /// filled from the row's `edited_at` marker on history loads.
    pub edited: bool,
    /// Stable wire ID used for deduplication, if the sender supplied one.
    pub msg_id: Option<String>,
    /// Immutable reply snapshot (US-006); `None` for an ordinary message.
    pub reply_to: Option<crate::ReplyRef>,
    /// Send time as `HH:MM`, if recorded.
    pub sent_at: Option<String>,
    /// Time the row was stored, formatted for display.
    pub created_at: String,
    /// Display label for the sender, when the nickname is known.
    pub sender_nickname: Option<String>,
    /// Attachment descriptor, if the message carries a file.
    pub attachment: Option<MobileAttachment>,
    /// Aggregated reactions on this message (US-008). Empty on a freshly sent
    /// row; history loaders fill it in one batched query.
    pub reactions: Vec<crate::reactions::ReactionSummary>,
}

fn group_to_mobile(
    group: crate::generated::models_queryable::Group,
    member_count: i64,
) -> MobileGroup {
    MobileGroup {
        group_id: group.group_id,
        display_name: group.display_name,
        member_count,
        is_private: group.is_private == 1,
    }
}

fn group_message_to_mobile(
    msg: crate::generated::models_queryable::GroupMessage,
) -> MobileGroupMessage {
    let sender_nickname = sender_nickname_label(msg.sender_nickname.clone(), msg.peer_id.clone());
    MobileGroupMessage {
        id: msg.id,
        content: msg.content,
        peer_id: msg.peer_id,
        sent: msg.sent == 1,
        edited: msg.edited_at.is_some(),
        msg_id: msg.msg_id,
        reply_to: msg.reply_to_json.as_deref().and_then(crate::reply::decode),
        sent_at: msg.sent_at.map(|t| {
            // `sent_at` is a Unix timestamp in seconds (well within i64 range);
            // `to_i64` truncates the fractional part exactly like `as i64` did.
            let secs = t.to_i64().unwrap_or_default();
            let dt = chrono::DateTime::from_timestamp(secs, 0)
                .unwrap_or_default()
                .with_timezone(&chrono::Local);
            dt.format("%H:%M").to_string()
        }),
        created_at: chrono::Utc
            .from_utc_datetime(&msg.created_at)
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        sender_nickname,
        attachment: attachment_from_row(msg.attachment_json.as_deref()),
        reactions: Vec::new(),
    }
}

/// Accept a pending group invite: once the local peer records itself as a group
/// member (idempotent — safe to re-accept), the Block-D inbound classifier flips
/// this peer from `GroupInvite`-routing to real `GroupMessage`-routing for that
/// group, and the returned `MobileGroup` hugs the exact same `group_to_mobile`
/// tail a fresh `create_group` returns (List parity in Flutter Groups tab).
///
/// An invite only carries the `group_id` (the stable hash of the group's
/// canonical name — the name itself is unknown here), so when no local `groups`
/// row exists yet one is materialized with the id as fallback display name.
///
/// # Errors
/// Returns an error if the group row cannot be materialized/loaded.
#[flutter_rust_bridge::frb(ignore)]
pub fn accept_group_invite(group_id: String) -> Result<MobileGroup, String> {
    let group = crate::groups::ensure_group_for_invite(&group_id).map_err(|e| e.to_string())?;
    let local_peer_id = crate::get_local_peer_id()
        .map_err(|e| e.to_string())?
        .to_string();
    let _ = crate::groups::record_group_member(&group_id, &local_peer_id);
    // The swarm only receives traffic for subscribed topics, so (like
    // `create_group`) re-subscribe now or the accepted group stays silent
    // until the next restart re-joins saved groups.
    let _ = blocking_send_cmd(SwarmCommand::SubscribeGroup {
        group_id: group_id.clone(),
    });
    let member_count = crate::groups::get_group_member_count(&group_id)
        .unwrap_or_default()
        .max(0);
    Ok(group_to_mobile(group, member_count))
}

/// Create a group using the standard `create_public_group` seam, then route
/// the group's local genesis `SubscribeGroup` and return the group's
/// `MobileGroup` shape with its current member count.
///
/// # Errors
///
/// Returns an error if the group name is empty, a group with this name already
/// exists, or the group membership count cannot be resolved.
#[flutter_rust_bridge::frb(ignore)]
pub fn create_group(name: &str) -> Result<MobileGroup, String> {
    let group = crate::groups::create_public_group(name).map_err(|e| e.to_string())?;
    let _ = blocking_send_cmd(SwarmCommand::SubscribeGroup {
        group_id: group.group_id.clone(),
    });
    let member_count = crate::groups::get_group_member_count(&group.group_id)
        .unwrap_or_default()
        .max(0);
    Ok(group_to_mobile(group, member_count))
}

/// Leave a group: drop the local row (messages, membership, the group itself)
/// and unsubscribe from its gossipsub topic so the swarm stops delivering it.
///
/// Local-only by design — public groups have no server-side membership to
/// notify; other members keep their copy of the history.
///
/// # Errors
/// Returns an error if the group rows cannot be deleted.
pub fn delete_group(group_id: String) -> Result<(), String> {
    crate::groups::delete_group(&group_id).map_err(|e| e.to_string())?;
    let _ = blocking_send_cmd(SwarmCommand::UnsubscribeGroup { group_id });
    Ok(())
}

/// List all known groups with their member counts.
#[flutter_rust_bridge::frb(ignore)]
pub fn list_groups() -> Result<Vec<MobileGroup>, String> {
    let summaries = crate::groups::list_groups_with_member_counts().map_err(|e| e.to_string())?;
    Ok(summaries
        .into_iter()
        .map(|s| group_to_mobile(s.group, s.member_count.max(0)))
        .collect())
}

/// Load a group's message history (newest-first from DB, reversed to chronological).
#[flutter_rust_bridge::frb(ignore)]
pub fn load_group_messages(
    group_id: String,
    limit: i64,
) -> Result<Vec<MobileGroupMessage>, String> {
    // `limit` is a message count; clamp a negative/wrapping value to the
    // maximum so the loader simply returns every available row.
    let limit_usize = usize::try_from(limit).unwrap_or(usize::MAX);
    let msgs =
        crate::groups::load_group_messages(&group_id, limit_usize).map_err(|e| e.to_string())?;
    let mut out: Vec<MobileGroupMessage> = msgs
        .into_iter()
        .rev()
        .map(group_message_to_mobile)
        .collect();
    hydrate_group_reactions(&mut out);
    Ok(out)
}

/// Save an outgoing group message, persist to DB, and publish via the swarm.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_outgoing_group(
    group_id: String,
    content: String,
    attachment_path: Option<String>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<MobileGroupMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_group_attached(group_id, content, attachment, reply_to)
}

/// [`save_outgoing_group`] with a pre-stored attachment (files lane §12); see
/// [`save_outgoing_broadcast_attached`].
pub(crate) fn save_outgoing_group_attached(
    group_id: String,
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
) -> Result<MobileGroupMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();
    let meta = crate::groups::GroupMessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
        reply_to: reply_to.clone(),
    };
    let msg = crate::groups::save_outgoing_group_message(&group_id, &content, meta)
        .map_err(|e| e.to_string())?;
    let _ = blocking_send_cmd(SwarmCommand::PublishGroup {
        group_id,
        content,
        nickname,
        msg_id: Some(msg_id),
        attachment,
        reply_to,
    });
    Ok(group_message_to_mobile(msg))
}

/// Absolute on-disk path for a received attachment's blob (files lane §12).
///
/// The blob may not exist yet if the pull hasn't finished (`file_id` from an
/// inbound descriptor); the sender's blob lands here once the swarm handler
/// completes the chunk transfer. Dart uses this to open/save the file.
///
/// # Errors
/// Returns an error if the store directory cannot be created.
pub fn attachment_file_path(file_id: &str) -> Result<String, String> {
    crate::files::store_path_for(file_id)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

// --- JSON types for FRB ---

/// A swarm event flattened into JSON for the Flutter event stream.
///
/// Only the fields relevant to the event's `event_type` are populated; the
/// rest stay `None` (the constructors all start from `default_event`).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmEventJson {
    /// Event kind, e.g. `dm`, `broadcast`, `receipt`, `call_*`.
    pub event_type: String,
    /// Peer the event concerns, when applicable.
    pub peer_id: Option<String>,
    /// Event-specific payload (group name, user ID, call ID, NAT status…).
    pub content: Option<String>,
    /// Round-trip time of a live message, in canonical display form.
    pub latency: Option<String>,
    /// Nickname announced by the peer, when applicable.
    pub nickname: Option<String>,
    /// Message ID of a receipt or delivered message, when applicable.
    pub msg_id: Option<String>,
    /// Immutable reply snapshot on a live chat event (US-006).
    pub reply_to: Option<crate::ReplyRef>,
    /// True when a live DM arrived sealed end to end (US-042).
    pub encrypted: bool,
    /// Multiaddr of a listen/external-address/discovery event.
    pub address: Option<String>,
    /// Group topic of a group message/invite event.
    pub group_id: Option<String>,
    /// Raw device public key of a device-binding intro.
    pub device_pubkey: Option<Vec<u8>>,
    /// Decoded 16 kHz mono PCM from a `call_media` event (calls lane, US-044).
    pub audio: Option<Vec<i16>>,
    /// Notification built in Rust for incoming messages (US-027/US-028).
    /// `None` when both channels are off or the event is not a user message.
    pub notification: Option<crate::notifications::Notification>,
}

/// Build an event envelope with only `event_type` populated, so match arms
/// only need to set their event-specific fields via struct update syntax.
fn event_with_type(event_type: &str) -> SwarmEventJson {
    SwarmEventJson {
        event_type: event_type.into(),
        ..default_event()
    }
}

fn event_to_json(ev: SwarmEvent) -> SwarmEventJson {
    match ev {
        SwarmEvent::GroupMessage(m) => group_message_event_json(m),
        SwarmEvent::GroupInvite {
            group_id,
            inviter_peer,
        } => group_invite_event_json(group_id, inviter_peer),
        SwarmEvent::BroadcastMessage(m) => message_event_json("broadcast", m),
        SwarmEvent::DirectMessage(m) => message_event_json("dm", m),
        SwarmEvent::Typing { peer_id, is_typing } => typing_event_json(peer_id, is_typing),
        SwarmEvent::Reaction { msg_id } => SwarmEventJson {
            msg_id: Some(msg_id),
            ..event_with_type("reaction")
        },
        SwarmEvent::PeerConnected(id) => SwarmEventJson {
            peer_id: Some(id),
            ..event_with_type("peer_connected")
        },
        SwarmEvent::PeerDisconnected(id) => SwarmEventJson {
            peer_id: Some(id),
            ..event_with_type("peer_disconnected")
        },
        SwarmEvent::ListenAddrEstablished(addr) => SwarmEventJson {
            address: Some(addr),
            ..event_with_type("listen_addr")
        },
        SwarmEvent::ExternalAddressAdded(addr) => SwarmEventJson {
            address: Some(addr),
            ..event_with_type("external_address")
        },
        #[cfg(feature = "nat")]
        SwarmEvent::NatStatusChanged(status) => SwarmEventJson {
            content: Some(status),
            ..event_with_type("nat_status")
        },
        #[cfg(feature = "nat")]
        SwarmEvent::HolePunchResult { peer_id, success } => hole_punch_event_json(peer_id, success),
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => SwarmEventJson {
            peer_id: Some(peer_id),
            msg_id: Some(ack_for),
            ..event_with_type("receipt")
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, addresses } => SwarmEventJson {
            peer_id: Some(peer_id),
            address: addresses.first().map(ToString::to_string),
            ..event_with_type("peer_discovered")
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => SwarmEventJson {
            peer_id: Some(peer_id),
            ..event_with_type("peer_expired")
        },
        SwarmEvent::DeviceBindingIntro {
            peer_id,
            device_pubkey,
        } => SwarmEventJson {
            peer_id: Some(peer_id),
            device_pubkey: Some(device_pubkey),
            ..event_with_type("device_binding_intro")
        },
        SwarmEvent::DeviceBindingReceived { user_id } => SwarmEventJson {
            content: Some(user_id),
            ..event_with_type("device_binding_received")
        },
        // Call-signaling events (calls lane). `content` carries the
        // end-to-end `call_id`; Dart keys its call screen/rings on
        // `event_type` + `peer_id` + the echoed `call_id`.
        SwarmEvent::CallInvite { call_id, peer_id } => {
            call_event_json("call_invite", call_id, peer_id)
        }
        SwarmEvent::CallAccepted { call_id, peer_id } => {
            call_event_json("call_accepted", call_id, peer_id)
        }
        SwarmEvent::CallRejected { call_id, peer_id } => {
            call_event_json("call_rejected", call_id, peer_id)
        }
        SwarmEvent::CallEnded { call_id, peer_id } => {
            call_event_json("call_ended", call_id, peer_id)
        }
        // Decoded voice frames (calls lane, US-044). `content` is the call id,
        // `peer_id` the speaker and `audio` the PCM for the local adapter.
        SwarmEvent::CallMediaFrame {
            call_id,
            peer_id,
            pcm,
        } => SwarmEventJson {
            peer_id: Some(peer_id),
            content: Some(call_id),
            audio: Some(pcm),
            ..event_with_type("call_media")
        },
        // An attachment finished downloading (files lane §12). `content` is the
        // content address and `nickname` the original filename.
        SwarmEvent::FileReceived {
            peer_id,
            file_id,
            name,
            path: _,
        } => SwarmEventJson {
            peer_id: Some(peer_id),
            content: Some(file_id),
            nickname: Some(name),
            ..event_with_type("file_received")
        },
    }
}

/// JSON envelope for a DM typing signal: `content` is `"start"`/`"stop"`.
fn typing_event_json(peer_id: String, is_typing: bool) -> SwarmEventJson {
    SwarmEventJson {
        event_type: "typing".into(),
        peer_id: Some(peer_id),
        content: Some(if is_typing { "start" } else { "stop" }.to_string()),
        ..default_event()
    }
}

/// JSON envelope for a `DCUtR` hole-punch result: `content` is `"ok"`/`"failed"`.
#[cfg(feature = "nat")]
fn hole_punch_event_json(peer_id: String, success: bool) -> SwarmEventJson {
    SwarmEventJson {
        event_type: "hole_punch".into(),
        peer_id: Some(peer_id),
        content: Some(if success { "ok" } else { "failed" }.to_string()),
        ..default_event()
    }
}

/// JSON envelope shared by all four call-signaling events: the `event_type`
/// distinguishes them, `content` carries the echoed `call_id`, and `peer_id`
/// names the other side.
fn call_event_json(event_type: &str, call_id: String, peer_id: String) -> SwarmEventJson {
    SwarmEventJson {
        event_type: event_type.to_string(),
        peer_id: Some(peer_id),
        content: Some(call_id),
        ..default_event()
    }
}

const fn default_event() -> SwarmEventJson {
    SwarmEventJson {
        event_type: String::new(),
        peer_id: None,
        content: None,
        latency: None,
        nickname: None,
        msg_id: None,
        reply_to: None,
        encrypted: false,
        address: None,
        group_id: None,
        device_pubkey: None,
        audio: None,
        notification: None,
    }
}

/// JSON envelope for broadcast/DM events (the two text-message kinds the Dart
/// chat surfaces persist); the `event_type` distinguishes them.
fn message_event_json(event_type: &str, m: crate::MessageEvent) -> SwarmEventJson {
    let notification = crate::notifications::for_message(
        event_type,
        &m.peer_id,
        m.nickname.as_deref(),
        &m.content,
        None,
    );
    SwarmEventJson {
        event_type: event_type.to_string(),
        peer_id: Some(m.peer_id),
        content: Some(m.content),
        latency: m.latency,
        nickname: m.nickname,
        msg_id: m.msg_id,
        reply_to: m.reply_to,
        encrypted: m.encrypted,
        address: None,
        group_id: None,
        device_pubkey: None,
        audio: None,
        notification,
    }
}

/// JSON envelope for group chat events; `group_id` names the room.
fn group_message_event_json(m: crate::GroupMessageEvent) -> SwarmEventJson {
    let notification = crate::notifications::for_message(
        "group_message",
        &m.peer_id,
        m.nickname.as_deref(),
        &m.content,
        Some(&m.group_id),
    );
    SwarmEventJson {
        event_type: "group_message".into(),
        peer_id: Some(m.peer_id),
        content: Some(m.content),
        nickname: m.nickname,
        msg_id: m.msg_id,
        reply_to: m.reply_to,
        group_id: Some(m.group_id),
        notification,
        ..default_event()
    }
}

/// JSON envelope for group invite events.
fn group_invite_event_json(group_id: String, inviter_peer: String) -> SwarmEventJson {
    SwarmEventJson {
        event_type: "group_invite".into(),
        peer_id: Some(inviter_peer),
        group_id: Some(group_id),
        ..default_event()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::models_queryable::Message;
    use crate::types::MessageEvent;
    use chrono::NaiveDateTime;
    use serial_test::serial;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    // ── event_to_json ──────────────────────────────────────────────────

    #[test]
    fn test_event_to_json_broadcast() {
        let ev = SwarmEvent::BroadcastMessage(MessageEvent {
            content: "hello".into(),
            peer_id: "p1".into(),
            latency: Some("5ms".into()),
            nickname: Some("Alice".into()),
            msg_id: Some("m1".into()),
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        let j = event_to_json(ev);
        assert_eq!(j.event_type, "broadcast");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.content.as_deref(), Some("hello"));
        assert_eq!(j.latency.as_deref(), Some("5ms"));
        assert_eq!(j.nickname.as_deref(), Some("Alice"));
        assert_eq!(j.msg_id.as_deref(), Some("m1"));
        assert!(j.address.is_none());
    }

    #[test]
    fn test_event_to_json_direct_message() {
        let ev = SwarmEvent::DirectMessage(MessageEvent {
            content: "secret".into(),
            peer_id: "p2".into(),
            latency: None,
            nickname: None,
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        let j = event_to_json(ev);
        assert_eq!(j.event_type, "dm");
        assert_eq!(j.peer_id.as_deref(), Some("p2"));
        assert_eq!(j.content.as_deref(), Some("secret"));
        assert!(j.latency.is_none());
        assert!(j.nickname.is_none());
    }

    #[test]
    fn test_event_to_json_peer_connected() {
        let j = event_to_json(SwarmEvent::PeerConnected("p1".into()));
        assert_eq!(j.event_type, "peer_connected");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert!(j.content.is_none());
    }

    #[test]
    fn test_event_to_json_peer_disconnected() {
        let j = event_to_json(SwarmEvent::PeerDisconnected("p3".into()));
        assert_eq!(j.event_type, "peer_disconnected");
        assert_eq!(j.peer_id.as_deref(), Some("p3"));
    }

    #[test]
    fn test_event_to_json_listen_addr() {
        let j = event_to_json(SwarmEvent::ListenAddrEstablished(
            "/ip4/0.0.0.0/tcp/4000".into(),
        ));
        assert_eq!(j.event_type, "listen_addr");
        assert_eq!(j.address.as_deref(), Some("/ip4/0.0.0.0/tcp/4000"));
    }

    #[test]
    fn test_event_to_json_receipt() {
        let j = event_to_json(SwarmEvent::Receipt {
            peer_id: "p1".into(),
            ack_for: "msg-42".into(),
            received_at: Some(1000.0),
        });
        assert_eq!(j.event_type, "receipt");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.msg_id.as_deref(), Some("msg-42"));
    }

    #[test]
    fn test_event_to_json_typing() {
        for (is_typing, expected) in [(true, "start"), (false, "stop")] {
            let j = event_to_json(SwarmEvent::Typing {
                peer_id: "p9".into(),
                is_typing,
            });
            assert_eq!(j.event_type, "typing");
            assert_eq!(j.peer_id.as_deref(), Some("p9"));
            assert_eq!(j.content.as_deref(), Some(expected));
        }
    }

    #[cfg(feature = "mdns")]
    #[test]
    fn test_event_to_json_peer_discovered() {
        let addr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/9000".parse().unwrap();
        let j = event_to_json(SwarmEvent::PeerDiscovered {
            peer_id: "p5".into(),
            addresses: vec![addr],
        });
        assert_eq!(j.event_type, "peer_discovered");
        assert_eq!(j.peer_id.as_deref(), Some("p5"));
        assert!(j.address.is_some());
    }

    #[cfg(feature = "mdns")]
    #[test]
    fn test_event_to_json_peer_expired() {
        let j = event_to_json(SwarmEvent::PeerExpired {
            peer_id: "p6".into(),
        });
        assert_eq!(j.event_type, "peer_expired");
        assert_eq!(j.peer_id.as_deref(), Some("p6"));
    }

    #[test]
    fn test_event_to_json_call_invite() {
        let j = event_to_json(SwarmEvent::CallInvite {
            call_id: "c1".into(),
            peer_id: "p1".into(),
        });
        assert_eq!(j.event_type, "call_invite");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.content.as_deref(), Some("c1"));
    }

    #[test]
    fn test_event_to_json_call_accepted() {
        let j = event_to_json(SwarmEvent::CallAccepted {
            call_id: "c1".into(),
            peer_id: "p1".into(),
        });
        assert_eq!(j.event_type, "call_accepted");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.content.as_deref(), Some("c1"));
    }

    #[test]
    fn test_event_to_json_call_rejected() {
        let j = event_to_json(SwarmEvent::CallRejected {
            call_id: "c1".into(),
            peer_id: "p1".into(),
        });
        assert_eq!(j.event_type, "call_rejected");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.content.as_deref(), Some("c1"));
    }

    #[test]
    fn test_event_to_json_call_ended() {
        let j = event_to_json(SwarmEvent::CallEnded {
            call_id: "c1".into(),
            peer_id: "p1".into(),
        });
        assert_eq!(j.event_type, "call_ended");
        assert_eq!(j.peer_id.as_deref(), Some("p1"));
        assert_eq!(j.content.as_deref(), Some("c1"));
    }

    // ── default_event ──────────────────────────────────────────────────

    #[test]
    fn test_default_event_fields() {
        let j = default_event();
        assert_eq!(j.event_type, "");
        assert!(j.peer_id.is_none());
        assert!(j.content.is_none());
        assert!(j.latency.is_none());
        assert!(j.nickname.is_none());
        assert!(j.msg_id.is_none());
        assert!(j.address.is_none());
    }

    // ── message_to_chat ────────────────────────────────────────────────

    #[test]
    fn test_message_to_chat_broadcast() {
        let msg = Message {
            id: 1,
            created_at: dt("2024-06-15 10:30:00"),
            content: "hello world".into(),
            peer_id: None,
            topic: "chat".into(),
            sent: 1,
            is_direct: 0,
            target_peer: None,
            msg_id: Some("msg-1".into()),
            sent_at: Some(1_718_455_800.0),
            sender_nickname: Some("Alice".into()),
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.id, 1);
        assert_eq!(chat.content, "hello world");
        assert!(chat.peer_id.is_none());
        assert!(chat.origin.is_broadcast);
        assert!(chat.origin.sent);
        assert_eq!(chat.msg_id.as_deref(), Some("msg-1"));
        assert!(chat.sent_at.is_some());
        assert_eq!(chat.sender_nickname.as_deref(), Some("Alice"));
    }

    #[test]
    #[serial(db)]
    fn test_message_to_chat_dm() {
        // Isolate DB state so the resolved display name is deterministic and
        // not contaminated by other tests running in parallel.
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_url = dir
            .path()
            .join("msg_to_chat.sqlite")
            .to_string_lossy()
            .into_owned();
        crate::reset_db_url();
        crate::db::set_db_url(&db_url);
        crate::init_database().expect("init db");

        let msg = Message {
            id: 2,
            created_at: dt("2024-06-15 11:00:00"),
            content: "secret msg".into(),
            peer_id: Some("peer-bob".into()),
            topic: "chat".into(),
            sent: 0,
            is_direct: 1,
            target_peer: Some("peer-bob".into()),
            msg_id: None,
            sent_at: None,
            sender_nickname: None,
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.id, 2);
        assert!(!chat.origin.is_broadcast);
        assert!(!chat.origin.sent);
        assert!(chat.sent_at.is_none());
        // No announced nickname: falls back to the resolved display name
        // (petname + short-id suffix), not a raw ID fragment.
        let expected = crate::get_peer_display_name("peer-bob").expect("resolved name");
        assert_eq!(chat.sender_nickname.as_deref(), Some(expected.as_str()));
        assert_eq!(chat.target_peer.as_deref(), Some("peer-bob"));
    }

    #[test]
    #[serial(db)]
    fn test_message_to_chat_announced_nickname_gets_suffix() {
        // Announced nickname + peer id => "Nick (short-id)" consistently.
        let msg = Message {
            id: 10,
            created_at: dt("2024-02-02 12:00:00"),
            content: "hello".into(),
            peer_id: Some("peer-bob".into()),
            topic: "chat".into(),
            sent: 0,
            is_direct: 0,
            target_peer: None,
            msg_id: None,
            sent_at: None,
            sender_nickname: Some("Bob".into()),
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        let suffix = crate::fmt::peer_id_suffix("peer-bob");
        assert_eq!(
            chat.sender_nickname.as_deref(),
            Some(format!("Bob ({suffix})").as_str())
        );
        assert!(chat.origin.is_broadcast);
    }

    #[test]
    #[serial(db)]
    fn test_message_to_chat_own_message_uses_nickname() {
        // No peer id (own message) => the announced nickname is used verbatim.
        let msg = Message {
            id: 11,
            created_at: dt("2024-03-03 09:00:00"),
            content: "mine".into(),
            peer_id: None,
            topic: "chat".into(),
            sent: 1,
            is_direct: 0,
            target_peer: None,
            msg_id: None,
            sent_at: None,
            sender_nickname: Some("Me".into()),
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.sender_nickname.as_deref(), Some("Me"));
        assert!(chat.origin.sent);
    }

    #[test]
    fn test_message_to_chat_empty_content() {
        let msg = Message {
            id: 3,
            created_at: dt("2024-01-01 00:00:00"),
            content: String::new(),
            peer_id: None,
            topic: "chat".into(),
            sent: 0,
            is_direct: 0,
            target_peer: None,
            msg_id: None,
            sent_at: None,
            sender_nickname: None,
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.content, "");
    }

    #[test]
    fn test_message_to_chat_none_sent_at() {
        let msg = Message {
            id: 4,
            created_at: dt("2024-03-01 00:00:00"),
            content: "test".into(),
            peer_id: Some("p1".into()),
            topic: "t".into(),
            sent: 0,
            is_direct: 0,
            target_peer: None,
            msg_id: Some("m4".into()),
            sent_at: None,
            sender_nickname: None,
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        assert!(chat.sent_at.is_none());
    }

    // ── process_event_for_mobile ───────────────────────────────────────

    #[test]
    fn test_process_event_stores_nickname_from_dm() {
        let ev = SwarmEvent::DirectMessage(MessageEvent {
            content: String::new(),
            peer_id: "peer-nick".into(),
            latency: None,
            nickname: Some("Bob".into()),
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        let _ = process_event_for_mobile(&ev);
    }

    #[test]
    fn test_process_event_broadcast_with_nickname() {
        let ev = SwarmEvent::BroadcastMessage(MessageEvent {
            content: "hi all".into(),
            peer_id: "peer-broad".into(),
            latency: None,
            nickname: Some("Charlie".into()),
            msg_id: Some("msg-b1".into()),
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        let _ = process_event_for_mobile(&ev);
    }

    #[test]
    fn test_process_event_logs_file_received() {
        // Unique markers avoid clashing with other tests writing to the shared
        // in-memory log buffer in parallel.
        let _ = process_event_for_mobile(&SwarmEvent::FileReceived {
            peer_id: "peer-file-log".into(),
            file_id: "file-id-log-marker".into(),
            name: "log-marker.bin".into(),
            path: "/tmp/log-marker.bin".into(),
        });
        let logs = crate::logging::get_tui_logs();
        assert!(
            logs.iter().any(|l| l.contains(
                "Received file log-marker.bin (file-id-log-marker) from peer-file-log at /tmp/log-marker.bin"
            )),
            "file_received must land in the shared log buffer: {logs:?}"
        );
    }

    #[cfg(feature = "mdns")]
    #[test]
    fn test_process_event_logs_peer_expired() {
        let _ = process_event_for_mobile(&SwarmEvent::PeerExpired {
            peer_id: "peer-expired-log-marker".into(),
        });
        let logs = crate::logging::get_tui_logs();
        assert!(
            logs.iter()
                .any(|l| l.contains("Peer expired: peer-expired-log-marker")),
            "peer_expired must land in the shared log buffer: {logs:?}"
        );
    }

    #[test]
    fn test_process_event_peer_connected_returns_greeting() {
        let cmd = process_event_for_mobile(&SwarmEvent::PeerConnected("p1".into()));
        match cmd {
            Some(SwarmCommand::SendDm { peer_id, .. }) => assert_eq!(peer_id, "p1"),
            other => panic!("PeerConnected must return a nickname greeting, got {other:?}"),
        }
    }

    #[test]
    fn test_process_event_ignored_variants() {
        assert!(process_event_for_mobile(&SwarmEvent::PeerDisconnected("p1".into())).is_none());
        assert!(
            process_event_for_mobile(&SwarmEvent::ListenAddrEstablished(
                "/ip4/0.0.0.0/tcp/0".into()
            ))
            .is_none()
        );
        assert!(
            process_event_for_mobile(&SwarmEvent::Receipt {
                peer_id: "p1".into(),
                ack_for: "m1".into(),
                received_at: None,
            })
            .is_none()
        );
    }

    #[cfg(feature = "mdns")]
    #[test]
    fn test_process_event_peer_discovered() {
        let addr: libp2p::Multiaddr = "/ip4/10.0.0.1/tcp/9000".parse().unwrap();
        let ev = SwarmEvent::PeerDiscovered {
            peer_id: "p-disc".into(),
            addresses: vec![addr],
        };
        let _ = process_event_for_mobile(&ev);
    }

    // ── forward_swarm_events ──────────────────────────────────────────

    #[tokio::test]
    #[serial(db)]
    async fn test_forward_swarm_events_sends_greeting_without_blocking() {
        // Regression: the forwarder runs *inside* a Tokio task. The old
        // implementation called `blocking_send` for the nickname greeting,
        // which panics ("Cannot block the current thread from within a
        // runtime") and killed the whole event pipeline on first connect.
        let guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("forwarder_test.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        let _ = crate::init_database();
        // `#[serial(db)]` holds the DB namespace across the whole async test;
        // drop the raw guard so it is not held across the awaits below. The
        // forwarder's DB writes are best-effort and not asserted here anyway.
        drop(guard);

        let (event_tx, event_rx) = mpsc::channel(4);
        let (cmd_tx, mut cmd_rx) = mpsc::channel(4);
        let (broadcast_tx, mut broadcast_rx) = tokio::sync::broadcast::channel(4);

        let task = tokio::spawn(forward_swarm_events(event_rx, cmd_tx, broadcast_tx));

        event_tx
            .send(SwarmEvent::PeerConnected("forwarder-peer".into()))
            .await
            .expect("feed event");
        drop(event_tx);

        // The greeting must arrive on the async command channel.
        let cmd = cmd_rx.recv().await.expect("greeting delivered");
        match cmd {
            SwarmCommand::SendDm { peer_id, .. } => assert_eq!(peer_id, "forwarder-peer"),
            other => panic!("expected nickname greeting, got {other:?}"),
        }
        // The same event is fanned out to broadcast consumers.
        let forwarded = broadcast_rx.recv().await.expect("event broadcast");
        assert!(matches!(forwarded, SwarmEvent::PeerConnected(id) if id == "forwarder-peer"));

        task.await.expect("forwarder task completes cleanly");
    }

    // ── is_nickname_only_dm ───────────────────────────────────────────

    #[test]
    fn test_is_nickname_only_dm_empty_content_with_nickname() {
        let ev = SwarmEvent::DirectMessage(MessageEvent {
            content: String::new(),
            peer_id: "p1".into(),
            latency: None,
            nickname: Some("Alice".into()),
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        assert!(is_nickname_only_dm(&ev));
    }

    #[test]
    fn test_is_nickname_only_dm_empty_without_nickname_is_regular_dm() {
        let ev = SwarmEvent::DirectMessage(MessageEvent {
            content: String::new(),
            peer_id: "p1".into(),
            latency: None,
            nickname: None,
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        assert!(!is_nickname_only_dm(&ev));
    }

    #[test]
    fn test_is_nickname_only_dm_real_content_with_nickname_is_regular_dm() {
        let ev = SwarmEvent::DirectMessage(MessageEvent {
            content: "hello".into(),
            peer_id: "p1".into(),
            latency: None,
            nickname: Some("Alice".into()),
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        assert!(!is_nickname_only_dm(&ev));
    }

    #[test]
    fn test_is_nickname_only_dm_ignores_empty_broadcasts() {
        let ev = SwarmEvent::BroadcastMessage(MessageEvent {
            content: String::new(),
            peer_id: "p1".into(),
            latency: None,
            nickname: Some("Alice".into()),
            msg_id: None,
            attachment: None,
            reply_to: None,
            encrypted: false,
        });
        assert!(!is_nickname_only_dm(&ev));
    }

    // ── SwarmEventJson / ChatMessage / MobilePeerRecord ────────────────

    #[test]
    fn test_swarm_event_json_clone_debug() {
        let j = default_event();
        let cloned = j.clone();
        assert_eq!(cloned.event_type, "");
        let _ = format!("{j:?}");
    }

    #[test]
    fn test_chat_message_clone_debug() {
        let msg = Message {
            id: 1,
            created_at: dt("2024-01-01 00:00:00"),
            content: "test".into(),
            peer_id: None,
            topic: "t".into(),
            sent: 1,
            is_direct: 0,
            target_peer: None,
            msg_id: None,
            sent_at: None,
            sender_nickname: None,
            attachment_json: None,
            reply_to_json: None,
            encrypted: 0,
            deleted_at: None,
            edited_at: None,
        };
        let chat = message_to_chat(msg);
        let cloned = chat.clone();
        assert_eq!(cloned.id, 1);
        let _ = format!("{chat:?}");
    }

    #[test]
    fn test_mobile_peer_record_clone_debug() {
        let r = MobilePeerRecord {
            peer_id: "p1".into(),
            first_seen: "2024-01-01".into(),
            last_seen: "2024-06-01".into(),
            nickname: Some("Alice".into()),
            local_nickname: None,
            display_name: "Alice".into(),
        };
        let cloned = r.clone();
        assert_eq!(cloned.peer_id, "p1");
        let _ = format!("{r:?}");
    }

    // ── start/stop node error paths ────────────────────────────────────

    #[test]
    fn test_stop_node_without_start_is_noop() {
        // stop_node should not panic even if NODE was never set
        // (it uses OnceLock::get which returns None)
        let _ = stop_node();
    }

    #[test]
    fn test_get_node_peer_id_without_start_fails() {
        let result = get_node_peer_id();
        assert!(result.is_err());
    }

    #[test]
    fn test_poll_event_without_start_fails() {
        let result = poll_event();
        assert!(result.is_err());
    }

    #[test]
    #[serial(db)]
    fn test_load_broadcast_messages_empty_db() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("bc_test.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        let _ = crate::init_database();

        let msgs = load_broadcast_messages(100).expect("load broadcast");
        assert!(msgs.is_empty());
    }

    #[test]
    #[serial(db)]
    fn test_load_dm_messages_empty_db() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dm_test.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        let _ = crate::init_database();

        let msgs = load_dm_messages("peer1".into(), 100).expect("load dm");
        assert!(msgs.is_empty());
    }

    // ── confirmed (delivery receipt) flags ─────────────────────────────

    fn save_outgoing_with_msg_id(
        content: &str,
        msg_id: &str,
        is_direct: bool,
        target: Option<&str>,
    ) {
        let saved = save_message_with_meta(
            content,
            None,
            CHAT_TOPIC,
            is_direct,
            target,
            MessageMeta {
                msg_id: Some(msg_id.to_string()),
                sent_at: Some(1.0),
                ..MessageMeta::default()
            },
        )
        .expect("save message");
        mark_message_sent(saved.id).expect("mark sent");
    }

    #[test]
    #[serial(db)]
    fn test_load_broadcast_messages_marks_confirmed_receipts() {
        let _guard = crate::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("bc_confirmed.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        crate::init_database().expect("init db");

        // Two outgoing broadcasts; only the first gets a receipt. An incoming
        // row carries a receipt id too, and another echoes the *same* id as the
        // acknowledged outgoing message: neither ever shows a delivery tick.
        save_outgoing_with_msg_id("acked", "msg-acked", false, None);
        save_outgoing_with_msg_id("unacked", "msg-unacked", false, None);
        let inbound = save_message_with_meta(
            "in",
            Some("peer-a"),
            CHAT_TOPIC,
            false,
            None,
            MessageMeta {
                msg_id: Some("msg-in".to_string()),
                sent_at: Some(3.0),
                ..MessageMeta::default()
            },
        )
        .expect("save inbound");
        let echoed = save_message_with_meta(
            "echo",
            Some("peer-a"),
            CHAT_TOPIC,
            false,
            None,
            MessageMeta {
                msg_id: Some("msg-acked".to_string()),
                sent_at: Some(4.0),
                ..MessageMeta::default()
            },
        )
        .expect("save echoed inbound");

        crate::messages::save_receipt("msg-acked", "peer-a", 0, 5.0).expect("receipt");
        crate::messages::save_receipt("msg-in", "peer-a", 0, 6.0).expect("inbound receipt");

        let msgs = load_broadcast_messages(100).expect("load broadcast");
        assert_eq!(msgs.len(), 4);
        let by_id = |id: &str, sent: bool| {
            msgs.iter()
                .find(|m| m.msg_id.as_deref() == Some(id) && m.origin.sent == sent)
                .expect("message present")
        };
        assert!(
            by_id("msg-acked", true).confirmed,
            "acked message shows a tick"
        );
        assert!(
            !by_id("msg-unacked", true).confirmed,
            "unacked message has no tick"
        );
        assert!(
            !by_id("msg-in", false).confirmed,
            "inbound message never ticks"
        );
        assert!(
            !by_id("msg-acked", false).confirmed,
            "an inbound echo of our wire id never ticks"
        );
        assert_eq!(inbound.sent, 0);
        assert_eq!(echoed.sent, 0);
    }

    #[test]
    #[serial(db)]
    fn test_load_dm_messages_marks_confirmed_receipts() {
        let _guard = crate::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("dm_confirmed.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        crate::init_database().expect("init db");

        save_outgoing_with_msg_id("hi", "dm-acked", true, Some("peer-b"));
        crate::messages::save_receipt("dm-acked", "peer-b", 1, 7.0).expect("receipt");

        let msgs = load_dm_messages("peer-b".into(), 100).expect("load dm");
        let dm = msgs
            .iter()
            .find(|m| m.msg_id.as_deref() == Some("dm-acked"))
            .expect("dm present");
        assert!(dm.confirmed, "acknowledged DM shows a tick");
    }
}
