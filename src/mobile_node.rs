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
use tokio::sync::mpsc::error::TryRecvError;

static NODE: OnceLock<Mutex<MobileNode>> = OnceLock::new();

/// Live set of peers currently connected to this node. Used to attribute
/// broadcasts we send to the peers that were online to receive them, and to
/// remember the most recent disconnection. Shared with the TUI via
/// [`crate::connected::ConnectedTracker`] so both frontends agree on what
/// counts as "currently connected".
static CONNECTED_PEERS: LazyLock<Mutex<crate::connected::ConnectedTracker>> =
    LazyLock::new(|| Mutex::new(crate::connected::ConnectedTracker::new()));

struct MobileNode {
    event_rx: Option<mpsc::Receiver<SwarmEvent>>,
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

    let new_node = MobileNode {
        event_rx: Some(event_rx),
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
        node.event_rx.take();
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
    // Clone the sender once so the loop body can call
    // `process_event_for_mobile` (a stateful routine that must never run while
    // the node guard is held) without keeping the lock across iterations.
    let cmd_tx = {
        let node = lock_node_mutex(m);
        node.cmd_tx.clone()
    };

    loop {
        let ev = {
            let mut node = lock_node_mutex(m);
            match node.event_rx.as_mut().ok_or("Node stopped")?.try_recv() {
                Ok(ev) => ev,
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => return Err("Swarm task ended".into()),
            }
        };

        // Process events like the TUI does (save peers, exchange nicknames)
        process_event_for_mobile(&ev, cmd_tx.as_ref());
        // A nickname-only DM is the name-exchange side channel, not a
        // message: its side effects (storing the announced nickname)
        // were already applied above, so skip it before it can reach
        // Dart — whose `content != null` guard would happily persist an
        // empty-content DM. Keep draining so a run of pure name
        // exchanges cannot stall the queue.
        if is_nickname_only_dm(&ev) {
            continue;
        }
        return Ok(Some(event_to_json(ev)));
    }
}

/// True for `DirectMessage` events whose empty content carries only a
/// nickname. These are the outgoing nickname-exchange DM we send on connect
/// (echoed back by the peer) and similar name-only messages: the nickname is
/// a peer-record update (handled by [`process_event_for_mobile`]), never a
/// chat message to persist or surface in the UI.
const fn is_nickname_only_dm(ev: &SwarmEvent) -> bool {
    matches!(
        ev,
        SwarmEvent::DirectMessage(m)
            if m.content.is_empty() && m.nickname.is_some() && m.attachment.is_none()
    )
}

/// Process a swarm event for side effects: save peers, send nickname, store received nicknames.
fn process_event_for_mobile(ev: &SwarmEvent, cmd_tx: Option<&mpsc::Sender<SwarmCommand>>) {
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
            // Send our nickname to the peer (nickname exchange)
            if let Some(tx) = cmd_tx {
                let nickname = get_self_nickname().ok().flatten();
                let msg_id = gen_msg_id();
                let _ = tx.blocking_send(SwarmCommand::SendDm {
                    peer_id: peer_id.clone(),
                    content: String::new(),
                    nickname,
                    msg_id: Some(msg_id),
                    ack_for: None,
                    attachment: None,
                });
            }
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            if let Ok(mut set) = CONNECTED_PEERS.lock() {
                set.on_peer_disconnected(peer_id, crate::current_timestamp());
                // Feed the peer-session history used for `get_network_size()`.
                let _ =
                    crate::peers::record_peer_session(i32::try_from(set.len()).unwrap_or(i32::MAX));
            }
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
        }
        SwarmEvent::GroupMessage(m) => {
            // Store the sender's announced nickname (the group message itself is
            // persisted by the swarm handler).
            if let Some(nick) = &m.nickname
                && !nick.is_empty()
            {
                let _ = record_peer_received_name_change(&m.peer_id, nick);
            }
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
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            // Mirrors the TUI's log line (mDNS's peer-expiry notice).
            p2plog_debug(format!("Peer expired: {peer_id}"));
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, addresses } => {
            let addrs: Vec<String> = addresses.iter().map(ToString::to_string).collect();
            if let Err(e) = save_peer(peer_id, &addrs) {
                p2plog_debug(format!("Failed to save discovered peer: {e}"));
            }
        }
        _ => {}
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobilePeerRecord {
    pub peer_id: String,
    pub first_seen: String,
    pub last_seen: String,
    pub nickname: Option<String>,
    pub local_nickname: Option<String>,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: i32,
    pub content: String,
    pub peer_id: Option<String>,
    pub is_broadcast: bool,
    pub target_peer: Option<String>,
    pub sent: bool,
    pub msg_id: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
    pub sender_nickname: Option<String>,
    pub attachment: Option<MobileAttachment>,
    /// True once a recipient acknowledged this (outgoing) message — a delivery
    /// tick in the UI. Always false for incoming messages and for the value
    /// returned right after sending; [`load_broadcast_messages`] /
    /// [`load_dm_messages`] fill it in from the receipts table.
    pub confirmed: bool,
    /// Round-trip latency of a *live* incoming message (`5ms`, `<1ms`, `1.2s`),
    /// the value the TUI prefixes onto the message line. `None` for outgoing
    /// messages, group messages, and every row read back from the database —
    /// the TUI measures it in memory only and never persists it.
    pub latency: Option<String>,
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
    /// Best-effort MIME type hint (`None` when unknown)
    pub mime: Option<String>,
}

fn mobile_attachment(att: crate::files::MessageAttachment) -> MobileAttachment {
    MobileAttachment {
        file_id: att.file_id,
        name: att.name,
        size: i64::try_from(att.size).unwrap_or(i64::MAX),
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
        is_broadcast: msg.is_direct == 0,
        target_peer: msg.target_peer,
        sent: msg.sent == 1,
        msg_id: msg.msg_id,
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
        confirmed: false,
        latency: None,
    }
}

/// Flag which of these outgoing messages a recipient has acknowledged, using a
/// single batched query over `message_receipts` (see
/// [`crate::messages::confirmed_message_ids`]). A DB failure leaves every flag
/// false and logs, so the history view still renders.
fn mark_confirmed(messages: &mut [ChatMessage]) {
    let ids: Vec<String> = messages
        .iter()
        .filter(|m| m.sent)
        .filter_map(|m| m.msg_id.clone())
        .collect();
    if ids.is_empty() {
        return;
    }
    match crate::messages::confirmed_message_ids(&ids) {
        Ok(confirmed) => {
            for m in messages.iter_mut() {
                m.confirmed = m.msg_id.as_ref().is_some_and(|id| confirmed.contains(id));
            }
        }
        Err(e) => p2plog_debug(format!("Failed to load message receipts: {e}")),
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
) -> Result<ChatMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_broadcast_attached(content, attachment)
}

/// [`save_outgoing_broadcast`] with a pre-stored attachment (files lane §12).
///
/// The web upload route (docs/2026-10-18_web_frontend.md) stores the blob from
/// raw bytes server-side and passes the descriptor straight through, so the
/// path-based step never happens on that flow.
pub(crate) fn save_outgoing_broadcast_attached(
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, false, None, meta)
        .map_err(|e| e.to_string())?;

