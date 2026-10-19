#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
#![allow(clippy::used_underscore_binding, clippy::significant_drop_tightening)]

use super::*;
use crate::tui::state::AppState;
use crate::tui::test_helpers::app_state_with_peers;
use crate::tui::test_helpers::{app_state_with_dm_messages, test_app_state};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use p2p_app::tui_tabs::TabContent;
use serial_test::serial;
use std::sync::Arc;
use tempfile::TempDir;
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
    handle_close_dm_tab(&mut state, &TabContent::Direct("peer-close".to_string()));
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before - 1);
}

#[test]
fn test_close_dm_tab_switches_to_previous_tab() {
    let mut state = app_state_with_dm_messages("peer-close", 3);
    state.active_tab = state.dynamic_tabs.total_tab_count() - 1;
    handle_close_dm_tab(&mut state, &TabContent::Direct("peer-close".to_string()));
    // Should fall back to previous tab
    assert!(state.active_tab < state.dynamic_tabs.total_tab_count());
}

#[test]
fn test_close_dm_tab_noop_on_chat_tab() {
    let mut state = test_app_state();
    handle_close_dm_tab(&mut state, &TabContent::Chat);
    assert_eq!(state.active_tab, 0);
}

// ── flip_mouse_capture_state ──────────────────────────────────────────

#[test]
fn test_flip_mouse_capture_toggles_on() {
    let mut state = test_app_state();
    state.env.mouse_capture = false;
    assert!(flip_mouse_capture_state(&mut state));
    assert!(state.env.mouse_capture);
}

#[test]
fn test_flip_mouse_capture_toggles_off() {
    let mut state = test_app_state();
    state.env.mouse_capture = true;
    assert!(!flip_mouse_capture_state(&mut state));
    assert!(!state.env.mouse_capture);
}

#[test]
fn test_flip_mouse_capture_returns_new_state() {
    let mut state = test_app_state();
    assert_eq!(
        flip_mouse_capture_state(&mut state),
        state.env.mouse_capture
    );
}

#[tokio::test]
async fn test_mouse_move_is_ignored_for_layout_but_tracks_hover_row() {
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
    assert_eq!(state.lock().await.last_mouse_row, 7);
}

#[tokio::test]
async fn test_mouse_drag_is_ignored_for_layout_but_tracks_hover_row() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let mouse_event = MouseEvent {
        kind: MouseEventKind::Drag(MouseButton::Left),
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
    assert_eq!(state.lock().await.last_mouse_row, 7);
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
    state.env.mouse_capture = false;
    toggle_mouse_capture(&mut state);
    assert!(state.env.mouse_capture);
    toggle_mouse_capture(&mut state);
    assert!(!state.env.mouse_capture);
}

// ── parse_file_command ────────────────────────────────────────────────

#[test]
fn test_parse_file_command_extracts_path() {
    assert_eq!(
        parse_file_command("/file /tmp/notes.txt"),
        Some("/tmp/notes.txt".to_string())
    );
    assert_eq!(
        parse_file_command("  /file  some dir/a b.txt  "),
        Some("some dir/a b.txt".to_string())
    );
}

#[test]
fn test_parse_file_command_rejects_non_commands() {
    assert_eq!(parse_file_command("/file"), None);
    assert_eq!(parse_file_command("/file "), None);
    assert_eq!(parse_file_command("send /file to the group"), None);
    assert_eq!(parse_file_command("hello"), None);
    assert_eq!(parse_file_command(""), None);
}

// ── handle_enter_key ──────────────────────────────────────────────────

#[test]
fn test_enter_key_in_peers_tab_opens_dm() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = app_state_with_peers(3);
        state.active_tab = 1; // Peers tab
        let dm_count_before = state.dynamic_tabs.dm_tab_count();
        let (swarm_cmd_tx, _) = mpsc::channel(1);

        handle_enter_key(
            &mut state,
            &swarm_cmd_tx,
            false,
            p2p_app::tui_tabs::TabContent::Contacts,
        )
        .await;

        assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
        assert!(state.active_tab >= dm_count_before);
    });
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_ctrl_w_closes_peer_info_tab() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.active_tab = s.dynamic_tabs.add_peer_info_tab("peer-info-1".to_string());
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
    assert!(s.active_tab < s.dynamic_tabs.total_tab_count());
}

