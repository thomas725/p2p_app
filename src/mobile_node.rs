//! Mobile node: manages the libp2p swarm lifecycle for Android/iOS.
//!
//! The swarm runs on a background tokio task. Commands are sent via MPSC,
//! events are polled by Dart via `poll_event()`.

use crate::messages::MessageMeta;
use crate::types::{SwarmCommand, SwarmEvent};
use crate::{
    build_swarm, current_timestamp, gen_msg_id, get_local_peer_id, get_network_size,
    get_self_nickname, load_direct_messages, load_messages, mark_message_sent, p2plog_debug,
    save_message_with_meta, save_peer, set_peer_received_nickname, spawn_swarm_handler,
    CHAT_TOPIC,
};
use libp2p::gossipsub;
use std::sync::Arc;
use std::sync::{Mutex, OnceLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

static NODE: OnceLock<Mutex<MobileNode>> = OnceLock::new();

struct MobileNode {
    event_rx: Option<mpsc::Receiver<SwarmEvent>>,
    cmd_tx: Option<mpsc::Sender<SwarmCommand>>,
    peer_id: String,
    _runtime: tokio::runtime::Runtime,
}

/// Start the p2p node with an explicit DB path.
/// Returns the local peer ID.
pub fn start_node(db_path: String) -> Result<String, String> {
    start_node_impl(Some(db_path))
}

/// Start the p2p node using automatic DB selection (lock-based, same as TUI).
/// Scans CWD for unlocked .db files, picks the first one, or creates a new one.
/// Returns the local peer ID.
pub fn start_node_auto() -> Result<String, String> {
    start_node_impl(None)
}

fn start_node_impl(db_path: Option<String>) -> Result<String, String> {
    if NODE.get().is_some() {
        let node = NODE.get().unwrap().lock().unwrap();
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
        // No path: use lock-based DB selection (same as TUI/Dioxus)
        crate::init_database().map_err(|e| e.to_string())?;
    }

    // Create a dedicated tokio runtime — FRB may call us from a non-tokio thread,
    // but mDNS/swarm need a reactor.
    let runtime = tokio::runtime::Runtime::new()
        .map_err(|e| format!("Failed to create runtime: {e}"))?;

    // Block on swarm setup: build_swarm (mDNS), listen, subscribe, spawn handler.
    // tokio::spawn inside block_on uses this runtime; keeping it alive keeps the task alive.
    let (event_rx, cmd_tx, peer_id) = runtime.block_on(async {
        let network_size = get_network_size().map_err(|e| e.to_string())?;
        let mut swarm =
            build_swarm(network_size).map_err(|e| format!("Failed to build swarm: {e}"))?;

        let topic = gossipsub::IdentTopic::new(CHAT_TOPIC);
        swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .map_err(|e| format!("Failed to listen: {e}"))?;
        swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&topic)
            .map_err(|e| format!("Failed to subscribe: {e}"))?;

        let (_handle, event_rx, cmd_tx) = spawn_swarm_handler(swarm, CHAT_TOPIC.to_string());

        let pid = get_local_peer_id()
            .map_err(|e| e.to_string())?
            .to_string();

        Ok::<_, String>((event_rx, cmd_tx, pid))
    })?;

    let node = MobileNode {
        event_rx: Some(event_rx),
        cmd_tx: Some(cmd_tx),
        peer_id: peer_id.clone(),
        _runtime: runtime,
    };

    let _ = NODE.set(Mutex::new(node));
    p2plog_debug(format!("Mobile node started: peer_id={peer_id}"));
    Ok(peer_id)
}

/// Stop the p2p node (drops the swarm task).
pub fn stop_node() -> Result<(), String> {
    if let Some(m) = NODE.get() {
        let mut node = m.lock().unwrap();
        node.cmd_tx.take();
        node.event_rx.take();
        p2plog_debug("Mobile node stopped".to_string());
    }
    // Release DB lock so subsequent starts can acquire the same database
    crate::logging::p2plog_debug("[stop_node] releasing database lock".to_string());
    crate::db::release_db_lock();
    Ok(())
}

