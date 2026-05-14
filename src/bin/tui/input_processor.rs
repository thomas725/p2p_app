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
        p2p_app::tui_helpers::relabel_dm_transcript(dm_msgs, old_nick, new_nick);
    }
}

/// Pure: flip the mouse capture flag. Returns the new state.
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
    let (old_nickname, peer_id) = if let Some(pid) = &state.editing_nickname_peer {
        let old = state
            .self_nicknames_for_peers
            .get(pid)
            .cloned()
            .unwrap_or_else(|| state.own_nickname.clone());
        (old, Some(pid.clone()))
    } else {
        (state.own_nickname.clone(), None)
    };
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
        state.own_nickname = new_nickname.clone();
        let _ = p2p_app::set_self_nickname(&new_nickname);
        for (peer_id, _, _) in &state.peers {
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
        p2plog_debug(format!("Closed DM tab with peer: {peer_id}"));
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
            p2plog_debug(format!("Opened DM with peer: {peer_id}"));
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
        dismiss_popup(&mut s);
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
            if dismiss_popup(&mut s) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::test_helpers::{app_state_with_dm_messages, test_app_state};
    use p2p_app::tui_tabs::TabContent;

    // ── update_dm_transcript_labels ──────────────────────────────────────────

    #[test]
    fn test_update_dm_transcript_labels_changes_nick() {
        let mut dm_messages: std::collections::HashMap<String, std::collections::VecDeque<String>> =
            std::collections::HashMap::new();
        dm_messages.insert(
            "peer1".to_string(),
            std::collections::VecDeque::from(vec![
                "[OldNick] hello".to_string(),
                "[OldNick] how are you".to_string(),
                "[Other] hi".to_string(),
            ]),
        );
        update_dm_transcript_labels(&mut dm_messages, "peer1", "OldNick", "NewNick");
        let msgs = dm_messages.get("peer1").unwrap();
        assert_eq!(msgs[0], "[NewNick] hello");
        assert_eq!(msgs[1], "[NewNick] how are you");
        assert_eq!(msgs[2], "[Other] hi");
    }

    #[test]
    fn test_update_dm_transcript_labels_no_match_does_nothing() {
        let mut dm_messages: std::collections::HashMap<String, std::collections::VecDeque<String>> =
            std::collections::HashMap::new();
        dm_messages.insert(
            "peer1".to_string(),
            std::collections::VecDeque::from(vec!["[Other] hello".to_string()]),
        );
        update_dm_transcript_labels(&mut dm_messages, "peer1", "OldNick", "NewNick");
        assert_eq!(dm_messages.get("peer1").unwrap()[0], "[Other] hello");
    }

    #[test]
    fn test_update_dm_transcript_labels_unknown_peer() {
        let mut dm_messages = std::collections::HashMap::new();
        update_dm_transcript_labels(&mut dm_messages, "unknown", "Old", "New");
        // no panic, no crash
    }

    // ── handle_close_dm_tab ──────────────────────────────────────────────────

    #[test]
    fn test_close_dm_tab_removes_tab() {
        let mut state = app_state_with_dm_messages("peer-close", 3);
        let dm_count_before = state.dynamic_tabs.dm_tab_count();
        handle_close_dm_tab(&mut state, TabContent::Direct("peer-close".to_string()));
        assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before - 1);
    }

    #[test]
    fn test_close_dm_tab_switches_to_previous_tab() {
        let mut state = app_state_with_dm_messages("peer-close", 3);
        state.active_tab = state.dynamic_tabs.total_tab_count() - 1;
        handle_close_dm_tab(&mut state, TabContent::Direct("peer-close".to_string()));
        // Should fall back to previous tab
        assert!(state.active_tab < state.dynamic_tabs.total_tab_count());
    }

    #[test]
    fn test_close_dm_tab_noop_on_chat_tab() {
        let mut state = test_app_state();
        handle_close_dm_tab(&mut state, TabContent::Chat);
        assert_eq!(state.active_tab, 0);
    }

    // ── flip_mouse_capture_state ──────────────────────────────────────────

    #[test]
    fn test_flip_mouse_capture_toggles_on() {
        let mut state = test_app_state();
        state.mouse_capture = false;
        assert!(flip_mouse_capture_state(&mut state));
        assert!(state.mouse_capture);
    }

    #[test]
    fn test_flip_mouse_capture_toggles_off() {
        let mut state = test_app_state();
        state.mouse_capture = true;
        assert!(!flip_mouse_capture_state(&mut state));
        assert!(!state.mouse_capture);
    }

    #[test]
    fn test_flip_mouse_capture_returns_new_state() {
        let mut state = test_app_state();
        assert_eq!(flip_mouse_capture_state(&mut state), state.mouse_capture);
    }

    // ── dismiss_popup ─────────────────────────────────────────────────────

    #[test]
    fn test_dismiss_popup_clears_when_set() {
        let mut state = test_app_state();
        state.popup = Some("test".to_string());
        assert!(dismiss_popup(&mut state));
        assert_eq!(state.popup, None);
    }

    #[test]
    fn test_dismiss_popup_noop_when_none() {
        let mut state = test_app_state();
        state.popup = None;
        assert!(!dismiss_popup(&mut state));
        assert_eq!(state.popup, None);
    }

    // ── prepare_nickname_update ───────────────────────────────────────────

    #[test]
    fn test_prepare_nickname_update_returns_none_when_empty() {
        let state = test_app_state();
        let result = prepare_nickname_update(&state);
        assert!(result.is_none());
    }

    #[test]
    fn test_prepare_nickname_update_global_nick() {
        let mut state = test_app_state();
        state.own_nickname = "Global".to_string();
        state.editing_nickname_peer = None;
        state.chat_input.insert_str("NewNick");
        let result = prepare_nickname_update(&state);
        assert_eq!(
            result,
            Some(("NewNick".to_string(), "Global".to_string(), None))
        );
    }

    #[test]
    fn test_prepare_nickname_update_per_peer() {
        let mut state = test_app_state();
        state.own_nickname = "Global".to_string();
        state
            .self_nicknames_for_peers
            .insert("peer-1".to_string(), "PerPeer".to_string());
        state.editing_nickname_peer = Some("peer-1".to_string());
        state.chat_input.insert_str("NewNick");
        let result = prepare_nickname_update(&state);
        assert_eq!(
            result,
            Some((
                "NewNick".to_string(),
                "PerPeer".to_string(),
                Some("peer-1".to_string())
            ))
        );
    }

    #[test]
    fn test_prepare_nickname_update_falls_back_to_own_nickname() {
        let mut state = test_app_state();
        state.own_nickname = "Global".to_string();
        state.editing_nickname_peer = Some("peer-1".to_string());
        state.chat_input.insert_str("NewNick");
        let result = prepare_nickname_update(&state);
        assert_eq!(
            result.as_ref().map(|(_, old, _)| old.as_str()),
            Some("Global")
        );
    }
}