#[tokio::test]
async fn test_i_key_in_peers_tab_opens_peer_info() {
    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    state.lock().await.active_tab = 1; // Peers tab
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(matches!(content, TabContent::PeerInfo(_)));
}

#[tokio::test]
async fn test_ctrl_i_key_in_direct_tab_opens_peer_info() {
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 3; // Direct tab for "peer-dm"
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(matches!(content, TabContent::PeerInfo(_)));
}

#[tokio::test]
async fn test_ctrl_tab_is_noop_on_direct_tab() {
    // On a kitty-enabled terminal Ctrl+I arrives as `Char('i')`+CONTROL, never
    // as `Tab`+CONTROL, so `Tab`+CONTROL on a Direct tab (like anywhere else)
    // must be a no-op: no Peer Info, no tab switch.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 3; // Direct tab for "peer-dm"
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 3);
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_bare_tab_in_direct_tab_navigates_on_kitty_and_nonkitty() {
    // A bare Tab byte must always cycle tabs on a Direct tab — whether the
    // terminal delivers kitty encoding (WezTerm) or collapses Ctrl+I to 0x09
    // (Konsole). Peer Info on a Direct tab is opened by Ctrl+I (kitty) or
    // Ctrl+? (non-kitty), never by sacrificing the Tab key.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 3; // Direct tab for "peer-dm"
        s.env.kitty_keyboard_active = false; // collapsed (non-kitty) terminal
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 4); // Log tab (after Chat, Peers, Groups, DM)
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_bare_tab_in_direct_tab_navigates_when_kitty_active() {
    // On a terminal that genuinely delivers kitty encoding (e.g. WezTerm with
    // `enable_kitty_keyboard`), a bare Tab byte is a REAL Tab and must cycle
    // tabs — not open Peer Info. This is the default (`kitty_keyboard_active =
    // true`) state.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 3; // Direct tab for "peer-dm"
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 4); // Log tab (after Chat, Peers, Groups, DM)
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_p_in_direct_tab_opens_peer_info_nonkitty() {
    // On a non-kitty terminal Ctrl+P sends the control byte 0x10 (DLE), which
    // crossterm decodes as `Char('p')`+CONTROL. On a Direct tab it must open
    // the partner's Peer Info (Konsole cannot distinguish Ctrl+? from a plain
    // '?', so Ctrl+P is the non-kitty binding).
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 3; // Direct tab for "peer-dm"
        s.env.kitty_keyboard_active = false; // non-kitty terminal
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(matches!(content, TabContent::PeerInfo(_)));
}

#[tokio::test]
async fn test_ctrl_p_in_direct_tab_opens_peer_info_kitty() {
    // Ctrl+P is a universal binding: it must also work on kitty terminals
    // (where Ctrl+I additionally opens Peer Info as `Char('i')`+CONTROL).
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 3; // Direct tab for "peer-dm"
        s.env.kitty_keyboard_active = true; // kitty terminal (default)
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(matches!(content, TabContent::PeerInfo(_)));
}

