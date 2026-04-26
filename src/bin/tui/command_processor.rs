use super::constants::{
    MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, MAX_PEERS, PAGE_SIZE, WHEEL_SCROLL_LINES,
};
use super::input_handler::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, SwarmEvent, p2plog_debug};
use std::time::SystemTime;
use tokio::sync::mpsc;

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

        let send_render = || async {
            let _ = render_tx.send(RenderEvent).await;
        };

        loop {
            tokio::select! {
                            Some(input_event) = input_rx.recv() => {
                                match input_event {
                                    InputEvent::Key(key_event) => {
                                        if key_event.code == crossterm::event::KeyCode::Esc
                                            || (key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                                                && key_event.code == crossterm::event::KeyCode::Char('q'))
                                        {
                                            p2plog_debug("Exit signal received".to_string());
                                            return;
                                        }

                                        let mut s = state.lock().await;
                                        match key_event.code {
                                            crossterm::event::KeyCode::Tab => {
                                                let max_tabs = s.dynamic_tabs.total_tab_count();
                                                s.active_tab = (s.active_tab + 1) % max_tabs;
                                                s.chat_scroll_offset = 0;
                                                p2plog_debug(format!("Switched to tab {}", s.active_tab));
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::BackTab => {
                                                let max_tabs = s.dynamic_tabs.total_tab_count();
                                                s.active_tab = if s.active_tab == 0 { max_tabs - 1 } else { s.active_tab - 1 };
                                                s.chat_scroll_offset = 0;
                                                p2plog_debug(format!("Switched to tab {}", s.active_tab));
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::Up => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    s.peer_selection = s.peer_selection.saturating_sub(1);
                                                } else {
                                                    if s.chat_auto_scroll {
                                                        s.chat_auto_scroll = false;
                                                        if s.messages.len() > s.visible_message_count {
                                                            s.chat_scroll_offset = s.max_scroll_offset;
                                                        }
                                                    } else if s.chat_scroll_offset > 0 {
                                                        s.chat_scroll_offset -= 1;
                                                    }
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::Down => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    if s.peer_selection < s.peers.len().saturating_sub(1) {
                                                        s.peer_selection += 1;
                                                    }
                                                } else {
                                                    s.chat_auto_scroll = false;
                                                    if s.messages.len() > s.visible_message_count {
                                                        let max_offset = s.max_scroll_offset;
                                                        if s.chat_scroll_offset < max_offset {
                                                            s.chat_scroll_offset += 1;
                                                        }
                                                    }
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::PageUp => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    if s.chat_auto_scroll {
                                                        s.chat_auto_scroll = false;
                                                        if s.messages.len() > s.visible_message_count {
                                                            s.chat_scroll_offset = s.max_scroll_offset;
                                                        }
                                                    } else {
                                                        s.chat_scroll_offset = s.chat_scroll_offset.saturating_sub(PAGE_SIZE);
                                                    }
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::PageDown => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    s.chat_auto_scroll = false;
                                                    if s.messages.len() > s.visible_message_count {
                                                        let max_offset = s.max_scroll_offset;
                                                        s.chat_scroll_offset = (s.chat_scroll_offset + PAGE_SIZE).min(max_offset);
                                                    }
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::Home => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    s.chat_auto_scroll = false;
                                                    s.chat_scroll_offset = s.max_scroll_offset;
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::End => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    s.chat_auto_scroll = true;
                                                }
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::F(12) => {
                                                s.mouse_capture = !s.mouse_capture;
                                                let mode = if s.mouse_capture { "enabled" } else { "disabled" };
                                                p2plog_debug(format!("Mouse capture {}", mode));
                                                use ratatui::crossterm::execute;
                                                let mut stdout = std::io::stdout();
                                                let _ = if s.mouse_capture {
                                                    execute!(stdout, crossterm::event::EnableMouseCapture)
                                                } else {
                                                    execute!(stdout, crossterm::event::DisableMouseCapture)
                                                };
                                                drop(s);
                                            }
                                            crossterm::event::KeyCode::Enter => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT) {
                                                    if tab_content.is_input_enabled() {
                                                        s.chat_input.insert_str("\n");
                                                    }
                                                    drop(s);
                                                } else if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
                                                    if let Some(peer_id) = s.peers.get(s.peer_selection).map(|(id, _, _)| id.clone()) {
                                                        let tab_idx = s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                                        s.active_tab = tab_idx;
                                                        p2plog_debug(format!("Opened DM with peer: {}", peer_id));
                                                    }
                                                    drop(s);
                                                } else if tab_content.is_input_enabled() {
                                                    let text: String = s.chat_input.lines().join("\n");
                                                    if !text.trim().is_empty() {
                                                        let (topic_str, own_nickname) = (s.topic_str.clone(), s.own_nickname.clone());
                                                        let is_direct = matches!(tab_content, p2p_app::tui_tabs::TabContent::Direct(_));
                                                        let dm_target_peer_id: Option<String> = if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
                                                            Some(pid.clone())
                                                        } else {
                                                            None
                                                        };
                                                        let ts = p2p_app::format_system_time(SystemTime::now());
                                                        if is_direct {
                                                            if let Some(ref peer_id) = dm_target_peer_id {
                                                                let msg = format!("{} [{}] {}", ts, own_nickname, text);
                                                                let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                                                dm_msgs.push_back(msg);
                                                                if dm_msgs.len() > MAX_DM_HISTORY {
                                                                    dm_msgs.pop_front();
                                                                }
                                                                p2plog_debug(format!("Sent DM to {}: {}", peer_id, text));
                                                            }
                                                        } else {
                                                            let msg = format!("{} [{}] {}", ts, own_nickname, text);
                                                            s.messages.push_back((msg, None));
                                                            if s.messages.len() > MAX_MESSAGE_HISTORY {
                                                                s.messages.pop_front();
                                                            }
                                                            p2plog_debug(format!("Sent broadcast: {}", text));
                                                        }
                                                        s.chat_input = ratatui_textarea::TextArea::default();
                                                        let (cmd_tx, is_direct_send, dm_target_send, topic_str_for_db, dm_target_for_db) = (
                                                            swarm_cmd_tx.clone(),
                                                            is_direct,
                                                            dm_target_peer_id.clone(),
                                                            topic_str,
                                                            dm_target_peer_id,
                                                        );
                                                        drop(s);
                                                        if is_direct_send {
                                                            if let Some(peer_id) = dm_target_send {
                                                                let _ = cmd_tx.send(SwarmCommand::SendDm { peer_id, content: text.clone() }).await;
                                                            }
                                                        } else {
                                                            let _ = cmd_tx.send(SwarmCommand::Publish(text.clone())).await;
                                                        }
                                                        if let Err(e) = p2p_app::save_message(&text, None, &topic_str_for_db, is_direct_send, dm_target_for_db.as_deref()) {
                                                            p2plog_debug(format!("Failed to save message: {}", e));
                                                        }
                                                    } else {
                                                        drop(s);
                                                    }
                                                } else {
                                                    drop(s);
                                                }
                                            }
                                            crossterm::event::KeyCode::Char('w') if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content
                                                    && let Some(closed_idx) = s.dynamic_tabs.remove_dm_tab(&peer_id) {
                                                        s.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                                                        s.peer_selection = 0;
                                                        p2plog_debug(format!("Closed DM tab with peer: {}", peer_id));
                                                    }
                                                drop(s);
                                            }
                                            _ => {
                                                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                                if tab_content.is_input_enabled() {
                                                    s.chat_input.input(key_event);
                                                }
                                                drop(s);
                                            }
                                        }
                                        send_render().await;
                                    }
                                    InputEvent::Mouse(mouse_event) => {
                                        let mut s = state.lock().await;
                                        let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                                        let is_chat_tab = !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers);

                                        match mouse_event.kind {
                                            crossterm::event::MouseEventKind::ScrollUp if is_chat_tab => {
            if s.chat_auto_scroll {
                                                        s.chat_auto_scroll = false;
                                                        if s.messages.len() > s.visible_message_count {
                                                            s.chat_scroll_offset = s.max_scroll_offset;
                                                        }
                                                } else if s.chat_scroll_offset >= WHEEL_SCROLL_LINES {
                                                    s.chat_scroll_offset -= WHEEL_SCROLL_LINES;
                                                } else {
                                                    s.chat_scroll_offset = 0;
                                                }
                                            }
                                            crossterm::event::MouseEventKind::ScrollDown if is_chat_tab => {
                                                s.chat_auto_scroll = false;
                                                if s.messages.len() > s.visible_message_count {
                                                    let max_offset = s.max_scroll_offset;
                                                    s.chat_scroll_offset = (s.chat_scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
                                                }
                                            }
                                            crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                                                if mouse_event.row == 0 {
                                                    let tab_titles = s.dynamic_tabs.all_titles();
                                                    let mut col_pos = 0;
                                                    for (idx, title) in tab_titles.iter().enumerate() {
                                                        let tab_width = title.len() + 3;
                                                        let tab_end = col_pos + tab_width;
                                                        if (mouse_event.column as usize) >= col_pos && (mouse_event.column as usize) < tab_end {
                                                            let close_start = tab_end - 4;
                                                            if (mouse_event.column as usize) >= close_start && title.contains("(X)") {
                                                                let tab_content = s.dynamic_tabs.tab_index_to_content(idx);
                                                                if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content
                                                                    && let Some(closed_idx) = s.dynamic_tabs.remove_dm_tab(&peer_id) {
                                                                        s.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                                                                        p2plog_debug(format!("Closed DM tab via mouse: {}", peer_id));
                                                                    }
                                                            } else if idx != s.active_tab {
                                                                s.active_tab = idx;
                                                                s.chat_scroll_offset = 0;
                                                                p2plog_debug(format!("Switched to tab {} via mouse click", s.active_tab));
                                                            }
                                                            break;
                                                        }
                                                        col_pos = tab_end;
                                                    }
                                                } else if mouse_event.row > 2 && mouse_event.row < 16 && is_chat_tab {
                                                    let peer_row = (mouse_event.row as usize).saturating_sub(3);
                                                    if peer_row < s.peers.len() {
                                                        if let Some((peer_id, _, _)) = s.peers.get(peer_row) {
                                                            let peer_id_clone = peer_id.clone();
                                                            let tab_idx = s.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
                                                            s.active_tab = tab_idx;
                                                            p2plog_debug(format!("Opened DM with peer via mouse: {}", peer_id_clone));
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                        drop(s);
                                    }
                                }
                            }
                            Some(swarm_event) = swarm_event_rx.recv() => {
                                match swarm_event {
                                    SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
                                        let mut s = state.lock().await;
                                        let ts = p2p_app::format_system_time(SystemTime::now());
                                        let sender_display = p2p_app::peer_display_name(&peer_id, &s.local_nicknames, &s.received_nicknames);
                                        let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
                                        s.messages.push_back((msg, Some(peer_id.clone())));
                                        if s.messages.len() > MAX_MESSAGE_HISTORY {
                                            s.messages.pop_front();
                                        }
                                        if s.active_tab != 0 {
                                            s.unread_broadcasts += 1;
                                        }
                                        p2plog_debug(format!("Broadcast from {}: {}", sender_display, content));
                                        if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, false, None) {
                                            p2plog_debug(format!("Failed to save: {}", e));
                                        }
                                        drop(s);
                                    }
                                    SwarmEvent::DirectMessage { content, peer_id, latency } => {
                                        let mut s = state.lock().await;
                                        let ts = p2p_app::format_system_time(SystemTime::now());
                                        let sender_display = p2p_app::peer_display_name(&peer_id, &s.local_nicknames, &s.received_nicknames);
                                        let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                        let msg = format!("{} {} [{}] {}", ts, latency.unwrap_or_default(), sender_display, content);
                                        dm_msgs.push_back(msg);
                                        if dm_msgs.len() > MAX_DM_HISTORY {
                                            dm_msgs.pop_front();
                                        }
                                        *s.unread_dms.entry(peer_id.clone()).or_insert(0) += 1;
                                        s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                        p2plog_debug(format!("DM from {}: {}", sender_display, content));
                                        if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, true, Some(&peer_id)) {
                                            p2plog_debug(format!("Failed to save DM: {}", e));
                                        }
                                        drop(s);
                                    }
                                    SwarmEvent::PeerConnected(peer_id) => {
                                        let mut s = state.lock().await;
                                        s.concurrent_peers += 1;
                                        p2plog_debug(format!("Peer connected: {} (total: {})", peer_id, s.concurrent_peers));
                                        if !s.peers.iter().any(|(id, _, _)| id == &peer_id) && s.peers.len() < MAX_PEERS {
                                            if let Ok(peer) = p2p_app::save_peer(&peer_id, &[peer_id.clone()]) {
                                                let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                                                let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                                                s.peers.push_front((peer_id, first_seen, last_seen));
                                            }
                                        }
                                        drop(s);
                                    }
                                    SwarmEvent::PeerDisconnected(peer_id) => {
                                        let mut s = state.lock().await;
                                        s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
                                        p2plog_debug(format!("Peer disconnected: {} (total: {})", peer_id, s.concurrent_peers));
                                        drop(s);
                                    }
                                    SwarmEvent::ListenAddrEstablished(addr) => {
                                        p2plog_debug(format!("Listening on: {}", addr));
                                    }
                                    #[cfg(feature = "mdns")]
                                    SwarmEvent::PeerDiscovered { peer_id, .. } => {
                                        let mut s = state.lock().await;
                                        if !s.peers.iter().any(|(id, _, _)| id == &peer_id) && s.peers.len() < MAX_PEERS {
                                            s.peers.push_front((peer_id, p2p_app::now_timestamp(), p2p_app::now_timestamp()));
                                        }
                                        drop(s);
                                    }
                                    #[cfg(feature = "mdns")]
                                    SwarmEvent::PeerExpired { peer_id } => {
                                        p2plog_debug(format!("Peer expired: {}", peer_id));
                                    }
                                }
                                send_render().await;
                            }
                        }
        }
    });

    (handle, cmd_tx_for_return)
}
