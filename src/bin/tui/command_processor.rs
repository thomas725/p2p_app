use super::event_source::InputEvent;
use super::input_processor::process_input_event;
use super::main_loop::RenderEvent;
use super::state::{AppState, SharedState};
use super::state::{MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, trim_history};
use p2p_app::{DisplayMessage, PeerRecord, SwarmCommand, SwarmEvent, p2plog_debug};
use std::time::SystemTime;
use tokio::sync::mpsc;

enum Event {
    Input(InputEvent),
    SwarmEvent(SwarmEvent),
}

fn sort_peers_by_last_seen(state: &mut AppState) {
    state.peer_selection =
        p2p_app::tui_helpers::sort_peers_by_last_seen(&mut state.peers, state.peer_selection);
}

fn upsert_peer_last_seen(state: &mut AppState, peer_id: &str, seen_at: chrono::NaiveDateTime) {
    let seen_str = p2p_app::format_peer_datetime(seen_at);
    state.peer_selection = p2p_app::tui_helpers::upsert_peer_last_seen(
        &mut state.peers,
        state.peer_selection,
        peer_id,
        &seen_str,
    );
}

/// Pure state mutation for incoming broadcast messages. Returns the formatted message string.
pub fn apply_broadcast_to_state(
    state: &mut AppState,
    content: &str,
    peer_id: &str,
    latency: Option<&str>,
    msg_id: Option<String>,
) -> String {
    let ts = p2p_app::format_system_time(SystemTime::now());
    let sender_display =
        p2p_app::peer_display_name(peer_id, &state.local_nicknames, &state.received_nicknames);
    let msg = format!(
        "{} {} [{}] {}",
        ts,
        latency.unwrap_or_default(),
        sender_display,
        content
    );
    state.messages.push_back(DisplayMessage {
        text: msg.clone(),
        sender_peer_id: Some(peer_id.to_string()),
    });
    state.message_ids.push_back(msg_id);
    trim_history(&mut state.messages, MAX_MESSAGE_HISTORY);
    trim_history(&mut state.message_ids, MAX_MESSAGE_HISTORY);
    msg
}

/// Pure state mutation for incoming direct messages. Returns the formatted message string.
pub fn apply_dm_to_state(
    state: &mut AppState,
    content: &str,
    peer_id: &str,
    latency: Option<&str>,
    msg_id: Option<String>,
) -> String {
    let ts = p2p_app::format_system_time(SystemTime::now());
    let sender_display =
        p2p_app::peer_display_name(peer_id, &state.local_nicknames, &state.received_nicknames);
    let msg = format!(
        "{} {} [{}] {}",
        ts,
        latency.unwrap_or_default(),
        sender_display,
        content
    );
    state
        .dm_message_ids
        .entry(peer_id.to_string())
        .or_default()
        .push_back(msg_id);
    let dm_msgs = state.dm_messages.entry(peer_id.to_string()).or_default();
    dm_msgs.push_back(msg.clone());
    trim_history(dm_msgs, MAX_DM_HISTORY);
    if let Some(ids) = state.dm_message_ids.get_mut(peer_id) {
        trim_history(ids, MAX_DM_HISTORY);
    }
    state.dynamic_tabs.add_dm_tab(peer_id.to_string());
    msg
}

/// Pure: check if a receipt is for a DM message
pub fn is_dm_receipt(state: &AppState, ack_for: &str) -> bool {
    state.dm_message_ids.values().any(|ids| {
        ids.iter()
            .any(|id| id.as_ref().is_some_and(|v| v == ack_for))
    })
}

/// Pure state mutation for receipts
pub fn apply_receipt_to_state(
    state: &mut AppState,
    ack_for: &str,
    peer_id: &str,
    at: f64,
    is_dm: bool,
) {
    if is_dm {
        state
            .dm_receipts
            .insert(ack_for.to_string(), (peer_id.to_string(), at));
    }
    state
        .broadcast_receipts
        .entry(ack_for.to_string())
        .or_default()
        .insert(peer_id.to_string(), at);
}

/// State mutation: increment connected peer count. Returns new count.
pub fn apply_peer_connected_count(state: &mut AppState) -> usize {
    state.concurrent_peers += 1;
    state.concurrent_peers
}

/// State mutation: add peer to peer list and re-sort
pub fn add_peer_to_state_list(
    state: &mut AppState,
    peer_id: &str,
    first_seen: &str,
    last_seen: &str,
) {
    state.peers.push_back(PeerRecord {
        peer_id: peer_id.to_string(),
        first_seen: first_seen.to_string(),
        last_seen: last_seen.to_string(),
    });
    sort_peers_by_last_seen(state);
}

/// State mutation: decrement connected peer count. Returns new count.
pub fn apply_peer_disconnected_count(state: &mut AppState) -> usize {
    state.concurrent_peers = state.concurrent_peers.saturating_sub(1);
    state.concurrent_peers
}

#[cfg(feature = "mdns")]
pub fn apply_peer_discovered_state(state: &mut AppState, peer_id: &str) {
    if !state.peers.iter().any(|p| p.peer_id == peer_id) {
        let now = p2p_app::now_timestamp();
        state.peers.push_back(PeerRecord {
            peer_id: peer_id.to_string(),
            first_seen: now.clone(),
            last_seen: now,
        });
        sort_peers_by_last_seen(state);
    }
}