    // Send via swarm
    let mut queued = false;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            queued = tx
                .blocking_send(SwarmCommand::Publish {
                    content,
                    nickname,
                    msg_id: Some(msg_id.clone()),
                    attachment,
                })
                .is_ok();
        }
    }

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
        .map(|t| t.connected_peer_ids().map(str::to_owned).collect())
        .unwrap_or_default();
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
) -> Result<ChatMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_dm_attached(peer_id, content, attachment)
}

/// [`save_outgoing_dm`] with a pre-stored attachment (files lane §12); see
/// [`save_outgoing_broadcast_attached`].
pub(crate) fn save_outgoing_dm_attached(
    peer_id: String,
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, true, Some(&peer_id), meta)
        .map_err(|e| e.to_string())?;

    // Send via swarm
    let mut queued = false;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            queued = tx
                .blocking_send(SwarmCommand::SendDm {
                    peer_id,
                    content,
                    nickname,
                    msg_id: Some(msg_id),
                    ack_for: None,
                    attachment,
                })
                .is_ok();
        }
    }

    let chat = message_to_chat(msg);
    if queued {
        let _ = mark_message_sent(chat.id);
    }
    Ok(chat)
}

/// Save an incoming message (broadcast or DM) to the database.
///
/// `latency` is the measured round-trip time of the live event; it decorates
/// the returned [`ChatMessage`] only (never written to the database), matching
/// the TUI's in-memory rendering.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
    latency: Option<String>,
) -> Result<ChatMessage, String> {
    let target = is_direct.then_some(peer_id.as_str());
    // Ensure the peer row exists (and gets a generated petname if it's a
    // silent peer seen for the first time / before this feature) so the
    // message's sender label can resolve to a name instead of a raw ID.
    if let Err(e) = crate::save_peer(&peer_id, &[]) {
        p2plog_debug(format!("Failed to ensure peer on incoming message: {e}"));
    }
    let meta = MessageMeta {
        sender_nickname: nickname,
        msg_id: None,
        sent_at: None,
        attachment: None,
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
    let mut queued = false;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            queued = tx
                .blocking_send(SwarmCommand::SendDeviceBinding {
                    peer_id,
                    request: Box::new(request),
                })
                .is_ok();
        }
    }
    if !queued {
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
pub fn accept_device_binding(peer_id: String, device_pubkey: &[u8]) -> Result<(), String> {
    let request =
        crate::identity::mint_device_binding(&peer_id, device_pubkey).map_err(|e| e.to_string())?;
    let mut queued = false;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            queued = tx
                .blocking_send(SwarmCommand::SendDeviceBinding {
                    peer_id,
                    request: Box::new(request),
                })
                .is_ok();
        }
    }
    if !queued {
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
    let mut queued = false;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            queued = tx
                .blocking_send(SwarmCommand::SendCallSignal {
                    peer_id,
                    signal: Box::new(crate::behavior::CallSignal::new(&kind, call_id)),
                })
                .is_ok();
        }
    }
    if !queued {
        return Err("Failed to queue call signal: node not running".into());
    }
    Ok(())
}

