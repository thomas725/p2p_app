use super::constants::{
    MAX_DM_HISTORY, MAX_MESSAGE_HISTORY, MAX_PEERS, PAGE_SIZE, WHEEL_SCROLL_LINES,
};
use super::input_handler::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, SwarmEvent, p2plog_debug};
use std::time::SystemTime;
use tokio::sync::mpsc;

enum Event {
    Input(InputEvent),
    SwarmEvent(SwarmEvent),
}

async fn handle_navigation_key(key_code: crossterm::event::KeyCode, state: &mut super::state::AppState) {
    match key_code {
        crossterm::event::KeyCode::Tab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = (state.active_tab + 1) % max_tabs;
            state.chat_scroll_offset = 0;
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        crossterm::event::KeyCode::BackTab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = if state.active_tab == 0 { max_tabs - 1 } else { state.active_tab - 1 };
            state.chat_scroll_offset = 0;
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        _ => {}
    }
}

async fn send_message(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    text: String,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    let (topic_str, own_nickname) = (state.topic_str.clone(), state.own_nickname.clone());
    let is_direct = matches!(tab_content, p2p_app::tui_tabs::TabContent::Direct(_));
    let dm_target_peer_id: Option<String> = if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
        Some(pid.clone())
    } else {
        None
    };
    let ts = p2p_app::format_system_time(SystemTime::now());

    if is_direct {
        if let Some(ref peer_id) = dm_target_peer_id {
            let msg = format!("{} [{}] {}", ts, own_nickname, &text);
            let dm_msgs = state.dm_messages.entry(peer_id.clone()).or_default();
            dm_msgs.push_back(msg);
            if dm_msgs.len() > MAX_DM_HISTORY {
                dm_msgs.pop_front();
            }
            p2plog_debug(format!("Sent DM to {}: {}", peer_id, text));
        }
    } else {
        let msg = format!("{} [{}] {}", ts, own_nickname, &text);
        state.messages.push_back((msg, None));
        if state.messages.len() > MAX_MESSAGE_HISTORY {
            state.messages.pop_front();
        }
        p2plog_debug(format!("Sent broadcast: {}", text));
    }

    state.chat_input = ratatui_textarea::TextArea::default();

    if is_direct {
        if let Some(peer_id) = dm_target_peer_id.clone() {
            let _ = swarm_cmd_tx.send(SwarmCommand::SendDm { peer_id, content: text.clone() }).await;
        }
    } else {
        let _ = swarm_cmd_tx.send(SwarmCommand::Publish(text.clone())).await;
    }

    if let Err(e) = p2p_app::save_message(&text, None, &topic_str, is_direct, dm_target_peer_id.as_deref()) {
        p2plog_debug(format!("Failed to save message: {}", e));
    }
}

fn handle_mouse_scroll(state: &mut super::state::AppState, scroll_dir: &str) {
    let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
    match scroll_dir {
        "up" => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            } else if state.chat_scroll_offset >= WHEEL_SCROLL_LINES {
                state.chat_scroll_offset -= WHEEL_SCROLL_LINES;
            } else {
                state.chat_scroll_offset = 0;
            }
        }
        "down" => {
            state.chat_auto_scroll = false;
            state.chat_scroll_offset = (state.chat_scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
        }
        _ => {}
    }
}

fn handle_tab_click(state: &mut super::state::AppState, mouse_column: u16, tab_titles: &[String]) -> bool {
    let mut col_pos = 0;
    for (idx, title) in tab_titles.iter().enumerate() {
        let tab_width = title.len() + 3;
        let tab_end = col_pos + tab_width;
        if (mouse_column as usize) >= col_pos && (mouse_column as usize) < tab_end {
            let close_start = tab_end - 4;
            if (mouse_column as usize) >= close_start && title.contains("(X)") {
                let tab_content = state.dynamic_tabs.tab_index_to_content(idx);
                if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content
                    && let Some(closed_idx) = state.dynamic_tabs.remove_dm_tab(&peer_id)
                {
                    state.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                    p2plog_debug(format!("Closed DM tab via mouse: {}", peer_id));
                }
                return true;
            } else if idx != state.active_tab {
                state.active_tab = idx;
                state.chat_scroll_offset = 0;
                p2plog_debug(format!("Switched to tab {} via mouse click", state.active_tab));
                return true;
            }
            break;
        }
        col_pos = tab_end;
    }
    false
}

fn handle_peer_row_click(state: &mut super::state::AppState, row: u16) {
    let peer_row = (row as usize).saturating_sub(3);
    if peer_row < state.peers.len() {
        if let Some((peer_id, _, _)) = state.peers.get(peer_row) {
            let peer_id_clone = peer_id.clone();
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer via mouse: {}", peer_id_clone));
        }
    }
}

async fn handle_enter_key(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    shift_held: bool,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    if shift_held {
        if tab_content.is_input_enabled() {
            state.chat_input.insert_str("\n");
        }
    } else if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
        if let Some(peer_id) = state.peers.get(state.peer_selection).map(|(id, _, _)| id.clone()) {
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer: {}", peer_id));
        }
    } else if tab_content.is_input_enabled() {
        let text: String = state.chat_input.lines().join("\n");
        if !text.trim().is_empty() {
            send_message(state, swarm_cmd_tx, text, tab_content).await;
        }
    }
}