async fn handle_incoming_message(
    s: &mut AppState,
    content: &str,
    peer_id: &str,
    latency: Option<String>,
    nickname: Option<String>,
    msg_id: Option<String>,
    is_direct: bool,
) {
    if let Some(n) = nickname.as_ref() {
        s.received_nicknames.insert(peer_id.to_string(), n.clone());
        let _ = p2p_app::set_peer_received_nickname(peer_id, n);
    }
    if content.trim().is_empty() && nickname.is_some() {
        upsert_peer_last_seen(s, peer_id, chrono::Utc::now().naive_utc());
        return;
    }
    let sender_display =
        p2p_app::peer_display_name(peer_id, &s.local_nicknames, &s.received_nicknames);
    if is_direct {
        apply_dm_to_state(s, content, peer_id, latency.as_deref(), msg_id);
    } else {
        apply_broadcast_to_state(s, content, peer_id, latency.as_deref(), msg_id);
    }
    p2plog_debug(format!(
        "{} from {sender_display}: {content}",
        if is_direct { "DM" } else { "Broadcast" }
    ));
    if let Err(e) = p2p_app::save_message(
        content,
        Some(peer_id),
        &s.topic_str,
        is_direct,
        is_direct.then_some(peer_id),
    ) {
        p2plog_debug(format!("Failed to save: {e}"));
    }
    upsert_peer_last_seen(s, peer_id, chrono::Utc::now().naive_utc());
}

/// Processes network (swarm) events and updates application state
async fn process_swarm_event(
    swarm_event: SwarmEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    match swarm_event {
        SwarmEvent::BroadcastMessage(msg) => {
            let mut s = state.lock().await;
            handle_incoming_message(
                &mut s,
                &msg.content,
                &msg.peer_id,
                msg.latency,
                msg.nickname,
                msg.msg_id,
                false,
            )
            .await;
        }
        SwarmEvent::DirectMessage(msg) => {
            let mut s = state.lock().await;
            handle_incoming_message(
                &mut s,
                &msg.content,
                &msg.peer_id,
                msg.latency,
                msg.nickname,
                msg.msg_id,
                true,
            )
            .await;
        }
        SwarmEvent::Receipt {
            peer_id, ack_for, ..
        } => {
            let mut s = state.lock().await;
            let at = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            let is_dm = is_dm_receipt(&s, &ack_for);
            apply_receipt_to_state(&mut s, &ack_for, &peer_id, at, is_dm);
            let kind = if is_dm { 1 } else { 0 };
            let _ = p2p_app::save_receipt(&ack_for, &peer_id, kind, at);
        }
        SwarmEvent::PeerConnected(peer_id) => {
            let (own_nickname, _concurrent_peers) = {
                let mut s = state.lock().await;
                let count = apply_peer_connected_count(&mut s);
                p2plog_debug(format!("Peer connected: {} (total: {})", peer_id, count));
                if !s.peers.iter().any(|p| p.peer_id == peer_id)
                    && let Ok(peer) = p2p_app::save_peer(&peer_id, &[])
                {
                    let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                    let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                    add_peer_to_state_list(&mut s, &peer_id, &first_seen, &last_seen);
                }
                (s.own_nickname.clone(), count)
            };

            let msg_id = p2p_app::gen_msg_id();
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SendDm {
                    peer_id: peer_id.clone(),
                    content: String::new(),
                    nickname: Some(own_nickname),
                    msg_id: Some(msg_id),
                    ack_for: None,
                })
                .await;
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            let mut s = state.lock().await;
            let count = apply_peer_disconnected_count(&mut s);
            p2plog_debug(format!("Peer disconnected: {} (total: {})", peer_id, count));
        }
        SwarmEvent::ListenAddrEstablished(addr) => {
            p2plog_debug(format!("Listening on: {addr}"));
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, .. } => {
            let mut s = state.lock().await;
            apply_peer_discovered_state(&mut s, &peer_id);
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            p2plog_debug(format!("Peer expired: {peer_id}"));
        }
    }
}

pub fn spawn_command_processor(
    state: SharedState,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    render_tx: mpsc::Sender<RenderEvent>,
    swarm_cmd_tx: mpsc::Sender<SwarmCommand>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let cmd_tx_for_return = swarm_cmd_tx.clone();
    let handle = tokio::spawn(async move {
        p2plog_debug("CommandProcessor task started".to_string());

        loop {
            let event = tokio::select! {
                Some(input_event) = input_rx.recv() => Some(Event::Input(input_event)),
                Some(swarm_event) = swarm_event_rx.recv() => Some(Event::SwarmEvent(swarm_event)),
                else => None,
            };

            match event {
                Some(Event::Input(input_event)) => {
                    if process_input_event(input_event, &state, &swarm_cmd_tx, &render_tx).await {
                        break;
                    }
                }
                Some(Event::SwarmEvent(swarm_event)) => {
                    process_swarm_event(swarm_event, &state, &swarm_cmd_tx).await;
                    let _ = render_tx.send(RenderEvent).await;
                }
                None => break,
            }
        }
    });

    (handle, cmd_tx_for_return)
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_command_processor.rs"]
mod tests;
