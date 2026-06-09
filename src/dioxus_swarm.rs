//! Swarm event processing for the Dioxus UI

use crate::dioxus_app::{AppState, MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, send_swarm_cmd};
use crate::{DisplayMessage, PeerRecord, SwarmCommand, SwarmEvent};
use dioxus::prelude::*;

pub(crate) fn process_swarm_event(state: &mut Signal<AppState>, event: SwarmEvent) {
    match event {
        SwarmEvent::BroadcastMessage(e) => {
            let mut s = state.write();
            if let Some(n) = e.nickname.as_ref() {
                s.received_nicknames.insert(e.peer_id.clone(), n.clone());
                let _ = crate::set_peer_received_nickname(&e.peer_id, n);
            }
            if e.content.trim().is_empty() && e.nickname.is_some() {
                return;
            }
            let ts = crate::format_now();
            let sender =
                crate::peer_display_name(&e.peer_id, &s.local_nicknames, &s.received_nicknames);
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                e.latency.unwrap_or_default(),
                sender,
                e.content
            );
            s.messages.push_back(DisplayMessage {
                text: msg,
                sender_peer_id: Some(e.peer_id.clone()),
            });
            s.message_ids.push_back(e.msg_id);
            if s.messages.len() > MAX_MESSAGE_HISTORY {
                s.messages.pop_front();
                s.message_ids.pop_front();
            }
            let _ = crate::save_message(&e.content, Some(&e.peer_id), &s.topic_str, false, None);
        }
        SwarmEvent::DirectMessage(e) => {
            let mut s = state.write();
            if let Some(n) = e.nickname.as_ref() {
                s.received_nicknames.insert(e.peer_id.clone(), n.clone());
                let _ = crate::set_peer_received_nickname(&e.peer_id, n);
            }
            if e.content.trim().is_empty() && e.nickname.is_some() {
                return;
            }
            let ts = crate::format_now();
            let sender =
                crate::peer_display_name(&e.peer_id, &s.local_nicknames, &s.received_nicknames);
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                e.latency.unwrap_or_default(),
                sender,
                e.content
            );
            s.dm_message_ids
                .entry(e.peer_id.clone())
                .or_default()
                .push_back(e.msg_id);
            let msgs = s.dm_messages.entry(e.peer_id.clone()).or_default();
            msgs.push_back(msg);
            if msgs.len() > MAX_DM_HISTORY {
                msgs.pop_front();
                s.dm_message_ids.get_mut(&e.peer_id).unwrap().pop_front();
            }
            if !s.dm_tabs.contains(&e.peer_id) {
                s.dm_tabs.push(e.peer_id.clone());
            }
            let _ = crate::save_message(
                &e.content,
                Some(&e.peer_id),
                &s.topic_str,
                true,
                Some(&e.peer_id),
            );
        }
        SwarmEvent::Receipt {
            peer_id,
            ack_for,
            received_at: _,
        } => {
            let mut s = state.write();
            let at = crate::current_timestamp();
            let mut is_dm = false;
            for ids in s.dm_message_ids.values() {
                if ids
                    .iter()
                    .any(|id| id.as_ref().is_some_and(|v| v == &ack_for))
                {
                    is_dm = true;
                    break;
                }
            }
            if is_dm {
                s.dm_receipts.insert(ack_for.clone(), (peer_id.clone(), at));
                let _ = crate::save_receipt(&ack_for, &peer_id, 1, at);
            }
            s.broadcast_receipts
                .entry(ack_for.clone())
                .or_default()
                .insert(peer_id.clone(), at);
            let _ = crate::save_receipt(&ack_for, &peer_id, 0, at);
        }
        SwarmEvent::PeerConnected(peer_id) => {
            let mut s = state.write();
            s.concurrent_peers += 1;
            if !s.peers.iter().any(|p| p.peer_id == peer_id)
                && let Ok(peer) = crate::save_peer(&peer_id, &[])
            {
                let fs = crate::format_peer_datetime(peer.first_seen);
                let ls = crate::format_peer_datetime(peer.last_seen);
                s.peers.push_back(PeerRecord {
                    peer_id: peer_id.clone(),
                    first_seen: fs,
                    last_seen: ls,
                });
            }
            let nickname = s.own_nickname.clone();
            let msg_id = crate::gen_msg_id();
            send_swarm_cmd(SwarmCommand::SendDm {
                peer_id,
                content: String::new(),
                nickname: Some(nickname),
                msg_id: Some(msg_id),
                ack_for: None,
            });
        }
        SwarmEvent::PeerDisconnected(peer_id) => {
            let mut s = state.write();
            s.connected = false;
            s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
            s.push_log(format!("Peer disconnected: {}", peer_id));
        }
        SwarmEvent::ListenAddrEstablished(addr) => {
            let mut s = state.write();
            s.push_log(format!("Listening on: {}", addr));
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerDiscovered { peer_id, .. } => {
            let mut s = state.write();
            if !s.peers.iter().any(|p| p.peer_id == peer_id) {
                let now = crate::now_timestamp();
                s.peers.push_back(PeerRecord {
                    peer_id,
                    first_seen: now.clone(),
                    last_seen: now,
                });
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            let mut s = state.write();
            s.push_log(format!("Peer expired: {}", peer_id));
        }
    }
}
