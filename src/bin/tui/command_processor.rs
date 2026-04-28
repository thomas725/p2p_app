use super::input_processor;
use super::event_source::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, SwarmEvent, p2plog_debug};
use std::time::SystemTime;
use tokio::sync::mpsc;

use input_processor::process_input_event;

enum Event {
    Input(InputEvent),
    SwarmEvent(SwarmEvent),
}

fn sort_peers_by_last_seen(state: &mut super::state::AppState) {
    let selected_peer_id = state
        .peers
        .get(state.peer_selection)
        .map(|(id, _, _)| id.clone());

    let mut peers_vec: Vec<_> = state.peers.drain(..).collect();
    // last_seen format is "YYYY-MM-DD HH:MM:SS" which sorts lexicographically.
    peers_vec.sort_by(|a, b| b.2.cmp(&a.2));
    state.peers = peers_vec.into();

    if let Some(sel_id) = selected_peer_id {
        if let Some(idx) = state.peers.iter().position(|(id, _, _)| id == &sel_id) {
            state.peer_selection = idx;
        }
    }
    if state.peer_selection >= state.peers.len() {
        state.peer_selection = state.peers.len().saturating_sub(1);
    }
}

fn upsert_peer_last_seen(state: &mut super::state::AppState, peer_id: &str, seen_at: chrono::NaiveDateTime) {
    let seen_str = p2p_app::format_peer_datetime(seen_at);

    if let Some((_, _first_seen, last_seen)) = state.peers.iter_mut().find(|(id, _, _)| id == peer_id) {
        *last_seen = seen_str;
    } else {
        // If we only know this peer from message history (no `peers` row), derive first/last from message time.
        state.peers.push_back((peer_id.to_string(), seen_str.clone(), seen_str));
    }
    sort_peers_by_last_seen(state);
}

/// Processes network (swarm) events and updates application state
async fn process_swarm_event(
    swarm_event: SwarmEvent,
    state: &SharedState,
) {
    match swarm_event {
        SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
            let mut s = state.lock().await;
            let ts = p2p_app::format_system_time(SystemTime::now());
            let sender_display = p2p_app::peer_display_name(&peer_id, &s.local_nicknames, &s.received_nicknames);
            let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
            s.messages.push_back((msg, Some(peer_id.clone())));
            if s.messages.len() > super::constants::MAX_MESSAGE_HISTORY {
                s.messages.pop_front();
            }
            if s.active_tab != 0 {
                s.unread_broadcasts += 1;
            }
            p2plog_debug(format!("Broadcast from {}: {}", sender_display, content));
            if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, false, None) {
                p2plog_debug(format!("Failed to save: {}", e));
            }
            upsert_peer_last_seen(&mut s, &peer_id, chrono::Utc::now().naive_utc());
        }
        SwarmEvent::DirectMessage { content, peer_id, latency } => {
            let mut s = state.lock().await;
            let ts = p2p_app::format_system_time(SystemTime::now());
            let sender_display = p2p_app::peer_display_name(&peer_id, &s.local_nicknames, &s.received_nicknames);
            let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
            let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
            dm_msgs.push_back(msg);
            if dm_msgs.len() > super::constants::MAX_DM_HISTORY {
                dm_msgs.pop_front();
            }
            *s.unread_dms.entry(peer_id.clone()).or_insert(0) += 1;
            s.dynamic_tabs.add_dm_tab(peer_id.clone());
            p2plog_debug(format!("DM from {}: {}", sender_display, content));
            if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, true, Some(&peer_id)) {
                p2plog_debug(format!("Failed to save DM: {}", e));
            }
            upsert_peer_last_seen(&mut s, &peer_id, chrono::Utc::now().naive_utc());
        }
        SwarmEvent::PeerConnected(peer_id) => {
            let mut s = state.lock().await;
            s.concurrent_peers += 1;
            p2plog_debug(format!("Peer connected: {} (total: {})", peer_id, s.concurrent_peers));
            if !s.peers.iter().any(|(id, _, _)| id == &peer_id)
                && let Ok(peer) = p2p_app::save_peer(&peer_id, std::slice::from_ref(&peer_id)) {
                let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                s.peers.push_back((peer_id, first_seen, last_seen));
                sort_peers_by_last_seen(&mut s);
            }
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            let mut s = state.lock().await;
            s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
            p2plog_debug(format!("Peer disconnected: {} (total: {})", peer_id, s.concurrent_peers));
        }
        SwarmEvent::ListenAddrEstablished(addr) => {
            p2plog_debug(format!("Listening on: {}", addr));
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, .. } => {
            let mut s = state.lock().await;
            if !s.peers.iter().any(|(id, _, _)| id == &peer_id) {
                let now = p2p_app::now_timestamp();
                s.peers.push_back((peer_id, now.clone(), now));
                sort_peers_by_last_seen(&mut s);
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            p2plog_debug(format!("Peer expired: {}", peer_id));
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
                    process_swarm_event(swarm_event, &state).await;
                    let _ = render_tx.send(RenderEvent).await;
                }
                None => break,
            }
        }
    });

    (handle, cmd_tx_for_return)
}
