use p2p_app::{SwarmCommand, SwarmEvent};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::sync::mpsc;
use super::state::AppState;
use super::input_handler::InputEvent;
use super::constants::{CHANNEL_CAPACITY, MAX_MESSAGE_HISTORY, MAX_DM_HISTORY, MAX_PEERS};

/// Spawns the command processor task (the main state mutation engine)
///
/// This task orchestrates the core business logic:
/// 1. Receives InputEvent from InputHandler
/// 2. Receives SwarmEvent from SwarmHandler
/// 3. Processes events and mutates the shared AppState
/// 4. Optionally sends SwarmCommand back to network layer
///
/// **Mutation logic:**
/// - InputEvent triggers: chat input, commands, navigation, DM interactions
/// - SwarmEvent triggers: peer updates, message display, connection status
///
/// **Concurrency model:**
/// - Uses `tokio::select!` to wait on both input and swarm event channels
/// - Locks AppState only when necessary for mutation
/// - Bounded message history (MAX_MESSAGE_HISTORY) prevents memory bloat
///
/// **Returns:**
/// - A JoinHandle to monitor task health
/// - A SwarmCommand sender (for potential future use)
///
/// The task runs indefinitely until explicitly shut down or on error.
pub fn spawn_command_processor(
    state: Arc<Mutex<AppState>>,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(CHANNEL_CAPACITY);

    let handle = tokio::spawn(async move {
        p2p_app::log_debug(&logs, "CommandProcessor task started".to_string());
        loop {
            tokio::select! {
                Some(input_event) = input_rx.recv() => {
                    // Handle input events from terminal
                    match input_event {
                        InputEvent::Key(key_event) => {
                            // Detect Ctrl+Q or Esc to exit
                            if key_event.code == crossterm::event::KeyCode::Esc
                                || (key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                                    && key_event.code == crossterm::event::KeyCode::Char('q'))
                            {
                                // Exit signal - tasks will stop when this loop exits
                                p2p_app::log_debug(&logs, "Exit signal received".to_string());
                                return;
                            }

                            // Handle tab navigation and input
                            if let Ok(mut s) = state.lock() {
                                match key_event.code {
                                    // Tab switching: Tab for next, Shift+Tab for previous
                                    crossterm::event::KeyCode::Tab => {
                                        let max_tabs = s.dynamic_tabs.total_tab_count();
                                        s.active_tab = (s.active_tab + 1) % max_tabs;
                                        s.chat_scroll_offset = 0;
                                        p2p_app::log_debug(&logs, format!("Switched to tab {}", s.active_tab));
                                    }
                                    crossterm::event::KeyCode::BackTab => {
                                        let max_tabs = s.dynamic_tabs.total_tab_count();
                                        s.active_tab = if s.active_tab == 0 { max_tabs - 1 } else { s.active_tab - 1 };
                                        s.chat_scroll_offset = 0;
                                        p2p_app::log_debug(&logs, format!("Switched to tab {}", s.active_tab));
                                    }
                                    // Scroll up/down in chat or logs
                                    crossterm::event::KeyCode::Up => {
                                        if s.chat_auto_scroll {
                                            s.chat_auto_scroll = false;
                                        }
                                        s.chat_scroll_offset = s.chat_scroll_offset.saturating_add(1);
                                    }
                                    crossterm::event::KeyCode::Down => {
                                        s.chat_scroll_offset = s.chat_scroll_offset.saturating_sub(1);
                                        if s.chat_scroll_offset == 0 {
                                            s.chat_auto_scroll = true;
                                        }
                                    }
                                    // Toggle mouse capture on F12
                                    crossterm::event::KeyCode::F(12) => {
                                        s.mouse_capture = !s.mouse_capture;
                                        let mode = if s.mouse_capture { "enabled" } else { "disabled" };
                                        p2p_app::log_debug(&logs, format!("Mouse capture {}", mode));

                                        // Execute the terminal command to actually toggle mouse capture
                                        use ratatui::crossterm::execute;
                                        let mut stdout = std::io::stdout();
                                        let _ = if s.mouse_capture {
                                            execute!(stdout, crossterm::event::EnableMouseCapture)
                                        } else {
                                            execute!(stdout, crossterm::event::DisableMouseCapture)
                                        };
                                    }
                                    // Send message on Enter
                                    crossterm::event::KeyCode::Enter => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if tab_content.is_input_enabled() {
                                            let text: String = s.chat_input.lines().join("\n");
                                            if !text.trim().is_empty() {
                                                match &tab_content {
                                                    p2p_app::tui_tabs::TabContent::Chat => {
                                                        // Send broadcast message
                                                        let now = SystemTime::now();
                                                        let ts = p2p_app::format_system_time(now);
                                                        let msg = format!("{} [{}] {}", ts, s.own_nickname, text);
                                                        s.messages.push_back((msg, None));
                                                        if s.messages.len() > MAX_MESSAGE_HISTORY {
                                                            s.messages.pop_front();
                                                        }
                                                        p2p_app::log_debug(&logs, format!("Sent broadcast: {}", text));
                                                        if let Err(e) = p2p_app::save_message(&text, None, &s.topic_str, false, None) {
                                                            p2p_app::log_debug(&logs, format!("Failed to save sent message: {}", e));
                                                        }
                                                    }
                                                    p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
                                                        // Send DM
                                                        let now = SystemTime::now();
                                                        let ts = p2p_app::format_system_time(now);
                                                        let msg = format!("{} [{}] {}", ts, s.own_nickname, text);
                                                        let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                                        dm_msgs.push_back(msg);
                                                        if dm_msgs.len() > MAX_DM_HISTORY {
                                                            dm_msgs.pop_front();
                                                        }
                                                        p2p_app::log_debug(&logs, format!("Sent DM to {}: {}", peer_id, text));
                                                        if let Err(e) = p2p_app::save_message(&text, None, &s.topic_str, true, Some(peer_id)) {
                                                            p2p_app::log_debug(&logs, format!("Failed to save sent DM: {}", e));
                                                        }
                                                    }
                                                    _ => {}
                                                }
                                                // Clear the input after sending
                                                s.chat_input = ratatui_textarea::TextArea::default();
                                            }
                                        }
                                    }
                                    // Enter text input to the chat input box
                                    _ => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if tab_content.is_input_enabled() {
                                            s.chat_input.input(key_event);
                                        }
                                    }
                                }
                            }
                        }
                        InputEvent::Mouse(_mouse_event) => {
                            // Mouse events are handled only when mouse capture is enabled.
                            // Currently, mouse input is limited to basic interaction.
                            // Tab switching is better done with Tab/Shift+Tab keys.
                        }
                    }
                }
                Some(swarm_event) = swarm_event_rx.recv() => {
                    match swarm_event {
                        SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = SystemTime::now();
                                let ts = p2p_app::format_system_time(now);
                                let sender_display = p2p_app::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                s.messages.push_back((msg.clone(), Some(peer_id.clone())));
                                if s.messages.len() > MAX_MESSAGE_HISTORY {
                                    s.messages.pop_front();
                                }
                                if s.active_tab != 0 {
                                    s.unread_broadcasts += 1;
                                }
                                p2p_app::log_debug(&logs, format!("Broadcast message from {}: {}", sender_display, content));
                                if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, false, None) {
                                    p2p_app::log_debug(&logs, format!("Failed to save message: {}", e));
                                }
                            }
                        }
                        SwarmEvent::DirectMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = SystemTime::now();
                                let ts = p2p_app::format_system_time(now);
                                let sender_display = p2p_app::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                dm_msgs.push_back(msg);
                                if dm_msgs.len() > MAX_DM_HISTORY {
                                    dm_msgs.pop_front();
                                }
                                *s.unread_dms.entry(peer_id.clone()).or_insert(0) += 1;
                                s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                p2p_app::log_debug(&logs, format!("Direct message from {}: {}", sender_display, content));
                                if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, true, Some(&peer_id)) {
                                    p2p_app::log_debug(&logs, format!("Failed to save DM: {}", e));
                                }
                            }
                        }
                        SwarmEvent::PeerConnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers += 1;
                                p2p_app::log_debug(&logs, format!("Peer connected: {} (total: {})", peer_id, s.concurrent_peers));

                                // Only add peer if not already in list (check prevents duplicates)
                                if !s.peers.iter().any(|(id, _, _)| id == &peer_id) && s.peers.len() < MAX_PEERS {
                                    let addresses = vec![peer_id.clone()];
                                    if let Ok(peer) = p2p_app::save_peer(&peer_id, &addresses) {
                                        let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                                        let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                                        s.peers.push_front((peer_id, first_seen, last_seen));
                                    }
                                }
                            }
                        }
                        SwarmEvent::PeerDisconnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
                                p2p_app::log_debug(&logs, format!("Peer disconnected: {} (total: {})", peer_id, s.concurrent_peers));
                            }
                        }
                        SwarmEvent::ListenAddrEstablished(addr) => {
                            p2p_app::log_debug(&logs, format!("Listening on: {}", addr));
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerDiscovered { peer_id, addresses: _ } => {
                            if let Ok(mut s) = state.lock()
                                && !s.peers.iter().any(|(id, _, _)| id == &peer_id)
                                && s.peers.len() < MAX_PEERS {
                                    s.peers.push_front((peer_id, p2p_app::now_timestamp(), p2p_app::now_timestamp()));
                                }
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerExpired { peer_id } => {
                            p2p_app::log_debug(&logs, format!("Peer expired: {}", peer_id));
                        }
                    }
                }
            }
        }
    });

    (handle, swarm_cmd_tx)
}