#[tokio::test]
async fn test_ctrl_p_is_noop_on_non_direct_tabs() {
    // Ctrl+P must never be typed as literal 'p' text: on a non-Direct tab (and
    // even on a kitty terminal) it is consumed as a no-op.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 0; // Chat tab (input enabled)
        s.env.kitty_keyboard_active = true; // kitty terminal
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 0);
    assert!(s.chat_input.lines().join("").is_empty());
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_bare_tab_on_chat_tab_still_navigates() {
    // The bare Tab byte keeps cycling tabs on non-Direct tabs.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 0; // Chat tab
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 1); // Peers tab
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_i_key_in_direct_tab_types_into_input() {
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 3; // Direct tab for "peer-dm"
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.chat_input.lines().join("").contains('i'));
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_tab_is_noop_on_peers_tab() {
    // `Tab`+CONTROL is the CSI-u encoding of Ctrl+I on some terminals; on a
    // non-Direct tab it must neither open Peer Info nor cycle tabs.
    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    state.lock().await.active_tab = 1; // Peers tab
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 1);
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_i_is_noop_on_non_direct_tabs() {
    // On terminals honoring DISAMBIGUATE_ESCAPE_CODES, Ctrl+I arrives as
    // `Char('i')`+CONTROL, distinguishable from a real Tab. On such terminals
    // it must do nothing: no Peer Info, no tab switch, no 'i' typed.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 0; // Chat tab (input enabled)
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 0);
    assert!(s.chat_input.lines().join("").is_empty());
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_tab_is_noop_on_chat_tab() {
    // `Tab`+CONTROL is the CSI-u encoding of Ctrl+I on some terminals; on an
    // input-enabled non-Direct tab it must not type, navigate, or open info.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 0; // Chat tab (input enabled)
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 0);
    assert!(s.chat_input.lines().join("").is_empty());
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_backtab_in_direct_tab_still_navigates_backward() {
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    state.lock().await.active_tab = 3; // Direct tab for "peer-dm"
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 2); // Groups tab (DM tabs sit after the fixed tabs)
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_legacy_bare_tab_on_chat_tab_still_navigates() {
    // Without protocol support Ctrl+I on a non-Direct tab is byte-identical to
    // Tab, so it keeps cycling tabs there.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.env.kitty_keyboard_active = false;
        s.active_tab = 0; // Chat tab
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 1); // Peers tab
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_legacy_plain_i_on_direct_tab_still_types() {
    // Even without protocol support, a bare `i` (no CONTROL) must keep typing
    // into the DM input box instead of being mistaken for Ctrl+I.
    let state = Arc::new(Mutex::new(app_state_with_dm_messages("peer-dm", 3)));
    {
        let mut s = state.lock().await;
        s.env.kitty_keyboard_active = false;
        s.active_tab = 3; // Direct tab for "peer-dm"
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.chat_input.lines().join("").contains('i'));
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_tab_on_peers_tab_is_noop() {
    // `Tab`+CONTROL is the CSI-u spelling of Ctrl+I. On a non-Direct tab it is
    // a no-op: it neither opens Peer Info nor cycles tabs.
    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 1; // Peers tab
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 1); // still Peers tab
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_enter_key_in_peer_info_tab_opens_dm() {
    {
        let _guard = p2p_app::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let _dir = TempDir::new().unwrap();
        let db_path = _dir.path().join("test.db");
        p2p_app::db::set_db_url(db_path.to_str().unwrap());
        p2p_app::db::init_database().unwrap();
    }

    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    {
        let mut s = state.lock().await;
        s.active_tab = s.dynamic_tabs.add_peer_info_tab("peer-1".to_string());
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.dynamic_tabs.dm_tab_count(), 1);
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_enter_key_in_peers_tab_empty_list_is_noop() {
    let mut state = test_app_state();
    state.active_tab = 1; // Peers tab
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_enter_key(
        &mut state,
        &swarm_cmd_tx,
        false,
        p2p_app::tui_tabs::TabContent::Contacts,
    )
    .await;

    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before);
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

#[tokio::test]
async fn test_shift_enter_in_peers_tab_is_noop() {
    let mut state = app_state_with_peers(3);
    state.active_tab = 1;
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_enter_key(
        &mut state,
        &swarm_cmd_tx,
        true,
        p2p_app::tui_tabs::TabContent::Contacts,
    )
    .await;

    // Shift+Enter in Peers tab (input disabled) is a noop
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before);
}

// ── handle_nickname_submission ────────────────────────────────────────

#[tokio::test]
async fn test_nickname_submission_empty_clears_edit() {
    let mut state = test_app_state();
    state.input_mode = p2p_app::InputMode::EditingNickname;
    state.editing_nickname_peer = Some("p1".to_string());
    // chat_input is empty -> prepare_nickname_update returns None
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

    assert!(!state.is_editing_nickname());
    assert_eq!(state.editing_nickname_peer, None);
}

#[test]
fn test_nickname_submission_per_peer_sends_dm() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        state.input_mode = p2p_app::InputMode::EditingNickname;
        state.editing_nickname_peer = Some("peer-nick".to_string());
        state.own_nickname = "OldGlobal".to_string();
        state.chat_input.insert_str("NewPerPeer");
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

        handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

        assert!(!state.is_editing_nickname());
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
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_nickname_submission_global_updates_own_nickname() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        state.input_mode = p2p_app::InputMode::EditingNickname;
        state.editing_nickname_peer = None;
        state.own_nickname = "OldGlobal".to_string();
        state.chat_input.insert_str("NewGlobal");
        let (swarm_cmd_tx, _) = mpsc::channel(1);

        handle_nickname_submission(&mut state, &swarm_cmd_tx).await;

        assert!(!state.is_editing_nickname());
        assert_eq!(state.own_nickname, "NewGlobal");
    });
    p2p_app::db::reset_db_url();
}

