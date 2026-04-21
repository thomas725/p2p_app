use super::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

/// Handles a UI command and returns whether the application should exit
pub fn handle_ui_command(
    cmd: UiCommand,
    messages: &mut VecDeque<(String, Option<String>)>,
    dm_messages: &mut HashMap<String, VecDeque<String>>,
    peers: &mut VecDeque<(String, String, String)>,
    dynamic_tabs: &mut DynamicTabs,
    active_tab: &mut usize,
    dm_inputs: &mut HashMap<String, TextArea<'static>>,
    chat_input: &mut TextArea<'static>,
    concurrent_peers: &mut usize,
    peer_selection: &mut usize,
    debug_scroll_offset: &mut usize,
    debug_auto_scroll: &mut bool,
    chat_scroll_offset: &mut usize,
    chat_auto_scroll: &mut bool,
    own_nickname: &str,
    local_nicknames: &mut HashMap<String, String>,
    received_nicknames: &mut HashMap<String, String>,
    logs: &Arc<Mutex<VecDeque<String>>>,
    mouse_capture: &mut bool,
    unread_broadcasts: &mut u32,
    unread_dms: &mut HashMap<String, u32>,
    topic_str: &str,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &gossipsub::IdentTopic,
    swarm_handler_handle: &mut tokio::task::JoinHandle<()>,
    input_handler_handle: &mut tokio::task::JoinHandle<()>,
) -> bool {
    match cmd {
        UiCommand::ShowMessage(msg, peer_id) => {
            messages.push_back((msg, peer_id));
            if messages.len() > MAX_MESSAGES {
                messages.pop_front();
            }
            false
        }
        UiCommand::Exit => {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
            let _ = disable_raw_mode();
            let _ = execute!(std::io::stdout(), PopKeyboardEnhancementFlags);
            let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
            swarm_handler_handle.abort();
            input_handler_handle.abort();
            true
        }
        UiCommand::AddDmTab(target) => {
            dynamic_tabs.add_dm_tab(target);
            false
        }
        UiCommand::PeerConnected(peer_id) => {
            *concurrent_peers += 1;
            let addresses = vec![peer_id.clone()];
            if let Ok(peer) = p2p_app::save_peer(&peer_id, &addresses) {
                let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                if !peers.iter().any(|(id, _, _)| id == &peer_id) {
                    peers.push_front((peer_id.clone(), first_seen, last_seen));
                }
            }
            if let Ok(peer_id_val) = peer_id.parse::<libp2p::PeerId>() {
                // Send command to swarm handler via channel
                // Note: In a full refactor, this would go through a channel
                // For now, we keep the direct call since we have access to swarm
                swarm
                    .behaviour_mut()
                    .gossipsub
                    .add_explicit_peer(&peer_id_val);
            }
            false
        }
        UiCommand::PeerDisconnected(peer_id) => {
            *concurrent_peers = (*concurrent_peers).saturating_sub(1);
            false
        }
        UiCommand::NewListenAddr(addr) => {
            let _ = p2p_app::log_debug(logs, format!("Listening on: {}", addr));
            false
        }
        UiCommand::BroadcastMessage(content, peer_id_str, latency) => {
            let now = SystemTime::now();
            let ts = p2p_app::format_system_time(now);
            let sender_display =
                p2p_app::peer_display_name(&peer_id_str, local_nicknames, received_nicknames);
            let msg = format!(
                "{} {} [{}] {}",
                ts,
                latency.unwrap_or_default(),
                sender_display,
                content
            );
            messages.push_back((msg.clone(), Some(peer_id_str.clone())));
            if messages.len() > MAX_MESSAGES {
                messages.pop_front();
            }
            if *active_tab != 0 {
                *unread_broadcasts += 1;
            }
            if let Err(e) =
                p2p_app::save_message(&content, Some(&peer_id_str), topic_str, false, None)
            {
                let _ = p2p_app::log_debug(logs, format!("Failed to save message: {}", e));
            }
            false
        }
        UiCommand::DirectMessage(content, peer_id_str, latency) => {
            let now = SystemTime::now();
            let ts = p2p_app::format_system_time(now);
            let sender_display =
                p2p_app::peer_display_name(&peer_id_str, local_nicknames, received_nicknames);
            let dm_msgs = dm_messages.entry(peer_id_str.clone()).or_default();
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
            let current_peer = match dynamic_tabs.tab_index_to_content(*active_tab) {
                TabContent::Direct(id) => Some(id),
                _ => None,
            };
            let is_current_dm = current_peer.as_ref() == Some(&peer_id_str);
            if !is_current_dm {
                *unread_dms.entry(peer_id_str.clone()).or_insert(0) += 1;
                dynamic_tabs.add_dm_tab(peer_id_str.clone());
            }
            if let Err(e) = p2p_app::save_message(
                &content,
                Some(&peer_id_str),
                topic_str,
                true,
                Some(&peer_id_str),
            ) {
                let _ = p2p_app::log_debug(logs, format!("Failed to save DM: {}", e));
            }
            false
        }
        UiCommand::UpdatePeers(new_peers) => {
            for (peer_id, first_seen, last_seen) in new_peers {
                if !peers.iter().any(|(id, _, _)| id == &peer_id) {
                    peers.push_front((peer_id, first_seen, last_seen));
                }
            }
            false
        }
        UiCommand::ShowDmMessages(peer_id, dm_msgs) => {
            dynamic_tabs.add_dm_tab(peer_id.clone());
            dm_messages.insert(peer_id, dm_msgs);
            false
        }
        UiCommand::RemoveDmTab(peer_id) => {
            dynamic_tabs.remove_dm_tab(&peer_id);
            dm_messages.remove(&peer_id);
            dm_inputs.remove(&peer_id);
            false
        }
        UiCommand::SetActiveTab(idx) => {
            *active_tab = idx;
            false
        }
        UiCommand::ClearInput => {
            *chat_input = init_textarea();
            false
        }
        UiCommand::SetChatInput(text) => {
            *chat_input = init_textarea();
            chat_input.insert_str(&text);
            false
        }
        UiCommand::ToggleMouseCapture => {
            *mouse_capture = !(*mouse_capture);
            if *mouse_capture {
                let _ = execute!(std::io::stdout(), crossterm::event::EnableMouseCapture);
            } else {
                let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);
            }
            false
        }
        _ => false,
    }
}
