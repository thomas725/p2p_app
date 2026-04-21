use super::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

pub fn handle_ui_command(
    cmd: UiCommand,
    state: &mut RunningState,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &gossipsub::IdentTopic,
) {
    match cmd {
        UiCommand::ShowMessage(msg, peer_id) => {
            state.messages.push_back((msg, peer_id));
            if state.messages.len() > MAX_MESSAGES {
                state.messages.pop_front();
            }
        }
        UiCommand::Exit => {
            // Exit handling will be done by the caller
        }
        UiCommand::AddDmTab(target) => {
            state.dynamic_tabs.add_dm_tab(target);
        }
        UiCommand::PeerConnected(peer_id) => {
            state.concurrent_peers += 1;
            let addresses = vec![peer_id.clone()];
            if let Ok(peer) = p2p_app::save_peer(&peer_id, &addresses) {
                let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                if !state.peers.iter().any(|(id, _, _)| id == &peer_id) {
                    state
                        .peers
                        .push_front((peer_id.clone(), first_seen, last_seen));
                }
            }
            if let Ok(peer_id_val) = peer_id.parse::<libp2p::PeerId>() {
                // Send command to swarm handler via channel
                // This would normally be done through a channel, but for simplicity
                // we're calling it directly here since we have access to swarm
                state
                    .swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id_val);
            }
        }
        UiCommand::PeerDisconnected(peer_id) => {
            state.concurrent_peers = state.concurrent_peers.saturating_sub(1);
        }
        UiCommand::NewListenAddr(addr) => {
            let _ = p2p_app::log_debug(&state.logs, format!("Listening on: {}", addr));
        }
        UiCommand::BroadcastMessage(content, peer_id_str, latency) => {
            let now = SystemTime::now();
            let ts = p2p_app::format_system_time(now);
            let sender_display = p2p_app::peer_display_name(
                &peer_id_str,
                &state.local_nicknames,
                &state.received_nicknames,
            );
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                latency.unwrap_or_default(),
                sender_display,
                content
            );
            state
                .messages
                .push_back((msg.clone(), Some(peer_id_str.clone())));
            if state.messages.len() > MAX_MESSAGES {
                state.messages.pop_front();
            }
            if state.active_tab != 0 {
                state.unread_broadcasts += 1;
            }
            if let Err(e) =
                p2p_app::save_message(&content, Some(&peer_id_str), &state.topic_str, false, None)
            {
                let _ = p2p_app::log_debug(&state.logs, format!("Failed to save message: {}", e));
            }
        }
        UiCommand::DirectMessage(content, peer_id_str, latency) => {
            let now = SystemTime::now();
            let ts = p2p_app::format_system_time(now);
            let sender_display = p2p_app::peer_display_name(
                &peer_id_str,
                &state.local_nicknames,
                &state.received_nicknames,
            );
            let dm_msgs = state.dm_messages.entry(peer_id_str.clone()).or_default();
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                latency.unwrap_or_default(),
                sender_display,
                content
            );
            dm_msgs.push_back(msg.clone());
            if dm_msgs.len() > MAX_MESSAGES {
                dm_msgs.pop_front();
            }
            let current_peer = match state.dynamic_tabs.tab_index_to_content(state.active_tab) {
                TabContent::Direct(id) => Some(id),
                _ => None,
            };
            let is_current_dm = current_peer.as_ref() == Some(&peer_id_str);
            if !is_current_dm {
                *state.unread_dms.entry(peer_id_str.clone()).or_insert(0) += 1;
                state.dynamic_tabs.add_dm_tab(peer_id_str.clone());
            }
            if let Err(e) = p2p_app::save_message(
                &content,
                Some(&peer_id_str),
                &state.topic_str,
                true,
                Some(&peer_id_str),
            ) {
                let _ = p2p_app::log_debug(&state.logs, format!("Failed to save DM: {}", e));
            }
        }
        UiCommand::UpdatePeers(new_peers) => {
            for (peer_id, first_seen, last_seen) in new_peers {
                if !state.peers.iter().any(|(id, _, _)| id == &peer_id) {
                    state.peers.push_front((peer_id, first_seen, last_seen));
                }
            }
        }
        UiCommand::ShowDmMessages(peer_id, dm_msgs) => {
            state.dynamic_tabs.add_dm_tab(peer_id.clone());
            state.dm_messages.insert(peer_id, dm_msgs);
        }
        UiCommand::RemoveDmTab(peer_id) => {
            state.dynamic_tabs.remove_dm_tab(&peer_id);
            state.dm_messages.remove(&peer_id);
            state.dm_inputs.remove(&peer_id);
        }
        UiCommand::SetActiveTab(idx) => {
            state.active_tab = idx;
        }
        UiCommand::ClearInput => {
            state.chat_input = init_textarea();
        }
        UiCommand::SetChatInput(text) => {
            state.chat_input = init_textarea();
            state.chat_input.insert_str(&text);
        }
        UiCommand::ToggleMouseCapture => {
            state.mouse_capture = !state.mouse_capture;
            if state.mouse_capture {
                let _ = execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
            } else {
                let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
            }
        }
        _ => {}
    }
}
