use super::constants::{PAGE_SIZE, WHEEL_SCROLL_LINES, MAX_DM_HISTORY};
use super::input_handler::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, p2plog_debug};
use ratatui_textarea::TextArea;
use std::collections::VecDeque;
use tokio::sync::mpsc;

/// Handles tab navigation (Tab and BackTab keys)
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

/// Handles scroll keys (arrow keys, Page Up/Down, Home, End)
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

/// Handles mouse wheel scrolling
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

/// Handles Enter key (send message or multi-line input)
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
            load_dm_messages(state, &peer_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer: {}", peer_id));
        }
    } else if tab_content.is_input_enabled() {
        let text: String = state.chat_input.lines().join("\n");
        if !text.trim().is_empty() {
            super::message_handlers::send_message(state, swarm_cmd_tx, text, tab_content).await;
        }
    }
}

/// Handles Ctrl+W (close DM tab)
fn handle_close_dm_tab(state: &mut super::state::AppState, tab_content: p2p_app::tui_tabs::TabContent) {
    if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content {
        if let Some(closed_idx) = state.dynamic_tabs.remove_dm_tab(&peer_id) {
            state.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
            state.peer_selection = 0;
            p2plog_debug(format!("Closed DM tab with peer: {}", peer_id));
        }
    }
}

/// Toggles mouse capture mode (F12)
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

/// Handles tab bar clicks and close button
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

/// Loads DM messages from database for a peer
fn load_dm_messages(state: &mut super::state::AppState, peer_id: &str) {
    if !state.dm_messages.contains_key(peer_id) {
        if let Ok(db_messages) = p2p_app::load_direct_messages(peer_id, MAX_DM_HISTORY) {
            let mut messages = VecDeque::new();
            for msg in db_messages.iter().rev() {
                let ts = p2p_app::format_peer_datetime(msg.created_at);
                let sender_display = msg
                    .peer_id
                    .as_ref()
                    .map(|p| p2p_app::peer_display_name(p, &state.local_nicknames, &state.received_nicknames))
                    .unwrap_or_else(|| state.own_nickname.clone());
                messages.push_back(format!("{} [{}] {}", ts, sender_display, msg.content));
            }
            state.dm_messages.insert(peer_id.to_string(), messages);
            p2plog_debug(format!("Loaded {} DM messages for {}", db_messages.len(), peer_id));
        }
    }
}

/// Handles peer row clicks in the Peers tab
fn handle_peer_row_click(state: &mut super::state::AppState, row: u16) {
    let peer_row = (row as usize).saturating_sub(3);
    if peer_row < state.peers.len() {
        if let Some((peer_id, _, _)) = state.peers.get(peer_row) {
            let peer_id_clone = peer_id.clone();
            load_dm_messages(state, &peer_id_clone);
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer via mouse: {}", peer_id_clone));
        }
    }
}

/// Handles clicks on messages in the chat view
fn handle_message_click(state: &mut super::state::AppState, row: u16) {
    let click_row = row as usize;
    let mut current_row = 3;
    let mut message_idx = 0;

    for line_count in &state.chat_message_lines {
        if current_row + line_count > click_row {
            break;
        }
        current_row += line_count;
        message_idx += 1;
    }

    let peer_id = state.messages.iter().skip(state.chat_message_offset).nth(message_idx).map(|(_, pid)| pid.clone());

    match peer_id {
        Some(Some(sender_id)) => {
            load_dm_messages(state, &sender_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(sender_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with sender via message click: {}", sender_id));
        }
        Some(None) => {
            state.editing_nickname = true;
            let mut textarea = TextArea::default();
            textarea.insert_str(&state.own_nickname);
            state.chat_input = textarea;
            p2plog_debug("Started editing nickname".to_string());
        }
        None => {}
    }
}

/// Handles left mouse button clicks
fn handle_mouse_left_click(
    state: &mut super::state::AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_peers_tab: bool,
) {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        handle_tab_click(state, mouse_column, &tab_titles);
    } else {
        let max_row = state.chat_area_height.saturating_sub(1) as u16;
        if mouse_row > 2 && mouse_row < max_row {
            if is_peers_tab {
                handle_peer_row_click(state, mouse_row);
            } else {
                handle_message_click(state, mouse_row);
            }
        }
    }
}

/// Processes keyboard input events, returns true if exit requested
async fn process_key_event(
    key_event: crossterm::event::KeyEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) -> bool {
    if key_event.code == crossterm::event::KeyCode::Esc {
        let mut s = state.lock().await;
        if s.editing_nickname {
            s.editing_nickname = false;
            s.chat_input = TextArea::default();
            p2plog_debug("Cancelled nickname edit".to_string());
            drop(s);
            let _ = render_tx.send(RenderEvent).await;
            return false;
        }
        drop(s);
        p2plog_debug("Exit signal received".to_string());
        return true;
    }

    if key_event.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
        && key_event.code == crossterm::event::KeyCode::Char('q')
    {
        p2plog_debug("Exit signal received".to_string());
        return true;
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
            if s.editing_nickname {
                let new_nickname = s.chat_input.lines().join("\n");
                if !new_nickname.trim().is_empty() {
                    s.own_nickname = new_nickname.clone();
                    p2plog_debug(format!("Updated nickname to: {}", new_nickname));
                }
                s.editing_nickname = false;
                s.chat_input = TextArea::default();
            } else {
                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                let shift_held = key_event.modifiers.contains(crossterm::event::KeyModifiers::SHIFT);
                handle_enter_key(&mut s, swarm_cmd_tx, shift_held, tab_content).await;
            }
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
    false
}

/// Processes mouse input events
async fn process_mouse_event(
    mouse_event: crossterm::event::MouseEvent,
    state: &SharedState,
    render_tx: &mpsc::Sender<RenderEvent>,
) {
    let mut s = state.lock().await;
    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    let is_peers_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers);
    let is_message_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Chat | p2p_app::tui_tabs::TabContent::Direct(_));

    match mouse_event.kind {
        crossterm::event::MouseEventKind::ScrollUp if is_message_tab => {
            handle_mouse_scroll(&mut s, "up");
        }
        crossterm::event::MouseEventKind::ScrollDown if is_message_tab => {
            handle_mouse_scroll(&mut s, "down");
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            handle_mouse_left_click(&mut s, mouse_event.row, mouse_event.column, is_peers_tab);
        }
        _ => {}
    }
    drop(s);
    let _ = render_tx.send(RenderEvent).await;
}

/// Main input event processor - routes keyboard and mouse events
/// Returns true if exit was requested, false otherwise
pub async fn process_input_event(
    input_event: InputEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) -> bool {
    match input_event {
        InputEvent::Key(key_event) => {
            process_key_event(key_event, state, swarm_cmd_tx, render_tx).await
        }
        InputEvent::Mouse(mouse_event) => {
            process_mouse_event(mouse_event, state, render_tx).await;
            false
        }
    }
}
