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
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::{LazyLock, Mutex, OnceLock};
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TryRecvError;

static NODE: OnceLock<Mutex<MobileNode>> = OnceLock::new();

/// Live set of peers currently connected to this node. Used to attribute
/// broadcasts we send to the peers that were online to receive them.
static CONNECTED_PEERS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

struct MobileNode {
    event_rx: Option<mpsc::Receiver<SwarmEvent>>,
    cmd_tx: Option<mpsc::Sender<SwarmCommand>>,
    peer_id: String,
    _runtime: tokio::runtime::Runtime,
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

    // Generate and persist a random nickname on first start (same as TUI/Dioxus)
    if let Err(e) = ensure_self_nickname() {
        p2plog_debug(format!("Failed to ensure self nickname: {e}"));
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

        let pid = get_local_peer_id().map_err(|e| e.to_string())?.to_string();

        Ok::<_, String>((event_rx, cmd_tx, pid))
    })?;

    let new_node = MobileNode {
        event_rx: Some(event_rx),
        cmd_tx: Some(cmd_tx),
        peer_id: peer_id.clone(),
        _runtime: runtime,
    };

    // NODE is an OnceLock, so it can only be initialized once. On the first
    // start it is empty; on a restart (after stop_node) it already holds the
    // previous, now-inactive node. Replacing the inner value keeps the new
    // runtime and swarm handler alive — calling NODE.set here would fail
    // (already set) and silently drop the new node, killing the relaunched
    // swarm so its listen addresses (and all events) never come back.
    match NODE.get() {
        Some(existing) => {
            let mut g = existing.lock().unwrap();
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
#[flutter_rust_bridge::frb(ignore)]
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
            // Track connected peers for broadcast attribution.
            if let Ok(mut set) = CONNECTED_PEERS.lock() {
                set.insert(peer_id.clone());
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
                });
            }
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            if let Ok(mut set) = CONNECTED_PEERS.lock() {
                set.remove(peer_id);
            }
        }
        SwarmEvent::BroadcastMessage(m) | SwarmEvent::DirectMessage(m) => {
            // Store the sender's announced nickname
            if let Some(nick) = &m.nickname
                && !nick.is_empty()
            {
                let _ = record_peer_received_name_change(&m.peer_id, nick);
            }
            // If DM is empty with a nickname, it's a nickname-only exchange — don't persist as message
            if matches!(ev, SwarmEvent::DirectMessage(_))
                && m.content.is_empty()
                && m.nickname.is_some()
            {
                // Nickname-only DM, already stored above
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, addresses } => {
            let addrs: Vec<String> = addresses.iter().map(|a| a.to_string()).collect();
            if let Err(e) = save_peer(peer_id, &addrs) {
                p2plog_debug(format!("Failed to save discovered peer: {e}"));
            }
        }
        _ => {}
    }
}

/// Send a broadcast message to all connected peers.
#[flutter_rust_bridge::frb(ignore)]
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
#[flutter_rust_bridge::frb(ignore)]
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
    // Always append a short peer-ID suffix to the sender name so it stays
    // consistent across messages (announced nickname or generated petname).
    // Falls back to the resolved display name when no nickname was announced.
    let sender_nickname = match (msg.sender_nickname.clone(), msg.peer_id.clone()) {
        (Some(nick), Some(pid)) => {
            let short = crate::fmt::short_peer_id(&pid);
            let suffix = &short[..3.min(short.len())];
            Some(format!("{nick} ({suffix})"))
        }
        (Some(nick), None) => Some(nick),
        (None, Some(pid)) => crate::get_peer_display_name(&pid).ok(),
        (None, None) => None,
    };
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
    }
}

/// Load broadcast messages (newest-first from DB, reversed to chronological).
#[flutter_rust_bridge::frb(ignore)]
pub fn load_broadcast_messages(limit: i64) -> Result<Vec<ChatMessage>, String> {
    let msgs = load_messages(CHAT_TOPIC, limit as usize).map_err(|e| e.to_string())?;
    Ok(msgs.into_iter().rev().map(message_to_chat).collect())
}