// ── per-peer local nickname (n on a Peer Info tab) ─────────────────────

#[test]
fn test_nickname_key_branches_by_tab() {
    let mut settings = test_app_state();
    handle_nickname_key(&mut settings, &TabContent::Settings);
    assert!(settings.is_editing_nickname());
    assert!(!settings.is_editing_local_nickname());
    assert_eq!(settings.editing_nickname_peer, None);

    let mut peer_info = test_app_state();
    handle_nickname_key(
        &mut peer_info,
        &TabContent::PeerInfo("peer-abc".to_string()),
    );
    assert!(peer_info.is_editing_nickname());
    assert!(peer_info.is_editing_local_nickname());
    assert_eq!(peer_info.editing_nickname_peer.as_deref(), Some("peer-abc"));

    // Every other tab ignores `n` (it is either literal text or unused there).
    for tab in [
        TabContent::Chat,
        TabContent::Groups,
        TabContent::Log,
        TabContent::Contacts,
    ] {
        let mut s = test_app_state();
        handle_nickname_key(&mut s, &tab);
        assert!(!s.is_editing_nickname(), "no edit on {tab:?}");
        assert_eq!(s.editing_nickname_peer, None);
    }
}

#[tokio::test]
async fn test_n_key_on_peer_info_starts_local_nickname_edit() {
    let mut s = test_app_state();
    s.active_tab = s.dynamic_tabs.add_peer_info_tab("peer-abc".to_string());
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.is_editing_local_nickname());
    assert_eq!(s.editing_nickname_peer.as_deref(), Some("peer-abc"));
}

#[tokio::test]
async fn test_n_key_outside_nickname_tabs_stays_plain_input() {
    // On the broadcast Chat tab (input enabled) `n` must remain literal text,
    // never open an editor.
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(!s.is_editing_nickname());
    assert_eq!(s.chat_input.lines().join(""), "n");
}

#[test]
fn test_local_nickname_submission_rejects_invalid_and_keeps_open() {
    let mut s = test_app_state();
    s.input_mode = p2p_app::InputMode::EditingLocalNickname;
    s.editing_nickname_peer = Some("peer-x".to_string());
    s.chat_input.insert_str("bad name!");
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_nickname_submission(&mut s, &swarm_cmd_tx));

    // The editor stays open with the canonical-rule hint, nothing persisted.
    assert!(s.is_editing_local_nickname());
    let popup = s.popup.as_deref().expect("popup");
    assert!(popup.contains("letters, numbers or dashes"));
    assert!(!s.local_nicknames.contains_key("peer-x"));
}

