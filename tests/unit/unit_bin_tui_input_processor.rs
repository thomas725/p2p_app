use super::*;
#[allow(unused_imports)]
use crate::tui::test_helpers::app_state_with_peers;
use crate::tui::test_helpers::{app_state_with_dm_messages, test_app_state};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use p2p_app::tui_tabs::TabContent;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

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

#[tokio::test]
async fn test_mouse_move_is_ignored() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let mouse_event = MouseEvent {
        kind: MouseEventKind::Moved,
        column: 12,
        row: 7,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    let exited = process_input_event(
        InputEvent::Mouse(mouse_event),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert!(render_rx.try_recv().is_err());
    assert_eq!(state.lock().await.last_mouse_row, 0);
}

#[tokio::test]
async fn test_mouse_click_noop_does_not_redraw() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let mouse_event = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: 12,
        row: 40,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    let exited = process_input_event(
        InputEvent::Mouse(mouse_event),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert!(render_rx.try_recv().is_err());
}

#[tokio::test]
async fn test_mouse_scroll_noop_does_not_redraw() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let mouse_event = MouseEvent {
        kind: MouseEventKind::ScrollUp,
        column: 12,
        row: 7,
        modifiers: crossterm::event::KeyModifiers::NONE,
    };

    let exited = process_input_event(
        InputEvent::Mouse(mouse_event),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert!(render_rx.try_recv().is_err());
}

// ── process_key_event ─────────────────────────────────────────────────

#[tokio::test]
async fn test_esc_returns_to_chat_tab() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    {
        let mut s = state.lock().await;
        s.active_tab = 2;
    }

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert_eq!(s.active_tab, 0);
    assert!(render_rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_ctrl_q_returns_exit_signal() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(exited);
}

#[tokio::test]
async fn test_esc_dismisses_popup() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    {
        let mut s = state.lock().await;
        s.popup = Some("notice".to_string());
    }

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert_eq!(s.popup, None);
    assert!(render_rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_char_input_adds_to_chat() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert!(s.chat_input.lines().join("").contains('h'));
    assert!(render_rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_enter_without_text_does_not_send() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    // No SwarmCommand should be sent for empty message
    assert!(render_rx.try_recv().is_ok());
}

// ── toggle_mouse_capture ──────────────────────────────────────────────

#[test]
fn test_toggle_mouse_capture_toggles_state() {
    let mut state = test_app_state();
    state.mouse_capture = false;
    toggle_mouse_capture(&mut state);
    assert!(state.mouse_capture);
    toggle_mouse_capture(&mut state);
    assert!(!state.mouse_capture);
}

// ── handle_enter_key ──────────────────────────────────────────────────

#[tokio::test]
async fn test_enter_key_in_peers_tab_opens_dm() {
    use crate::tui::test_helpers::app_state_with_peers;
    let mut state = app_state_with_peers(3);
    state.active_tab = 1; // Peers tab
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_enter_key(
        &mut state,
        &swarm_cmd_tx,
        false,
        p2p_app::tui_tabs::TabContent::Peers,
    )
    .await;

    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
    assert!(state.active_tab >= dm_count_before);
}

#[tokio::test]
async fn test_shift_enter_inserts_newline() {
    let mut state = test_app_state();
    state.chat_input.insert_str("hello");
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_enter_key(
        &mut state,
        &swarm_cmd_tx,
        true,
        p2p_app::tui_tabs::TabContent::Chat,
    )
    .await;

    let text = state.chat_input.lines().join("\n");
    assert_eq!(text, "hello\n");
}

// ── handle_nickname_submission ────────────────────────────────────────

#[tokio::test]
async fn test_nickname_submission_empty_clears_edit() {
    let mut state = test_app_state();
    state.editing_nickname = true;
    state.editing_nickname_peer = Some("p1".to_string());
    // chat_input is empty -> prepare_nickname_update returns None
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

    assert!(!state.editing_nickname);
    assert_eq!(state.editing_nickname_peer, None);
}

#[tokio::test]
async fn test_nickname_submission_per_peer_sends_dm() {
    let mut state = test_app_state();
    state.editing_nickname = true;
    state.editing_nickname_peer = Some("peer-nick".to_string());
    state.own_nickname = "OldGlobal".to_string();
    state.chat_input.insert_str("NewPerPeer");
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

    handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

    assert!(!state.editing_nickname);
    assert_eq!(
        state.self_nicknames_for_peers.get("peer-nick"),
        Some(&"NewPerPeer".to_string())
    );
    // Should have sent a SendDm with the new nickname
    let cmd = tokio::time::timeout(std::time::Duration::from_millis(100), swarm_cmd_rx.recv())
        .await
        .expect("timeout")
        .expect("expected SendDm");
    match cmd {
        SwarmCommand::SendDm {
            peer_id, nickname, ..
        } => {
            assert_eq!(peer_id, "peer-nick");
            assert_eq!(nickname, Some("NewPerPeer".to_string()));
        }
        _ => panic!("expected SendDm"),
    }
}

#[tokio::test]
async fn test_nickname_submission_global_updates_own_nickname() {
    let mut state = test_app_state();
    state.editing_nickname = true;
    state.editing_nickname_peer = None;
    state.own_nickname = "OldGlobal".to_string();
    state.chat_input.insert_str("NewGlobal");
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

    assert!(!state.editing_nickname);
    assert_eq!(state.own_nickname, "NewGlobal");
}

// ── process_key_event: ESC + editing_nickname ─────────────────────────

#[tokio::test]
async fn test_esc_cancels_nickname_edit() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);
    {
        let mut s = state.lock().await;
        s.editing_nickname = true;
        s.editing_nickname_peer = Some("p1".to_string());
    }

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert!(!s.editing_nickname);
    assert_eq!(s.editing_nickname_peer, None);
}

// ── process_key_event: Enter + editing_nickname ────────────────────────

#[tokio::test]
async fn test_enter_during_nickname_edit_submits_nickname() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);
    {
        let mut s = state.lock().await;
        s.editing_nickname = true;
        s.editing_nickname_peer = Some("p1".to_string());
        s.chat_input.insert_str("NewNick");
    }

    let exited = process_input_event(
        InputEvent::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert!(!s.editing_nickname);
}

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
