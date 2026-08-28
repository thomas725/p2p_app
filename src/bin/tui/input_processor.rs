use super::event_source::InputEvent;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::{SwarmCommand, p2plog_debug};
use std::collections::VecDeque;
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
        p2p_app::tui_helpers::relabel_dm_transcript(dm_msgs, old_nick, new_nick);
    }
}

/// Pure: flip the mouse capture flag. Returns the new state.
#[allow(clippy::missing_const_for_fn)]
fn flip_mouse_capture_state(state: &mut super::state::AppState) -> bool {
    state.mouse_capture = !state.mouse_capture;
    state.mouse_capture
}

/// Pure: dismiss the popup if one is open. Returns true if a popup was dismissed.
fn dismiss_popup(state: &mut super::state::AppState) -> bool {
    if state.popup.is_some() {
        state.popup = None;
        true
    } else {
        false
    }
}

/// Pure: extract nickname update data from state. Returns None if the new nickname is empty.
fn prepare_nickname_update(
    state: &super::state::AppState,
) -> Option<(String, String, Option<String>)> {
    let new_nickname = state.chat_input.lines().join("\n");
    if new_nickname.trim().is_empty() {
        return None;
    }
    let (old_nickname, peer_id) = state
        .editing_nickname_peer
        .as_ref()
        .map_or_else(
            || (state.own_nickname.clone(), None),
            |pid| {
                let old = state
                    .self_nicknames_for_peers
                    .get(pid)
                    .cloned()
                    .unwrap_or_else(|| state.own_nickname.clone());
                (old, Some(pid.clone()))
            },
        );
    Some((new_nickname, old_nickname, peer_id))
}

/// Toggles mouse capture mode (F12)
fn toggle_mouse_capture(state: &mut super::state::AppState) {
    use ratatui::crossterm::execute;
    flip_mouse_capture_state(state);
    let mode = if state.mouse_capture {
        "enabled"
    } else {
        "disabled"
    };
    p2plog_debug(format!("Mouse capture {mode}"));
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
    let Some((new_nickname, old_nickname, peer_id)) = prepare_nickname_update(state) else {
        state.cancel_nickname_edit();
        return;
    };

    if let Some(peer_id) = peer_id {
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
        p2plog_debug(format!("Updated per-peer nickname to: {new_nickname}"));
    } else {
        state.own_nickname.clone_from(&new_nickname);
        let _ = p2p_app::set_self_nickname(&new_nickname);
        for p in &state.peers {
            let peer_id = &p.peer_id;
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
        p2plog_debug(format!("Updated broadcast nickname to: {new_nickname}"));
    }
    state.cancel_nickname_edit();
}

/// Handles Ctrl+W (close DM or Peer Info tab)
fn handle_close_dm_tab(
    state: &mut super::state::AppState,
    tab_content: &p2p_app::tui_tabs::TabContent,
) {
    let closed_idx = match tab_content {
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            state.dynamic_tabs.remove_dm_tab(peer_id)
        }
        p2p_app::tui_tabs::TabContent::PeerInfo(peer_id) => {
            state.dynamic_tabs.remove_peer_info_tab(peer_id)
        }
        _ => None,
    };
    if let Some(closed_idx) = closed_idx {
        state.active_tab = if closed_idx > 0 { closed_idx.saturating_sub(1) } else { 0 };
        state.peer_selection = 0;
        p2plog_debug(format!("Closed tab: {tab_content:?}"));
    }
}

/// Handles Esc: dismiss popup / cancel nickname edit / return to broadcast chat.
async fn handle_esc_key(
    state: &SharedState,
    render_tx: &mpsc::Sender<RenderEvent>,
) {
    let mut s = state.lock().await;
    dismiss_popup(&mut s);
    if s.editing_nickname {
        s.cancel_nickname_edit();
        p2plog_debug("Cancelled nickname edit".to_string());
    } else {
        s.active_tab = 0;
        s.broadcast_selection = None;
        s.chat_scroll_offset = 0;
        s.chat_auto_scroll = true;
        p2plog_debug("Returned to Broadcast Chat (Esc)".to_string());
    }
    drop(s);
    let _ = render_tx.send(RenderEvent).await;
}

/// Opens the Peer Info tab for the selected peer (Peers tab), DM partner,
/// or the sender of the selected chat/log message.
fn open_peer_info_for_active_tab(state: &mut super::state::AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    let peer = match &tab_content {
        p2p_app::tui_tabs::TabContent::Peers => {
            state.peers.get(state.peer_selection).map(|p| p.peer_id.clone())
        }
        p2p_app::tui_tabs::TabContent::Direct(pid) => Some(pid.clone()),
        p2p_app::tui_tabs::TabContent::Chat | p2p_app::tui_tabs::TabContent::Log => state
            .broadcast_selection
            .and_then(|idx| state.messages.get(idx))
            .and_then(|m| m.sender_peer_id.clone()),
        _ => None,
    };
    if let Some(peer_id) = peer {
        state.active_tab = state.dynamic_tabs.add_peer_info_tab(peer_id);
    }
}