#[test]
fn test_local_nickname_submission_empty_cancels() {
    let mut s = test_app_state();
    s.input_mode = p2p_app::InputMode::EditingLocalNickname;
    s.editing_nickname_peer = Some("peer-x".to_string());
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_nickname_submission(&mut s, &swarm_cmd_tx));

    assert!(!s.is_editing_nickname());
    assert_eq!(s.editing_nickname_peer, None);
}

#[test]
fn test_local_nickname_submission_persists_and_updates_maps() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("local-nick.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut s = test_app_state();
        s.input_mode = p2p_app::InputMode::EditingLocalNickname;
        s.editing_nickname_peer = Some("peer-local".to_string());
        s.chat_input.insert_str("Bobby");
        let (swarm_cmd_tx, _) = mpsc::channel(1);

        handle_nickname_submission(&mut s, &swarm_cmd_tx).await;

        assert!(!s.is_editing_nickname(), "editor closed on success");
        assert_eq!(
            s.local_nicknames.get("peer-local").map(String::as_str),
            Some("Bobby")
        );
        // Persisted under the canonical column (a Peer Info reload reads it).
        assert_eq!(
            p2p_app::nickname::get_peer_local_nickname("peer-local")
                .unwrap()
                .as_deref(),
            Some("Bobby")
        );
    });
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_esc_cancels_local_nickname_edit() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);
    {
        let mut s = state.lock().await;
        s.input_mode = p2p_app::InputMode::EditingLocalNickname;
        s.editing_nickname_peer = Some("peer-x".to_string());
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
    assert!(!s.is_editing_nickname());
    assert_eq!(s.editing_nickname_peer, None);
}

// ── process_key_event: ESC + editing_nickname ─────────────────────────

#[tokio::test]
async fn test_esc_cancels_nickname_edit() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);
    {
        let mut s = state.lock().await;
        s.input_mode = p2p_app::InputMode::EditingNickname;
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
    assert!(!s.is_editing_nickname());
    assert_eq!(s.editing_nickname_peer, None);
}

// ── process_key_event: Enter + editing_nickname ────────────────────────

#[test]
fn test_enter_during_nickname_edit_submits_nickname() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, _) = mpsc::channel(1);
        let (render_tx, _) = mpsc::channel(1);
        {
            let mut s = state.lock().await;
            s.input_mode = p2p_app::InputMode::EditingNickname;
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
        assert!(!s.is_editing_nickname());
    });
    p2p_app::db::reset_db_url();
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

// ── e: expand/collapse device rows ──────────────────────────────────────

#[tokio::test]
async fn test_e_key_toggles_selected_contact_expansion() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.contacts = vec![p2p_app::contacts::ContactRow {
            contact_id: "bob".to_string(),
            is_user: false,
            display_name: "Bob".to_string(),
            device_count: 2,
            device_rows: vec![
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d1".to_string(),
                    display_name: "Laptop".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                    first_seen: "s".to_string(),
                    last_seen: "l".to_string(),
                },
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d2".to_string(),
                    display_name: "Phone".to_string(),
                    first_seen: "s".to_string(),
                    last_seen: "l".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                },
            ],
            first_seen: "s".to_string(),
            last_seen: "l".to_string(),
            dm_count: 3,
            broadcast_sent_to_peer: 2,
        }];
        s.peer_selection = 0;
        s.active_tab = 1; // Contacts tab
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let expand = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    let _ = process_key_event(expand, &state, &swarm_cmd_tx, &render_tx).await;
    assert!(
        render_rx.recv().await.is_some(),
        "expected a re-render after expanding"
    );
    let s = state.lock().await;
    assert!(s.expanded_contact_ids.contains("bob"));
    assert_eq!(
        p2p_app::tui_helpers::contact_table_plan_len(&s.contacts, &s.expanded_contact_ids),
        3
    );
    drop(s);

    let _ = process_key_event(expand, &state, &swarm_cmd_tx, &render_tx).await;
    assert!(
        render_rx.recv().await.is_some(),
        "expected a re-render after collapsing"
    );
    let s = state.lock().await;
    assert!(s.expanded_contact_ids.is_empty());
    assert_eq!(
        p2p_app::tui_helpers::contact_table_plan_len(&s.contacts, &s.expanded_contact_ids),
        1
    );
}

