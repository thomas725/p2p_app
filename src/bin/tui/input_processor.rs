use super::event_source::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, p2plog_debug};
use tokio::sync::mpsc;

use crate::tui::click_handlers::{handle_mouse_left_click, load_dm_messages};
use crate::tui::scroll_handlers::{handle_mouse_scroll, handle_navigation_key, handle_scroll_key};

fn update_dm_transcript_labels(
    dm_messages: &mut std::collections::HashMap<String, std::collections::VecDeque<String>>,
    peer_id: &str,
    old_nick: &str,
    new_nick: &str,
) {
    if let Some(dm_msgs) = dm_messages.get_mut(peer_id) {
        let from = format!("[{}] ", old_nick);
        let to = format!("[{}] ", new_nick);
        for line in dm_msgs.iter_mut() {
            if line.contains(&from) {
                *line = line.replace(&from, &to);
            }
        }
    }
}

/// Toggles mouse capture mode (F12)
fn toggle_mouse_capture(state: &mut super::state::AppState) {
    use ratatui::crossterm::execute;
    state.mouse_capture = !state.mouse_capture;
    let mode = if state.mouse_capture {
        "enabled"
    } else {
        "disabled"
    };
    p2plog_debug(format!("Mouse capture {}", mode));
    let mut stdout = std::io::stdout();
    let _ = if state.mouse_capture {
        execute!(stdout, crossterm::event::EnableMouseCapture)
    } else {
        execute!(stdout, crossterm::event::DisableMouseCapture)
    };
}

async fn handle_nickname_submission(
    state: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let new_nickname = state.chat_input.lines().join("\n");
    if new_nickname.trim().is_empty() {
        state.cancel_nickname_edit();
        return;
    }

    if let Some(peer_id) = state.editing_nickname_peer.clone() {
        let old_nickname = state
            .self_nicknames_for_peers
            .get(&peer_id)
            .cloned()
            .unwrap_or_else(|| state.own_nickname.clone());
        state
            .self_nicknames_for_peers
            .insert(peer_id.clone(), new_nickname.clone());
        let _ = p2p_app::set_peer_self_nickname_for_peer(&peer_id, &new_nickname);
        let _ = swarm_cmd_tx
            .send(SwarmCommand::SendDm {
                peer_id: peer_id.clone(),
                content: String::new(),
                nickname: Some(new_nickname.clone()),
                msg_id: None,
                ack_for: None,
            })
            .await;
        update_dm_transcript_labels(
            &mut state.dm_messages,
            &peer_id,
            &old_nickname,
            &new_nickname,
        );
        p2plog_debug(format!("Updated per-peer nickname to: {}", new_nickname));
    } else {
        let old_nickname = state.own_nickname.clone();
        state.own_nickname = new_nickname.clone();
        let _ = p2p_app::set_self_nickname(&new_nickname);
        for (peer_id, _, _) in state.peers.iter() {
            if state.self_nicknames_for_peers.contains_key(peer_id) {
                continue;
            }
            let _ = swarm_cmd_tx
                .send(SwarmCommand::SendDm {
                    peer_id: peer_id.clone(),
                    content: String::new(),
                    nickname: Some(new_nickname.clone()),
                    msg_id: None,
                    ack_for: None,
                })
                .await;
        }
        let peer_ids: Vec<String> = state.dm_messages.keys().cloned().collect();
        for peer_id in peer_ids {
            if state.self_nicknames_for_peers.contains_key(&peer_id) {
                continue;
            }
            update_dm_transcript_labels(
                &mut state.dm_messages,
                &peer_id,
                &old_nickname,
                &new_nickname,
            );
        }
        p2plog_debug(format!("Updated broadcast nickname to: {}", new_nickname));
    }
    state.cancel_nickname_edit();
}

/// Handles Ctrl+W (close DM tab)
fn handle_close_dm_tab(
    state: &mut super::state::AppState,
    tab_content: p2p_app::tui_tabs::TabContent,
) {
    if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content
        && let Some(closed_idx) = state.dynamic_tabs.remove_dm_tab(&peer_id)
    {
        state.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
        state.peer_selection = 0;
        p2plog_debug(format!("Closed DM tab with peer: {}", peer_id));
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
        if let Some(peer_id) = state
            .peers
            .get(state.peer_selection)
            .map(|(id, _, _)| id.clone())
        {
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

/// Processes keyboard input events, returns true if exit requested
async fn process_key_event(
    key_event: crossterm::event::KeyEvent,
    state: &SharedState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
) -> bool {
    if key_event.code == crossterm::event::KeyCode::Esc {
        let mut s = state.lock().await;
        // ESC is "back" (exit is Ctrl+Q). Prefer dismissing transient UI states first.
        if s.popup.is_some() {
            s.popup = None;
        }
        if s.editing_nickname {
            s.cancel_nickname_edit();
            p2plog_debug("Cancelled nickname edit".to_string());
        } else {
            // Return to broadcast chat.
            s.active_tab = 0;
            s.broadcast_selection = None;
            s.chat_scroll_offset = 0;
            s.chat_auto_scroll = true;
            p2plog_debug("Returned to Broadcast Chat (Esc)".to_string());
        }
        drop(s);
        let _ = render_tx.send(RenderEvent).await;
        return false;
    }

    if key_event
        .modifiers
        .contains(crossterm::event::KeyModifiers::CONTROL)
        && key_event.code == crossterm::event::KeyCode::Char('q')
    {
        p2plog_debug("Exit signal received".to_string());
        return true;
    }

    let mut s = state.lock().await;

    // If a popup is open, any key dismisses it (except we still honor exit keys above).
    if s.popup.is_some() {
        s.popup = None;
        drop(s);
        let _ = render_tx.send(RenderEvent).await;
        return false;
    }

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
                handle_nickname_submission(&mut s, swarm_cmd_tx).await;
            } else {
                let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                let shift_held = key_event
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::SHIFT);
                handle_enter_key(&mut s, swarm_cmd_tx, shift_held, tab_content).await;
            }
        }
        crossterm::event::KeyCode::Char('w')
            if key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
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

    s.last_mouse_row = mouse_event.row;

    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    let is_peers_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers);
    let is_scrollable_tab = matches!(
        tab_content,
        p2p_app::tui_tabs::TabContent::Chat
            | p2p_app::tui_tabs::TabContent::Direct(_)
            | p2p_app::tui_tabs::TabContent::Log
    );
    let (is_dm_tab, peer_id) = if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
        (true, Some(pid.as_str()))
    } else {
        (false, None)
    };

    match mouse_event.kind {
        crossterm::event::MouseEventKind::ScrollUp if is_scrollable_tab => {
            handle_mouse_scroll(&mut s, "up", peer_id);
        }
        crossterm::event::MouseEventKind::ScrollDown if is_scrollable_tab => {
            handle_mouse_scroll(&mut s, "down", peer_id);
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if s.popup.is_some() {
                s.popup = None;
                drop(s);
                let _ = render_tx.send(RenderEvent).await;
                return;
            }
            handle_mouse_left_click(
                &mut s,
                mouse_event.row,
                mouse_event.column,
                is_peers_tab,
                is_dm_tab,
                peer_id,
            );
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
