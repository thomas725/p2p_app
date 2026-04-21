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
                                    // Scroll up/down or select peer
                                    crossterm::event::KeyCode::Up => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                            // Navigate peer list up
                                            s.peer_selection = s.peer_selection.saturating_sub(1);
                                        } else {
                                            // Scroll up in chat/logs
                                            if s.chat_auto_scroll {
                                                s.chat_auto_scroll = false;
                                            }
                                            s.chat_scroll_offset = s.chat_scroll_offset.saturating_add(1);
                                        }
                                    }
                                    crossterm::event::KeyCode::Down => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                            // Navigate peer list down
                                            if s.peer_selection < s.peers.len().saturating_sub(1) {
                                                s.peer_selection += 1;
                                            }
                                        } else {
                                            // Scroll down in chat/logs
                                            s.chat_scroll_offset = s.chat_scroll_offset.saturating_sub(1);
                                            if s.chat_scroll_offset == 0 {
                                                s.chat_auto_scroll = true;
                                            }
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
                                    // Send message on Enter, or open DM on Peers tab (Shift+Enter adds newline)
                                    crossterm::event::KeyCode::Enter => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);

                                        // Shift+Enter adds a newline
                                        if key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                            if tab_content.is_input_enabled() {
                                                s.chat_input.insert_str("\n");
                                            }
                                        } else if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                            // Open DM with selected peer
                                            let peer_id_opt = s.peers.iter().nth(s.peer_selection).map(|(id, _, _)| id.clone());
                                            if let Some(peer_id) = peer_id_opt {
                                                let tab_idx = s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                                s.active_tab = tab_idx;
                                                p2p_app::log_debug(&logs, format!("Opened DM with peer: {}", peer_id));
                                            }
                                        } else if tab_content.is_input_enabled() {
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
                                    // Close current tab with Ctrl+W
                                    crossterm::event::KeyCode::Char('w') if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content {
                                            if let Some(closed_idx) = s.dynamic_tabs.remove_dm_tab(&peer_id) {
                                                // Switch to previous tab
                                                s.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                                                s.peer_selection = 0;
                                                p2p_app::log_debug(&logs, format!("Closed DM tab with peer: {}", peer_id));
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
                        InputEvent::Mouse(mouse_event) => {
                            // Handle mouse clicks for tab switching, closing, and peer selection
                            if let crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) = mouse_event.kind {
                                if let Ok(mut s) = state.lock() {
                                    // Tabs are on row 0; calculate click position
                                    if mouse_event.row == 0 {
                                        let tab_titles = s.dynamic_tabs.all_titles();
                                        let mut col_pos = 0;

                                        // Find which tab was clicked by calculating cumulative positions
                                        for (idx, title) in tab_titles.iter().enumerate() {
                                            // Each tab takes up: title length + 1 space + 1 separator (" | " or similar)
                                            let tab_width = title.len() + 3;
                                            let tab_end = col_pos + tab_width;

                                            if (mouse_event.column as usize) >= col_pos && (mouse_event.column as usize) < tab_end {
                                                // Check if clicking the (X) close button
                                                let close_start = tab_end - 4; // " (X)" is 4 chars
                                                if (mouse_event.column as usize) >= close_start && title.contains("(X)") {
                                                    // Close this DM tab
                                                    let tab_content = s.dynamic_tabs.tab_index_to_content(idx);
                                                    if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content {
                                                        if let Some(closed_idx) = s.dynamic_tabs.remove_dm_tab(&peer_id) {
                                                            s.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                                                            p2p_app::log_debug(&logs, format!("Closed DM tab via mouse: {}", peer_id));
                                                        }
                                                    }
                                                } else if idx != s.active_tab {
                                                    // Switch tab
                                                    s.active_tab = idx;
                                                    s.chat_scroll_offset = 0;
                                                    p2p_app::log_debug(&logs, format!("Switched to tab {} via mouse click", s.active_tab));
                                                }
                                                break;
                                            }

                                            col_pos = tab_end;
                                        }
                                    } else if mouse_event.row > 2 && mouse_event.row < 16 {
                                        // Peer list click - rows 3+ (accounting for tab row 0, peer info row 1, border row 2)
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                            // Calculate which peer was clicked (row 3 = first peer, row 4 = second, etc.)
                                            let peer_row = (mouse_event.row as usize).saturating_sub(3);
                                            if peer_row < s.peers.len() {
                                                // Update selection and open DM
                                                s.peer_selection = peer_row;
                                                if let Some((peer_id, _, _)) = s.peers.iter().nth(peer_row) {
                                                    let peer_id_clone = peer_id.clone();
                                                    let tab_idx = s.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
                                                    s.active_tab = tab_idx;
                                                    p2p_app::log_debug(&logs, format!("Opened DM with peer via mouse: {}", peer_id_clone));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
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