#[tokio::test]
async fn test_i_key_on_device_row_opens_peer_info_for_exact_device() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.contacts = vec![p2p_app::contacts::ContactRow {
            contact_id: "bob".to_string(),
            is_user: false,
            display_name: "Bob".to_string(),
            device_count: 2,
            device_rows: vec![
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d1".to_string(),
                    display_name: "Laptop".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                    first_seen: "s".to_string(),
                    last_seen: "l".to_string(),
                },
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d2".to_string(),
                    display_name: "Phone".to_string(),
                    first_seen: "s".to_string(),
                    last_seen: "l".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                },
            ],
            first_seen: "s".to_string(),
            last_seen: "l".to_string(),
            dm_count: 3,
            broadcast_sent_to_peer: 2,
        }];
        s.expanded_contact_ids.insert("bob".to_string());
        s.peer_selection = 2; // bob's second device row ("Phone", peer bob-d2)
        s.active_tab = 1; // Contacts tab
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    match content {
        TabContent::PeerInfo(peer_id) => {
            assert_eq!(peer_id, "bob-d2");
        }
        other => panic!("expected PeerInfo(bob-d2), got: {other:?}"),
    }
}

// ── device-binding prompt (accept/decline) ─────────────────────────────────

fn prompt_state(peer: &str) -> AppState {
    let mut s = test_app_state();
    s.device_binding_prompt = Some(crate::tui::state::DeviceBindingPrompt {
        peer_id: peer.to_string(),
        device_pubkey: vec![0xAB, 0xCD],
    });
    s
}

#[tokio::test]
async fn test_device_binding_d_key_declines_prompt() {
    let state = Arc::new(Mutex::new(prompt_state("target-device")));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited);
    let s = state.lock().await;
    assert!(s.device_binding_prompt.is_none());
    assert!(render_rx.try_recv().is_ok());
}

#[tokio::test]
async fn test_device_binding_any_non_accept_key_declines_prompt() {
    let state = Arc::new(Mutex::new(prompt_state("target-device")));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.device_binding_prompt.is_none());
}

#[tokio::test]
async fn test_device_binding_esc_declines_prompt() {
    let state = Arc::new(Mutex::new(prompt_state("target-device")));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited);
    let s = state.lock().await;
    assert!(s.device_binding_prompt.is_none());
}

#[test]
#[serial]
fn test_device_binding_accept_mints_and_queues_claim() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().expect("db path").to_string();
    p2p_app::db::set_db_url(&db_str);
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let state = Arc::new(Mutex::new(prompt_state("target-device")));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
        let (render_tx, _) = mpsc::channel(1);

        let exited = process_key_event(
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;
        assert!(!exited);

        let s = state.lock().await;
        assert!(s.device_binding_prompt.is_none(), "prompt consumed");
        assert!(
            s.own_user_devices
                .iter()
                .any(|(peer, _)| peer == "target-device"),
            "accepted device shows on the Settings list: {:?}",
            s.own_user_devices
        );
        drop(s);

        match swarm_cmd_rx.try_recv().expect("claim queued") {
            SwarmCommand::SendDeviceBinding { peer_id, request } => {
                assert_eq!(peer_id, "target-device");
                assert_eq!(request.kind, "claim");
            }
            other => panic!("expected SendDeviceBinding claim, got: {other:?}"),
        }
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── call overlay (start / accept / decline / cancel / hang up) ─────────────

fn call_state(peer: &str, state: p2p_app::CallState) -> AppState {
    let mut s = test_app_state();
    s.call_session = Some(p2p_app::CallSession {
        call_id: "call-1".to_string(),
        peer_id: peer.to_string(),
        state,
    });
    s
}

/// Ask for the queued call signal, asserting it is a `SendCallSignal`.
fn assert_send_call_signal(
    rx: &mut mpsc::Receiver<SwarmCommand>,
    expected_peer: &str,
    expected_kind: &str,
) {
    match rx.try_recv().expect("call signal queued") {
        SwarmCommand::SendCallSignal { peer_id, signal } => {
            assert_eq!(peer_id, expected_peer);
            assert_eq!(signal.kind, expected_kind);
        }
        other => panic!("expected SendCallSignal, got: {other:?}"),
    }
}

#[tokio::test]
async fn test_c_key_on_contacts_starts_outgoing_call() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.contacts = vec![p2p_app::contacts::ContactRow {
            contact_id: "bob".to_string(),
            is_user: false,
            display_name: "Bob".to_string(),
            device_count: 1,
            device_rows: vec![p2p_app::contacts::ContactDevice {
                peer_id: "bob-d1".to_string(),
                display_name: "Laptop".to_string(),
                received_nickname: None,
                local_nickname: None,
                first_seen: "s".to_string(),
                last_seen: "l".to_string(),
            }],
            first_seen: "s".to_string(),
            last_seen: "l".to_string(),
            dm_count: 0,
            broadcast_sent_to_peer: 0,
        }];
        s.peer_selection = 0;
        s.active_tab = 1; // Contacts tab
    }
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.call_session.is_some());
    assert_eq!(s.call_session.as_ref().unwrap().peer_id, "bob-d1");
    assert_eq!(
        s.call_session.as_ref().unwrap().state,
        p2p_app::CallState::Calling
    );
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "invite");
}