/// Re-sorts the peer list by the currently active column/order, preserving the
/// selected peer (by id).
fn resort_peers(state: &mut super::state::AppState) {
    let sender_ids: VecDeque<Option<String>> =
        state.messages.iter().map(|m| m.sender_peer_id.clone()).collect();
    state.peer_selection = p2p_app::tui_helpers::sort_peers_by_column(
        &mut state.peers,
        &state.dm_messages,
        &sender_ids,
        state.peer_sort_column,
        state.peer_sort_ascending,
        state.peer_selection,
    );
}

/// Sets the peers-table sort column, toggling direction if the same column is
/// chosen again (mirroring Flutter's `_PeerList` header tap behavior), and
/// re-sorts the peer list while preserving the selected peer.
fn apply_peer_sort(state: &mut super::state::AppState, column: usize) {
    if state.peer_sort_column == column {
        state.peer_sort_ascending = !state.peer_sort_ascending;
    } else {
        state.peer_sort_column = column;
        state.peer_sort_ascending = false;
    }
    resort_peers(state);
}

/// Dispatches a peer-list sort key (`1`-`4`, `n`/`m`/`b`/`l` for columns,
/// `o` to toggle direction) when the Peers tab is active.
fn handle_peer_sort_key(state: &mut super::state::AppState, c: char) {
    match c {
        '1' | 'n' | 'N' => apply_peer_sort(state, 0),
        '2' | 'm' | 'M' => apply_peer_sort(state, 1),
        '3' | 'b' | 'B' => apply_peer_sort(state, 2),
        '4' | 'l' | 'L' => apply_peer_sort(state, 3),
        'o' | 'O' => {
            state.peer_sort_ascending = !state.peer_sort_ascending;
            resort_peers(state);
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
        if let Some(peer_id) = state
            .peers
            .get(state.peer_selection)
            .map(|p| p.peer_id.clone())
        {
            load_dm_messages(state, &peer_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer: {peer_id}"));
        }
    } else if let p2p_app::tui_tabs::TabContent::PeerInfo(peer_id) = &tab_content {
        load_dm_messages(state, peer_id);
        let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id.clone());
        state.active_tab = tab_idx;
        p2plog_debug(format!("Opened DM with peer: {peer_id}"));
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
        handle_esc_key(state, render_tx).await;
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
    if dismiss_popup(&mut s) {
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
            handle_close_dm_tab(&mut s, &tab_content);
        }
        crossterm::event::KeyCode::Char('n')
            if !key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
                && matches!(
                    s.dynamic_tabs.tab_index_to_content(s.active_tab),
                    p2p_app::tui_tabs::TabContent::Settings
                )
                && !s.editing_nickname =>
        {
            s.editing_nickname = true;
            s.editing_nickname_peer = None;
            s.chat_input = super::TextArea::default();
            p2plog_debug("Started nickname edit from Settings tab".to_string());
        }
        crossterm::event::KeyCode::Char('i')
            if !key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
                && !s.editing_nickname =>
        {
            open_peer_info_for_active_tab(&mut s);
            p2plog_debug("Opened Peer Info tab".to_string());
        }
        crossterm::event::KeyCode::Char(c)
            if !key_event
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL)
                && !s.editing_nickname
                && matches!(
                    s.dynamic_tabs.tab_index_to_content(s.active_tab),
                    p2p_app::tui_tabs::TabContent::Peers
                ) =>
        {
            handle_peer_sort_key(&mut s, c);
        }
        _ => {
            let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
            if tab_content.is_input_enabled() || s.editing_nickname {
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
    if matches!(
        mouse_event.kind,
        crossterm::event::MouseEventKind::Moved | crossterm::event::MouseEventKind::Drag(_)
    ) {
        return;
    }

    let mut s = state.lock().await;
    let mut should_render = false;

    s.last_mouse_row = mouse_event.row;

    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    let is_peers_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers);
    let is_scrollable_tab = matches!(
        tab_content,
        p2p_app::tui_tabs::TabContent::Chat
            | p2p_app::tui_tabs::TabContent::Direct(_)
            | p2p_app::tui_tabs::TabContent::Log
    );
    let peer_id = if let p2p_app::tui_tabs::TabContent::Direct(pid) = &tab_content {
        Some(pid.as_str())
    } else {
        None
    };

    match mouse_event.kind {
        crossterm::event::MouseEventKind::ScrollUp if is_scrollable_tab => {
            should_render = handle_mouse_scroll(&mut s, "up", peer_id);
        }
        crossterm::event::MouseEventKind::ScrollDown if is_scrollable_tab => {
            should_render = handle_mouse_scroll(&mut s, "down", peer_id);
        }
        crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            if dismiss_popup(&mut s) {
                should_render = true;
            } else {
                should_render = handle_mouse_left_click(
                    &mut s,
                    mouse_event.row,
                    mouse_event.column,
                    is_peers_tab,
                );
            }
        }
        _ => {}
    }
    drop(s);
    if should_render {
        let _ = render_tx.send(RenderEvent).await;
    }
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

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_input_processor.rs"]
mod tests;