/// Poll the next swarm event (non-blocking). Returns None if no event ready.
/// Also processes events: saves peers, sends nickname DMs, stores received nicknames.
pub fn poll_event() -> Result<Option<SwarmEventJson>, String> {
    let m = NODE.get().ok_or("Node not started")?;
    let mut node = m.lock().unwrap();
    let rx = node.event_rx.as_mut().ok_or("Node stopped")?;

    match rx.try_recv() {
        Ok(ev) => {
            // Process events like the TUI does (save peers, exchange nicknames)
            process_event_for_mobile(&ev, &node.cmd_tx);
            Ok(Some(event_to_json(ev)))
        }
        Err(TryRecvError::Empty) => Ok(None),
        Err(TryRecvError::Disconnected) => Err("Swarm task ended".into()),
    }
}

/// Process a swarm event for side effects: save peers, send nickname, store received nicknames.
fn process_event_for_mobile(ev: &SwarmEvent, cmd_tx: &Option<mpsc::Sender<SwarmCommand>>) {
    match ev {
        SwarmEvent::PeerConnected(peer_id) => {
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
                });
            }
        }
        SwarmEvent::BroadcastMessage(m) | SwarmEvent::DirectMessage(m) => {
            // Store the sender's announced nickname
            if let Some(nick) = &m.nickname {
                if !nick.is_empty() {
                    let _ = set_peer_received_nickname(&m.peer_id, nick);
                }
            }
            // If DM is empty with a nickname, it's a nickname-only exchange — don't persist as message
            if matches!(ev, SwarmEvent::DirectMessage(_))
                && m.content.is_empty()
                && m.nickname.is_some()
            {
                // Nickname-only DM, already stored above
                return;
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered {
            peer_id,
            addresses,
        } => {
            let addrs: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();
            if let Err(e) = save_peer(peer_id, &addrs) {
                p2plog_debug(format!("Failed to save discovered peer: {e}"));
            }
        }
        _ => {}
    }
}

/// Send a broadcast message to all connected peers.
pub fn send_broadcast(content: String) -> Result<(), String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = m.lock().unwrap();
    let tx = node.cmd_tx.as_ref().ok_or("Node stopped")?;
    let msg_id = Some(crate::gen_msg_id());
    let nickname = crate::get_self_nickname().ok().flatten();
    tx.blocking_send(SwarmCommand::Publish {
        content,
        nickname,
        msg_id,
    })
    .map_err(|e| format!("Send failed: {e}"))
}

/// Send a direct message to a specific peer.
pub fn send_dm(peer_id: String, content: String) -> Result<(), String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = m.lock().unwrap();
    let tx = node.cmd_tx.as_ref().ok_or("Node stopped")?;
    let msg_id = Some(crate::gen_msg_id());
    let nickname = crate::get_self_nickname().ok().flatten();
    tx.blocking_send(SwarmCommand::SendDm {
        peer_id,
        content,
        nickname,
        msg_id,
        ack_for: None,
    })
    .map_err(|e| format!("Send failed: {e}"))
}

/// Get the local peer ID.
pub fn get_node_peer_id() -> Result<String, String> {
    let m = NODE.get().ok_or("Node not started")?;
    let node = m.lock().unwrap();
    Ok(node.peer_id.clone())
}

// --- Mobile peer record with nicknames ---

#[derive(Debug, Clone)]
pub struct MobilePeerRecord {
    pub peer_id: String,
    pub first_seen: String,
    pub last_seen: String,
    pub nickname: Option<String>,
    pub local_nickname: Option<String>,
    pub display_name: String,
}

/// Get all known peers from the database with nickname info.
pub fn get_known_peers() -> Result<Vec<MobilePeerRecord>, String> {
    let known = crate::load_peers().map_err(|e| e.to_string())?;
    Ok(known
        .into_iter()
        .map(|p| {
            let display_name = crate::get_peer_display_name(&p.peer_id)
                .unwrap_or_else(|_| p.peer_id.clone());
            MobilePeerRecord {
                peer_id: p.peer_id,
                first_seen: p.first_seen.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                last_seen: p.last_seen.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                nickname: p.received_nickname,
                local_nickname: p.peer_local_nickname,
                display_name,
            }
        })
        .collect())
}

// --- Message history types ---

#[derive(Debug, Clone)]
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
}

fn message_to_chat(msg: crate::generated::models_queryable::Message) -> ChatMessage {
    ChatMessage {
        id: msg.id,
        content: msg.content,
        peer_id: msg.peer_id,
        is_broadcast: msg.is_direct == 0,
        target_peer: msg.target_peer,
        sent: msg.sent == 1,
        msg_id: msg.msg_id,
        sent_at: msg.sent_at.map(|t| {
            let dt = chrono::DateTime::from_timestamp(t as i64, 0)
                .unwrap_or_default();
            dt.format("%H:%M").to_string()
        }),
        created_at: msg
            .created_at
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
        sender_nickname: msg.sender_nickname,
    }
}