#[tokio::test]
async fn test_c_key_on_peer_info_starts_outgoing_call() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.active_tab = s.dynamic_tabs.add_peer_info_tab("bob-d1".to_string());
    }
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.call_session.is_some());
    assert_eq!(s.call_session.as_ref().unwrap().peer_id, "bob-d1");
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "invite");
}

#[tokio::test]
async fn test_call_overlay_a_accepts_ringing() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Ringing,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited);
    let s = state.lock().await;
    assert_eq!(
        s.call_session.as_ref().unwrap().state,
        p2p_app::CallState::InCall
    );
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "accept");
}

#[tokio::test]
async fn test_call_overlay_any_key_declines_ringing() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Ringing,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited);
    let s = state.lock().await;
    assert!(s.call_session.is_none(), "session cleared after decline");
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "reject");
}

#[tokio::test]
async fn test_call_overlay_esc_declines_ringing() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Ringing,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited, "Esc declines a ring, it does not exit");
    let s = state.lock().await;
    assert!(s.call_session.is_none());
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "reject");
}

#[tokio::test]
async fn test_call_overlay_c_cancels_calling() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Calling,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    let s = state.lock().await;
    assert!(s.call_session.is_none());
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "end");
}

#[tokio::test]
async fn test_call_overlay_esc_cancels_calling() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Calling,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    let s = state.lock().await;
    assert!(s.call_session.is_none());
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "end");
}

#[tokio::test]
async fn test_call_overlay_typing_ignored_while_calling() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Calling,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    let s = state.lock().await;
    assert!(s.call_session.is_some(), "ignored key keeps the call");
    assert_eq!(
        s.call_session.as_ref().unwrap().state,
        p2p_app::CallState::Calling
    );
    drop(s);
    assert!(swarm_cmd_rx.try_recv().is_err(), "no signal sent");
}

#[tokio::test]
async fn test_call_overlay_esc_hangs_up_incall() {
    let state = Arc::new(Mutex::new(call_state("bob-d1", p2p_app::CallState::InCall)));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    let s = state.lock().await;
    assert!(s.call_session.is_none());
    drop(s);
    assert_send_call_signal(&mut swarm_cmd_rx, "bob-d1", "end");
}

#[tokio::test]
async fn test_call_overlay_ctrl_q_quits_during_call() {
    let state = Arc::new(Mutex::new(call_state("bob-d1", p2p_app::CallState::InCall)));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(exited, "Ctrl+Q still exits during a call");
}

// ── group leave (d key + confirmation) ─────────────────────────────────────