// --- Group chat types and functions ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileGroup {
    pub group_id: String,
    pub display_name: String,
    pub member_count: i64,
    pub is_private: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileGroupMessage {
    pub id: i32,
    pub content: String,
    pub peer_id: Option<String>,
    pub sent: bool,
    pub msg_id: Option<String>,
    pub sent_at: Option<String>,
    pub created_at: String,
    pub sender_nickname: Option<String>,
    pub attachment: Option<MobileAttachment>,
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
        msg_id: msg.msg_id,
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
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::SubscribeGroup {
                group_id: group_id.clone(),
            });
        }
    }
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
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::SubscribeGroup {
                group_id: group.group_id.clone(),
            });
        }
    }
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
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::UnsubscribeGroup { group_id });
        }
    }
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
    Ok(msgs
        .into_iter()
        .rev()
        .map(group_message_to_mobile)
        .collect())
}

/// Save an outgoing group message, persist to DB, and publish via the swarm.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_outgoing_group(
    group_id: String,
    content: String,
    attachment_path: Option<String>,
) -> Result<MobileGroupMessage, String> {
    let attachment = store_send_attachment(attachment_path)?;
    save_outgoing_group_attached(group_id, content, attachment)
}

/// [`save_outgoing_group`] with a pre-stored attachment (files lane §12); see
/// [`save_outgoing_broadcast_attached`].
pub(crate) fn save_outgoing_group_attached(
    group_id: String,
    content: String,
    attachment: Option<crate::files::MessageAttachment>,
) -> Result<MobileGroupMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();
    let meta = crate::groups::GroupMessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
        attachment: attachment.clone(),
    };
    let msg = crate::groups::save_outgoing_group_message(&group_id, &content, meta)
        .map_err(|e| e.to_string())?;
    if let Some(m) = NODE.get() {
        let node = lock_node_mutex(m);
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::PublishGroup {
                group_id,
                content,
                nickname,
                msg_id: Some(msg_id),
                attachment,
            });
        }
    }
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmEventJson {
    pub event_type: String,
    pub peer_id: Option<String>,
    pub content: Option<String>,
    pub latency: Option<String>,
    pub nickname: Option<String>,
    pub msg_id: Option<String>,
    pub address: Option<String>,
    pub group_id: Option<String>,
    pub device_pubkey: Option<Vec<u8>>,
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
        SwarmEvent::PeerConnected(id) => SwarmEventJson {
            event_type: "peer_connected".into(),
            peer_id: Some(id),
            ..default_event()
        },
        SwarmEvent::PeerDisconnected(id) => SwarmEventJson {
            event_type: "peer_disconnected".into(),
            peer_id: Some(id),
            ..default_event()
        },
        SwarmEvent::ListenAddrEstablished(addr) => SwarmEventJson {
            event_type: "listen_addr".into(),
            address: Some(addr),
            ..default_event()
        },
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => SwarmEventJson {
            event_type: "receipt".into(),
            peer_id: Some(peer_id),
            msg_id: Some(ack_for),
            ..default_event()
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, addresses } => SwarmEventJson {
            event_type: "peer_discovered".into(),
            peer_id: Some(peer_id),
            address: addresses.first().map(ToString::to_string),
            ..default_event()
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => SwarmEventJson {
            event_type: "peer_expired".into(),
            peer_id: Some(peer_id),
            ..default_event()
        },
        SwarmEvent::DeviceBindingIntro {
            peer_id,
            device_pubkey,
        } => SwarmEventJson {
            event_type: "device_binding_intro".into(),
            peer_id: Some(peer_id),
            device_pubkey: Some(device_pubkey),
            ..default_event()
        },
        SwarmEvent::DeviceBindingReceived { user_id } => SwarmEventJson {
            event_type: "device_binding_received".into(),
            content: Some(user_id),
            ..default_event()
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
        // An attachment finished downloading (files lane §12). `content` is the
        // content address and `nickname` the original filename.
        SwarmEvent::FileReceived {
            peer_id,
            file_id,
            name,
            path: _,
        } => SwarmEventJson {
            event_type: "file_received".into(),
            peer_id: Some(peer_id),
            content: Some(file_id),
            nickname: Some(name),
            ..default_event()
        },
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
        address: None,
        group_id: None,
        device_pubkey: None,
    }
}

/// JSON envelope for broadcast/DM events (the two text-message kinds the Dart
/// chat surfaces persist); the `event_type` distinguishes them.
fn message_event_json(event_type: &str, m: crate::MessageEvent) -> SwarmEventJson {
    SwarmEventJson {
        event_type: event_type.to_string(),
        peer_id: Some(m.peer_id),
        content: Some(m.content),
        latency: m.latency,
        nickname: m.nickname,
        msg_id: m.msg_id,
        address: None,
        group_id: None,
        device_pubkey: None,
    }
}

/// JSON envelope for group chat events; `group_id` names the room.
fn group_message_event_json(m: crate::GroupMessageEvent) -> SwarmEventJson {
    SwarmEventJson {
        event_type: "group_message".into(),
        peer_id: Some(m.peer_id),
        content: Some(m.content),
        nickname: m.nickname,
        msg_id: m.msg_id,
        group_id: Some(m.group_id),
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
        assert!(j.event_type.is_empty());
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
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.id, 1);
        assert_eq!(chat.content, "hello world");
        assert!(chat.peer_id.is_none());
        assert!(chat.is_broadcast);
        assert!(chat.sent);
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
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.id, 2);
        assert!(!chat.is_broadcast);
        assert!(!chat.sent);
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
        };
        let chat = message_to_chat(msg);
        let suffix = crate::fmt::peer_id_suffix("peer-bob");
        assert_eq!(
            chat.sender_nickname.as_deref(),
            Some(format!("Bob ({suffix})").as_str())
        );
        assert!(chat.is_broadcast);
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
        };
        let chat = message_to_chat(msg);
        assert_eq!(chat.sender_nickname.as_deref(), Some("Me"));
        assert!(chat.sent);
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
        };
        let chat = message_to_chat(msg);
        assert!(chat.content.is_empty());
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
        });
        process_event_for_mobile(&ev, None);
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
        });
        process_event_for_mobile(&ev, None);
    }

    #[test]
    fn test_process_event_logs_file_received() {
        // Unique markers avoid clashing with other tests writing to the shared
        // in-memory log buffer in parallel.
        process_event_for_mobile(
            &SwarmEvent::FileReceived {
                peer_id: "peer-file-log".into(),
                file_id: "file-id-log-marker".into(),
                name: "log-marker.bin".into(),
                path: "/tmp/log-marker.bin".into(),
            },
            None,
        );
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
        process_event_for_mobile(
            &SwarmEvent::PeerExpired {
                peer_id: "peer-expired-log-marker".into(),
            },
            None,
        );
        let logs = crate::logging::get_tui_logs();
        assert!(
            logs.iter()
                .any(|l| l.contains("Peer expired: peer-expired-log-marker")),
            "peer_expired must land in the shared log buffer: {logs:?}"
        );
    }

    #[test]
    fn test_process_event_peer_connected_no_sender() {
        process_event_for_mobile(&SwarmEvent::PeerConnected("p1".into()), None);
    }

    #[test]
    fn test_process_event_ignored_variants() {
        process_event_for_mobile(&SwarmEvent::PeerDisconnected("p1".into()), None);
        process_event_for_mobile(
            &SwarmEvent::ListenAddrEstablished("/ip4/0.0.0.0/tcp/0".into()),
            None,
        );
        process_event_for_mobile(
            &SwarmEvent::Receipt {
                peer_id: "p1".into(),
                ack_for: "m1".into(),
                received_at: None,
            },
            None,
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
        process_event_for_mobile(&ev, None);
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
        });
        assert!(!is_nickname_only_dm(&ev));
    }

    // ── SwarmEventJson / ChatMessage / MobilePeerRecord ────────────────

    #[test]
    fn test_swarm_event_json_clone_debug() {
        let j = default_event();
        let cloned = j.clone();
        assert!(cloned.event_type.is_empty());
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

        // Two outgoing broadcasts; only the first gets a receipt. A third
        // *incoming* row (never `sent`) carries a receipt id too, but an
        // inbound message never shows a delivery tick on our side.
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

        crate::messages::save_receipt("msg-acked", "peer-a", 0, 5.0).expect("receipt");
        crate::messages::save_receipt("msg-in", "peer-a", 0, 6.0).expect("inbound receipt");

        let msgs = load_broadcast_messages(100).expect("load broadcast");
        assert_eq!(msgs.len(), 3);
        let by_id = |id: &str| {
            msgs.iter()
                .find(|m| m.msg_id.as_deref() == Some(id))
                .expect("message present")
        };
        assert!(by_id("msg-acked").confirmed, "acked message shows a tick");
        assert!(
            !by_id("msg-unacked").confirmed,
            "unacked message has no tick"
        );
        assert!(!by_id("msg-in").confirmed, "inbound message never ticks");
        assert_eq!(inbound.sent, 0);
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