/// Load DM history with a specific peer (already oldest-first in DB).
#[flutter_rust_bridge::frb(ignore)]
pub fn load_dm_messages(peer_id: String, limit: i64) -> Result<Vec<ChatMessage>, String> {
    let msgs = load_direct_messages(&peer_id, limit as usize).map_err(|e| e.to_string())?;
    Ok(msgs.into_iter().map(message_to_chat).collect())
}

/// Save an outgoing broadcast message, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
#[flutter_rust_bridge::frb(ignore)]
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

    // Attribute this broadcast to every peer that was online to receive it.
    let connected: Vec<String> = CONNECTED_PEERS
        .lock()
        .map(|set| set.iter().cloned().collect())
        .unwrap_or_default();
    if !connected.is_empty() {
        let _ = crate::peers::record_broadcasts_sent(&connected);
    }

    Ok(chat)
}

/// Save an outgoing DM, persist to DB, and send via swarm.
/// Returns the saved ChatMessage.
#[flutter_rust_bridge::frb(ignore)]
pub fn save_outgoing_dm(peer_id: String, content: String) -> Result<ChatMessage, String> {
    let msg_id = gen_msg_id();
    let sent_at = current_timestamp();
    let nickname = crate::get_self_nickname().ok().flatten();

    let meta = MessageMeta {
        sender_nickname: nickname.clone(),
        msg_id: Some(msg_id.clone()),
        sent_at: Some(sent_at),
    };
    let msg = save_message_with_meta(&content, None, CHAT_TOPIC, true, Some(&peer_id), meta)
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
#[flutter_rust_bridge::frb(ignore)]
pub fn save_incoming_message(
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
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
            sent_at: Some(1718455800.0),
            sender_nickname: Some("Alice".into()),
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
        };
        let chat = message_to_chat(msg);
        let short = crate::fmt::short_peer_id("peer-bob");
        let suffix = &short[..3.min(short.len())];
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
        });
        process_event_for_mobile(&ev, &None);
    }

    #[test]
    fn test_process_event_broadcast_with_nickname() {
        let ev = SwarmEvent::BroadcastMessage(MessageEvent {
            content: "hi all".into(),
            peer_id: "peer-broad".into(),
            latency: None,
            nickname: Some("Charlie".into()),
            msg_id: Some("msg-b1".into()),
        });
        process_event_for_mobile(&ev, &None);
    }

    #[test]
    fn test_process_event_peer_connected_no_sender() {
        process_event_for_mobile(&SwarmEvent::PeerConnected("p1".into()), &None);
    }

    #[test]
    fn test_process_event_ignored_variants() {
        process_event_for_mobile(&SwarmEvent::PeerDisconnected("p1".into()), &None);
        process_event_for_mobile(
            &SwarmEvent::ListenAddrEstablished("/ip4/0.0.0.0/tcp/0".into()),
            &None,
        );
        process_event_for_mobile(
            &SwarmEvent::Receipt {
                peer_id: "p1".into(),
                ack_for: "m1".into(),
                received_at: None,
            },
            &None,
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
        process_event_for_mobile(&ev, &None);
    }

    // ── SwarmEventJson / ChatMessage / MobilePeerRecord ────────────────

    #[test]
    fn test_swarm_event_json_clone_debug() {
        let j = default_event();
        let cloned = j.clone();
        assert!(cloned.event_type.is_empty());
        let _ = format!("{:?}", j);
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
        };
        let chat = message_to_chat(msg);
        let cloned = chat.clone();
        assert_eq!(cloned.id, 1);
        let _ = format!("{:?}", chat);
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
        let _ = format!("{:?}", r);
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
    fn test_send_broadcast_without_start_fails() {
        let result = send_broadcast("hello".into());
        assert!(result.is_err());
    }

    #[test]
    fn test_send_dm_without_start_fails() {
        let result = send_dm("peer1".into(), "hi".into());
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
}