fn handle_close_dm_tab(state: &mut super::state::AppState, tab_content: p2p_app::tui_tabs::TabContent) {
    if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content {
        if let Some(closed_idx) = state.dynamic_tabs.remove_dm_tab(&peer_id) {
            state.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
            state.peer_selection = 0;
            p2plog_debug(format!("Closed DM tab with peer: {}", peer_id));
        }
    }
}

fn toggle_mouse_capture(state: &mut super::state::AppState) {
    use ratatui::crossterm::execute;
    state.mouse_capture = !state.mouse_capture;
    let mode = if state.mouse_capture { "enabled" } else { "disabled" };
    p2plog_debug(format!("Mouse capture {}", mode));
    let mut stdout = std::io::stdout();
    let _ = if state.mouse_capture {
        execute!(stdout, crossterm::event::EnableMouseCapture)
    } else {
        execute!(stdout, crossterm::event::DisableMouseCapture)
    };
}

fn handle_mouse_left_click(
    state: &mut super::state::AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_chat_tab: bool,
) {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        handle_tab_click(state, mouse_column, &tab_titles);
    } else if mouse_row > 2 && mouse_row < 16 && is_chat_tab {
        handle_peer_row_click(state, mouse_row);
    }
}

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
            if !s.peers.iter().any(|(id, _, _)| id == &peer_id) && s.peers.len() < MAX_PEERS {
                s.peers.push_front((peer_id, p2p_app::now_timestamp(), p2p_app::now_timestamp()));
            }
        }
        #[cfg(feature = "mdns")]
        SwarmEvent::PeerExpired { peer_id } => {
            p2plog_debug(format!("Peer expired: {}", peer_id));
        }
    }
}

async fn process_input_event(
    input_event: InputEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) {
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
                crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::BackTab => {
                    handle_navigation_key(key_event.code, &mut s).await;
                }
                crossterm::event::KeyCode::Up
                | crossterm::event::KeyCode::Down
                | crossterm::event::KeyCode::PageUp
                | crossterm::event::KeyCode::PageDown
                | crossterm::event::KeyCode::Home
                | crossterm::event::KeyCode::End => {
                    handle_scroll_key(key_event.code, &mut s).await;
                }
                crossterm::event::KeyCode::F(12) => {
                    toggle_mouse_capture(&mut s);
                }
                crossterm::event::KeyCode::Enter => {
                    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                    let shift_held = key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);
                    handle_enter_key(&mut s, swarm_cmd_tx, shift_held, tab_content).await;
                }
                crossterm::event::KeyCode::Char('w') if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) => {
                    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                    handle_close_dm_tab(&mut s, tab_content);
                }
                _ => {
                    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                    if tab_content.is_input_enabled() {
                        s.chat_input.input(key_event);
                    }
                }
            }
            drop(s);
            let _ = render_tx.send(RenderEvent).await;
        }
        InputEvent::Mouse(mouse_event) => {
            let mut s = state.lock().await;
            let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
            let is_chat_tab = !matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers);

            match mouse_event.kind {
                crossterm::event::MouseEventKind::ScrollUp if is_chat_tab => {
                    handle_mouse_scroll(&mut s, "up");
                }
                crossterm::event::MouseEventKind::ScrollDown if is_chat_tab => {
                    handle_mouse_scroll(&mut s, "down");
                }
                crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                    handle_mouse_left_click(&mut s, mouse_event.row, mouse_event.column, is_chat_tab);
                }
                _ => {}
            }
            drop(s);
            let _ = render_tx.send(RenderEvent).await;
        }
    }
}

async fn handle_scroll_key(key_code: crossterm::event::KeyCode, state: &mut super::state::AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
        match key_code {
            crossterm::event::KeyCode::Up => {
                state.peer_selection = state.peer_selection.saturating_sub(1);
            }
            crossterm::event::KeyCode::Down => {
                if state.peer_selection < state.peers.len().saturating_sub(1) {
                    state.peer_selection += 1;
                }
            }
            _ => {}
        }
    } else {
        match key_code {
            crossterm::event::KeyCode::Up => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                if state.chat_scroll_offset > 0 {
                    state.chat_scroll_offset -= 1;
                }
            }
            crossterm::event::KeyCode::Down => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
                if state.chat_scroll_offset < max_offset {
                    state.chat_scroll_offset += 1;
                }
            }
            crossterm::event::KeyCode::PageUp => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                state.chat_scroll_offset = state.chat_scroll_offset.saturating_sub(PAGE_SIZE);
            }
            crossterm::event::KeyCode::PageDown => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
                state.chat_scroll_offset = (state.chat_scroll_offset + PAGE_SIZE).min(max_offset);
            }
            crossterm::event::KeyCode::Home => {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = 0;
            }
            crossterm::event::KeyCode::End => {
                state.chat_auto_scroll = true;
            }
            _ => {}
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

        let send_render = || async {
            let _ = render_tx.send(RenderEvent).await;
        };

        loop {
            let event = tokio::select! {
                Some(input_event) = input_rx.recv() => Some(Event::Input(input_event)),
                Some(swarm_event) = swarm_event_rx.recv() => Some(Event::SwarmEvent(swarm_event)),
                else => None,
            };

            match event {
                Some(Event::Input(input_event)) => {
                    process_input_event(input_event, &state, &swarm_cmd_tx, &render_tx).await;
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