/// Load broadcast messages (newest-first from DB, reversed to chronological).
pub fn load_broadcast_messages(limit: i64) -> Result<Vec<ChatMessage>, String> {
    let msgs = load_messages(CHAT_TOPIC, limit as usize).map_err(|e| e.to_string())?;
    Ok(msgs.into_iter().rev().map(message_to_chat).collect())
}

/// Load DM history with a specific peer (already oldest-first in DB).
pub fn load_dm_messages(peer_id: String, limit: i64) -> Result<Vec<ChatMessage>, String> {
    let msgs =
        load_direct_messages(&peer_id, limit as usize).map_err(|e| e.to_string())?;
    Ok(msgs.into_iter().map(message_to_chat).collect())
}

/// Save an outgoing broadcast message, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
pub fn save_outgoing_broadcast(content: String) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, false, None, meta)
        .map_err(|e| e.to_string())?;

    // Send via swarm
    if let Some(m) = NODE.get() {
        let node = m.lock().unwrap();
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::Publish {
                content,
                nickname,
                msg_id: Some(msg_id),
            });
        }
    }

    let chat = message_to_chat(msg);
    // Mark sent in DB (best-effort)
    let _ = mark_message_sent(chat.id);
    Ok(chat)
}

/// Save an outgoing DM, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
pub fn save_outgoing_dm(peer_id: String, content: String) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
    };
    let msg = save_message_with_meta(
        &content,
        None,
        CHAT_TOPIC,
        true,
        Some(&peer_id),
        meta,
    )
    .map_err(|e| e.to_string())?;

    // Send via swarm
    if let Some(m) = NODE.get() {
        let node = m.lock().unwrap();
        if let Some(tx) = node.cmd_tx.as_ref() {
            let _ = tx.blocking_send(SwarmCommand::SendDm {
                peer_id,
                content,
                nickname,
                msg_id: Some(msg_id),
                ack_for: None,
            });
        }
    }

    let chat = message_to_chat(msg);
    let _ = mark_message_sent(chat.id);
    Ok(chat)
}

/// Save an incoming message (broadcast or DM) to the database.
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
) -> Result<ChatMessage, String> {
    let target = is_direct.then_some(peer_id.as_str());
    let meta = MessageMeta {
        sender_nickname: nickname,
        msg_id: None,
        sent_at: None,
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
    Ok(message_to_chat(msg))
}

// --- JSON types for FRB ---

#[derive(Debug, Clone)]
pub struct SwarmEventJson {
    pub event_type: String,
    pub peer_id: Option<String>,
    pub content: Option<String>,
    pub latency: Option<String>,
    pub nickname: Option<String>,
    pub msg_id: Option<String>,
    pub address: Option<String>,
}

fn event_to_json(ev: SwarmEvent) -> SwarmEventJson {
    match ev {
        SwarmEvent::BroadcastMessage(m) => SwarmEventJson {
            event_type: "broadcast".into(),
            peer_id: Some(m.peer_id),
            content: Some(m.content),
            latency: m.latency,
            nickname: m.nickname,
            msg_id: m.msg_id,
            address: None,
        },
        SwarmEvent::DirectMessage(m) => SwarmEventJson {
            event_type: "dm".into(),
            peer_id: Some(m.peer_id),
            content: Some(m.content),
            latency: m.latency,
            nickname: m.nickname,
            msg_id: m.msg_id,
            address: None,
        },
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
            peer_id,
            ack_for,
            ..
        } => SwarmEventJson {
            event_type: "receipt".into(),
            peer_id: Some(peer_id),
            msg_id: Some(ack_for),
            ..default_event()
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered {
            peer_id,
            addresses,
        } => SwarmEventJson {
            event_type: "peer_discovered".into(),
            peer_id: Some(peer_id),
            address: addresses.first().map(|a| a.to_string()),
            ..default_event()
        },
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => SwarmEventJson {
            event_type: "peer_expired".into(),
            peer_id: Some(peer_id),
            ..default_event()
        },
    }
}

fn default_event() -> SwarmEventJson {
    SwarmEventJson {
        event_type: String::new(),
        peer_id: None,
        content: None,
        latency: None,
        nickname: None,
        msg_id: None,
        address: None,
    }
}