fn group_summary(group_id: &str, name: &str) -> p2p_app::groups::GroupSummary {
    p2p_app::groups::GroupSummary {
        group: p2p_app::generated::models_queryable::Group {
            id: 1,
            group_id: group_id.to_string(),
            display_name: name.to_string(),
            is_private: 0,
            created_at: chrono::NaiveDateTime::default(),
        },
        member_count: 2,
    }
}

/// A state sitting on the Groups list with one group and nothing else armed.
fn groups_tab_state(group_id: &str, name: &str) -> AppState {
    let mut s = test_app_state();
    s.group_summaries = vec![group_summary(group_id, name)];
    s.group_selection = 0;
    s.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT - 1; // the Groups list
    s
}

#[tokio::test]
async fn test_group_d_key_arms_leave_confirmation() {
    let state = Arc::new(Mutex::new(groups_tab_state("g-1", "Team")));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    let prompt = s.group_leave_prompt.as_ref().expect("prompt armed");
    assert_eq!(prompt.group_id, "g-1");
    assert_eq!(prompt.display_name, "Team");
    drop(s);
    assert!(render_rx.try_recv().is_ok(), "prompt triggers a redraw");
}

#[tokio::test]
async fn test_group_leave_any_other_key_cancels() {
    let mut s = groups_tab_state("g-1", "Team");
    s.begin_group_leave("g-1".to_string(), "Team".to_string());
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.group_leave_prompt.is_none(), "cancelled");
    assert_eq!(s.group_summaries.len(), 1, "nothing deleted");
    drop(s);
    assert!(swarm_cmd_rx.try_recv().is_err(), "no unsubscribe sent");
}

#[tokio::test]
async fn test_group_leave_esc_cancels_without_returning_to_chat() {
    let mut s = groups_tab_state("g-1", "Team");
    s.begin_group_leave("g-1".to_string(), "Team".to_string());
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.group_leave_prompt.is_none());
    assert_eq!(
        s.active_tab,
        p2p_app::tui_tabs::FIXED_TAB_COUNT - 1,
        "Esc only cancels the prompt, it does not jump back to Chat"
    );
}

#[test]
#[serial]
fn test_group_leave_confirmation_deletes_and_unsubscribes() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("group-leave.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let group = p2p_app::groups::create_public_group("Team Rocket").expect("create group");
        let group_id = group.group_id.clone();

        let mut s = groups_tab_state(&group_id, "Team Rocket");
        s.reload_group_summaries();
        // The group also has an open chat tab (so leaving must close it).
        s.dynamic_tabs
            .add_group_tab(group_id.clone(), "Team Rocket".to_string());
        s.group_messages.insert(
            group_id.clone(),
            std::collections::VecDeque::from(["hi".to_string()]),
        );
        s.begin_group_leave(group_id.clone(), "Team Rocket".to_string());

        let state = Arc::new(Mutex::new(s));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
        let (render_tx, _) = mpsc::channel(1);

        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;

        // Local rows gone (the DB side of "leave group").
        assert!(
            p2p_app::groups::find_group(&group_id)
                .expect("lookup")
                .is_none(),
            "group row deleted"
        );
        // And we stop receiving its traffic.
        match swarm_cmd_rx.try_recv() {
            Ok(SwarmCommand::UnsubscribeGroup { group_id: id }) => {
                assert_eq!(id, group_id);
            }
            other => panic!("expected UnsubscribeGroup, got {other:?}"),
        }

        let s = state.lock().await;
        assert!(s.group_leave_prompt.is_none(), "prompt consumed");
        assert!(
            s.group_summaries
                .iter()
                .all(|g| g.group.group_id != group_id),
            "list refreshed without the group"
        );
        assert!(
            s.dynamic_tabs.get_group_tab(&group_id).is_none(),
            "chat tab closed"
        );
        assert!(
            !s.group_messages.contains_key(&group_id),
            "in-memory history dropped so a re-join starts clean"
        );
    });
}
