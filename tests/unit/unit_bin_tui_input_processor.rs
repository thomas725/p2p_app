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
        state.active_tab = 1; // Contacts tab
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

// ── Peer blocklist toggle (US-054) ───────────────────────────────────────

#[test]
fn test_blocklist_toggle_from_peer_info_tab() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let peer_id = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
        .to_string();
    let mut state = app_state_with_peers(0);
    let tab_idx = state.dynamic_tabs.add_peer_info_tab(peer_id.clone());
    state.active_tab = tab_idx;
    let tab = p2p_app::tui_tabs::TabContent::PeerInfo(peer_id.clone());
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

    // First press mutes the peer: it is persisted, the UI set is refreshed, and
    // the live connection is closed (the boundary only refuses new ones).
    toggle_peer_block(&tab, &mut state, &swarm_cmd_tx);
    assert!(matches!(
        swarm_cmd_rx.try_recv(),
        Ok(SwarmCommand::DisconnectPeer { .. })
    ));
    assert!(state.blocked_peers.contains(&peer_id));
    assert!(p2p_app::blocklist::is_blocked(&peer_id));
    assert!(
        state
            .popup
            .as_deref()
            .unwrap_or_default()
            .contains("Blocked")
    );

    // Second press unmutes it again (and queues no disconnect).
    toggle_peer_block(&tab, &mut state, &swarm_cmd_tx);
    assert!(swarm_cmd_rx.try_recv().is_err());
    assert!(!state.blocked_peers.contains(&peer_id));
    assert!(!p2p_app::blocklist::is_blocked(&peer_id));
    assert!(
        state
            .popup
            .as_deref()
            .unwrap_or_default()
            .contains("Unblocked")
    );

    p2p_app::db::reset_db_url();
}

// ── shortcut help overlay ────────────────────────────────────────────────

#[tokio::test]
async fn test_question_mark_opens_shortcut_help_without_typing() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(
        s.popup.as_deref(),
        Some(p2p_app::tui_render::shortcuts_help_text())
    );
    assert_eq!(s.chat_input.lines().join(""), "");
}

#[tokio::test]
async fn test_any_key_dismisses_shortcut_help() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let open = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
    let _ = process_key_event(open, &state, &swarm_cmd_tx, &render_tx).await;
    while render_rx.try_recv().is_ok() {}

    let dismiss = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::empty());
    let _ = process_key_event(dismiss, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.popup.is_none());
    assert_eq!(s.chat_input.lines().join(""), "");
}

#[tokio::test]
async fn test_question_mark_is_literal_while_editing_a_nickname() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.input_mode = p2p_app::InputMode::EditingNickname;
        s.chat_input.insert_str("nick");
    }
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.popup.is_none());
    assert_eq!(s.chat_input.lines().join(""), "nick?");
}

#[tokio::test]
async fn test_ctrl_question_mark_does_not_open_shortcut_help() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('?'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.popup.is_none());
    assert_eq!(s.chat_input.lines().join(""), "");
}

// ── Contacts search key flow ─────────────────────────────────────────────

/// Press one key and drain the render channel.
///
/// `process_key_event` renders once per handled key; with a live receiver on a
/// single-slot channel the second send would block forever, so the tests drain
/// after every press.
async fn press_contacts_key(
    key: KeyEvent,
    state: &Arc<Mutex<AppState>>,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    render_tx: &mpsc::Sender<RenderEvent>,
    render_rx: &mut mpsc::Receiver<RenderEvent>,
) {
    let _ = process_key_event(key, state, swarm_cmd_tx, render_tx).await;
    while render_rx.try_recv().is_ok() {}
}

#[tokio::test]
async fn test_slash_key_on_contacts_starts_a_search_and_filters() {
    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 1; // Contacts tab
        // Give the table rows the search can match (the helper's peers carry
        // generated names, so filter on a device peer id substring).
        s.contacts_all = s.contacts.clone();
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    // A key press renders once, so the channel is drained between presses (a
    // full single-slot channel with a live receiver would block the send).
    let (render_tx, mut render_rx) = mpsc::channel(1);

    // `/` opens the search box (and consumes the key: it must not sort).
    press_contacts_key(
        KeyEvent::new(KeyCode::Char('/'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
        &mut render_rx,
    )
    .await;
    let all_rows = {
        let s = state.lock().await;
        assert!(s.is_searching_contacts());
        assert_eq!(s.input_mode, p2p_app::InputMode::SearchingContacts);
        s.contacts.len()
    };
    assert!(all_rows > 0);

    // Typing narrows the table; the query mirrors the shared input box.
    let mut query = String::new();
    for c in "zzz-no-such-contact".chars() {
        query.push(c);
        press_contacts_key(
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
            &mut render_rx,
        )
        .await;
    }
    {
        let s = state.lock().await;
        assert_eq!(s.contact_search, query);
        assert!(s.contacts.is_empty());
    }

    // Backspace widens it again (last char removed).
    press_contacts_key(
        KeyEvent::new(KeyCode::Backspace, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
        &mut render_rx,
    )
    .await;
    {
        let s = state.lock().await;
        assert_eq!(s.contact_search.len(), query.len() - 1);
    }

    // Esc exits the search and clears the filter.
    press_contacts_key(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
        &mut render_rx,
    )
    .await;
    {
        let s = state.lock().await;
        assert!(!s.is_searching_contacts());
        assert_eq!(s.contact_search, "");
        assert_eq!(s.contacts.len(), all_rows);
    }
}

#[tokio::test]
async fn test_enter_key_keeps_the_contacts_search_filter() {
    let state = Arc::new(Mutex::new(app_state_with_peers(3)));
    {
        let mut s = state.lock().await;
        s.active_tab = 1;
        s.contacts_all = s.contacts.clone();
        s.begin_contact_search();
        s.chat_input.insert_str("peer");
        s.set_contact_search("peer");
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
    let _ = process_key_event(enter, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(!s.is_searching_contacts());
    assert_eq!(s.contact_search, "peer");
    // Enter must not have opened a DM tab from the filtered selection.
    assert_eq!(s.dynamic_tabs.dm_tab_count(), 0);
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
    state.lock().await.active_tab = 1; // Contacts tab
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
    assert_eq!(s.chat_input.lines().join(""), "");
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
    assert_eq!(s.active_tab, 1); // Contacts tab
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
    state.lock().await.active_tab = 1; // Contacts tab
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
    assert_eq!(s.chat_input.lines().join(""), "");
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
    assert_eq!(s.chat_input.lines().join(""), "");
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
    assert_eq!(s.active_tab, 1); // Contacts tab
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
        s.active_tab = 1; // Contacts tab
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Tab, KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, 1); // still Contacts tab
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
    state.active_tab = 1; // Contacts tab
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

    // Shift+Enter in Contacts tab (input disabled) is a noop
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

#[test]
#[serial]
fn test_d_key_on_contacts_sends_device_intro() {
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
        let mut state = test_app_state();
        state.contacts = vec![p2p_app::contacts::ContactRow {
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
        state.peer_selection = 0;
        state.active_tab = 1; // Contacts tab
        let state = Arc::new(Mutex::new(state));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
        let (render_tx, _) = mpsc::channel(1);

        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;

        match swarm_cmd_rx.try_recv().expect("intro queued") {
            SwarmCommand::SendDeviceBinding { peer_id, request } => {
                assert_eq!(peer_id, "bob-d1");
                assert_eq!(request.kind, "intro");
            }
            other => panic!("expected SendDeviceBinding intro, got: {other:?}"),
        }
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
#[serial]
fn test_d_key_on_peer_info_sends_device_intro() {
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
        let mut state = test_app_state();
        state.active_tab = state.dynamic_tabs.add_peer_info_tab("bob-d1".to_string());
        let state = Arc::new(Mutex::new(state));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);
        let (render_tx, _) = mpsc::channel(1);

        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;

        match swarm_cmd_rx.try_recv().expect("intro queued") {
            SwarmCommand::SendDeviceBinding { peer_id, request } => {
                assert_eq!(peer_id, "bob-d1");
                assert_eq!(request.kind, "intro");
            }
            other => panic!("expected SendDeviceBinding intro, got: {other:?}"),
        }
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
#[serial(logging)]
fn test_c_key_on_log_tab_clears_logs() {
    p2p_app::init_logging();
    p2p_app::clear_tui_logs();
    p2p_app::push_log("a line to be cleared");
    assert!(
        p2p_app::get_tui_logs()
            .iter()
            .any(|l| l.contains("a line to be cleared"))
    );

    let mut state = test_app_state();
    // With no dynamic tabs, Log is the first tab after the three fixed ones.
    state.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT;
    assert_eq!(
        state.dynamic_tabs.tab_index_to_content(state.active_tab),
        TabContent::Log
    );
    let state = Arc::new(Mutex::new(state));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _) = mpsc::channel(1);

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;
    });

    assert!(
        p2p_app::get_tui_logs().is_empty(),
        "Log tab `c` must clear the buffer"
    );
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

/// The media session opens on accept (calls lane, US-044), pinning the peer and
/// call id before any frame can arrive.
fn assert_start_call_media(rx: &mut mpsc::Receiver<SwarmCommand>, expected_peer: &str) {
    match rx.try_recv().expect("start media command queued") {
        SwarmCommand::StartCallMedia { call_id, peer_id } => {
            assert_eq!(peer_id, expected_peer);
            assert_eq!(call_id, "call-1");
        }
        other => panic!("expected StartCallMedia, got: {other:?}"),
    }
}

/// Every hang-up path closes the media session so the swarm loop drops late
/// frames from the settled call (calls lane, US-044).
fn assert_stop_call_media(rx: &mut mpsc::Receiver<SwarmCommand>) {
    match rx.try_recv().expect("stop media command queued") {
        SwarmCommand::StopCallMedia => {}
        other => panic!("expected StopCallMedia, got: {other:?}"),
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
    // Accept queues both the signal and the media start; capacity 1 would
    // deadlock the second `send().await` (the swarm loop drains continuously
    // in production, but this test drives the processor directly).
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
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
    assert_start_call_media(&mut swarm_cmd_rx, "bob-d1");
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
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
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
    assert_stop_call_media(&mut swarm_cmd_rx);
}

#[tokio::test]
async fn test_call_overlay_esc_cancels_calling() {
    let state = Arc::new(Mutex::new(call_state(
        "bob-d1",
        p2p_app::CallState::Calling,
    )));
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
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
    assert_stop_call_media(&mut swarm_cmd_rx);
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
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
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
    assert_stop_call_media(&mut swarm_cmd_rx);
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

// ── reply drafts (Ctrl+R capture, Esc clear) ─────────────────────────────

fn reply_state() -> AppState {
    let mut s = test_app_state();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] older line".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("reply-wire".to_string()));
    s
}

#[tokio::test]
async fn test_ctrl_r_captures_the_newest_replyable_line() {
    let state = Arc::new(Mutex::new(reply_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    let reply = s.reply_draft(&TabContent::Chat).expect("draft armed");
    assert_eq!(reply.msg_id, "reply-wire");
    assert_eq!(reply.sender, "Alice");
    assert_eq!(reply.preview, "older line");
}

#[tokio::test]
async fn test_esc_clears_a_reply_draft_before_returning_to_chat() {
    let mut s = reply_state();
    s.dynamic_tabs.add_dm_tab("peer-dm".to_string());
    s.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT; // the DM tab
    let dm = TabContent::Direct("peer-dm".to_string());
    s.dm_messages.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from(["12:01 [Bob] dm line".to_string()]),
    );
    s.dm_message_ids.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from([Some("dm-wire".to_string())]),
    );
    assert!(s.begin_reply(&dm).is_some());
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.reply_draft(&dm).is_none());
    assert_eq!(s.active_tab, p2p_app::tui_tabs::FIXED_TAB_COUNT);
}

// ── local-only sent-message deletion (Del key + confirmation) ────────────

fn delete_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Delete, KeyModifiers::empty())
}

fn confirm_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty())
}

fn sent_broadcast_state(db_id: i32) -> AppState {
    let mut s = test_app_state();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [TestUser] delete me".to_string(),
        sender_peer_id: None,
    });
    s.message_ids.push_back(Some("wire-1".to_string()));
    s.message_db_ids.push_back(Some(db_id));
    s
}

#[tokio::test]
async fn test_delete_key_on_chat_arms_confirmation_for_newest_sent() {
    let state = Arc::new(Mutex::new(sent_broadcast_state(5)));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(delete_key(), &state, &swarm_cmd_tx, &render_tx).await;

    assert!(!exited);
    let s = state.lock().await;
    let prompt = s.delete_message_prompt.as_ref().expect("prompt armed");
    assert_eq!(prompt.message_id, 5);
    assert!(!prompt.is_group);
    assert!(prompt.preview.contains("delete me"));
    assert_eq!(prompt.tab_content, TabContent::Chat);
}

#[tokio::test]
async fn test_delete_key_without_sent_message_shows_noop_popup() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(delete_key(), &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.delete_message_prompt.is_none());
    assert_eq!(
        s.popup.as_deref(),
        Some("No sent message to delete in this conversation")
    );
}

#[tokio::test]
async fn test_delete_key_arms_prompt_on_dm_and_group_tabs() {
    let mut s = sent_broadcast_state(7);
    let dm_tab = s.dynamic_tabs.add_dm_tab("peer-dm".to_string());
    s.active_tab = dm_tab;
    s.dm_messages.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from(["dm text".to_string()]),
    );
    s.dm_message_ids.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from([Some("dm-wire".to_string())]),
    );
    s.dm_message_db_ids.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from([Some(9)]),
    );
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(delete_key(), &state, &swarm_cmd_tx, &render_tx).await;
    let prompt = state.lock().await.delete_message_prompt.clone().unwrap();
    assert_eq!(prompt.message_id, 9);
    assert_eq!(prompt.tab_content, TabContent::Direct("peer-dm".into()));

    let mut s = state.lock().await;
    s.cancel_delete_message();
    let group_tab = s
        .dynamic_tabs
        .add_group_tab("g-1".to_string(), "Group".to_string());
    s.active_tab = group_tab;
    s.group_messages.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from(["group text".to_string()]),
    );
    s.group_message_ids
        .insert("g-1".to_string(), std::collections::VecDeque::from([None]));
    s.group_message_db_ids.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from([Some(11)]),
    );
    s.group_message_peer_ids
        .insert("g-1".to_string(), std::collections::VecDeque::from([None]));
    drop(s);

    let _ = process_key_event(delete_key(), &state, &swarm_cmd_tx, &render_tx).await;
    let prompt = state.lock().await.delete_message_prompt.clone().unwrap();
    assert_eq!(prompt.message_id, 11);
    assert!(prompt.is_group);
    assert_eq!(prompt.tab_content, TabContent::GroupChat("g-1".into()));
}

#[tokio::test]
async fn test_delete_key_is_noop_outside_chat_tabs() {
    let mut s = sent_broadcast_state(5);
    s.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT - 1; // Groups list
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(delete_key(), &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(s.delete_message_prompt.is_none());
    assert_eq!(
        s.message_db_ids,
        std::collections::VecDeque::from([Some(5)])
    );
}

#[tokio::test]
async fn test_delete_prompt_esc_cancels_without_leaving_tab() {
    let mut s = sent_broadcast_state(5);
    let dm_tab = s.dynamic_tabs.add_dm_tab("peer-dm".to_string());
    s.active_tab = dm_tab;
    s.begin_delete_message(5, false, "delete me".into(), TabContent::Chat);
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.delete_message_prompt.is_none());
    assert_eq!(s.active_tab, dm_tab);
    assert_eq!(
        s.message_db_ids,
        std::collections::VecDeque::from([Some(5)])
    );
}

fn with_local_delete_db(f: impl FnOnce()) {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("delete.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    f();

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

fn saved_broadcast_meta() -> p2p_app::MessageMeta {
    p2p_app::MessageMeta {
        sender_nickname: Some("TestUser".to_string()),
        msg_id: Some("wire-1".to_string()),
        sent_at: Some(p2p_app::current_timestamp()),
        attachment: None,
        reply_to: None,
        encrypted: false,
    }
}

#[test]
#[serial]
fn test_delete_confirmation_tombstones_db_and_removes_broadcast_line() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let saved = p2p_app::save_message_with_meta(
                "delete me",
                None,
                "test-net",
                false,
                None,
                saved_broadcast_meta(),
            )
            .expect("save message");
            p2p_app::messages::mark_message_sent(saved.id).expect("mark sent");

            let mut s = sent_broadcast_state(saved.id);
            s.begin_delete_message(saved.id, false, "delete me".into(), TabContent::Chat);
            let state = Arc::new(Mutex::new(s));
            let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
            let (render_tx, mut render_rx) = mpsc::channel(4);

            let exited = process_key_event(confirm_key(), &state, &swarm_cmd_tx, &render_tx).await;

            assert!(!exited);
            assert!(
                p2p_app::load_messages("test-net", 10)
                    .expect("reload")
                    .is_empty(),
                "history loader must skip the tombstone"
            );
            assert!(swarm_cmd_rx.try_recv().is_err(), "delete is local-only");
            assert!(render_rx.try_recv().is_ok(), "confirmation triggers redraw");

            let s = state.lock().await;
            assert!(s.delete_message_prompt.is_none());
            assert!(s.messages.is_empty());
            assert!(s.message_ids.is_empty());
            assert!(s.message_db_ids.is_empty());
            assert!(s.popup.is_none());
        });
    });
}

#[tokio::test]
async fn test_bare_e_still_types_in_the_chat_composer() {
    let state = Arc::new(Mutex::new(sent_broadcast_state(5)));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty());
    assert!(!process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await);

    let s = state.lock().await;
    assert!(
        !s.is_editing_message(),
        "a printable key must never open the editor"
    );
    assert_eq!(s.chat_input.lines(), ["e"]);
}

#[test]
#[serial]
fn test_ctrl_e_and_enter_replace_db_body_without_swarm_traffic() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let saved = p2p_app::save_message_with_meta(
                "delete me",
                None,
                "test-net",
                false,
                None,
                saved_broadcast_meta(),
            )
            .expect("save message");
            p2p_app::messages::mark_message_sent(saved.id).expect("mark sent");

            let state = Arc::new(Mutex::new(sent_broadcast_state(saved.id)));
            let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
            let (render_tx, _) = mpsc::channel(4);

            let edit_key = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
            assert!(!process_key_event(edit_key, &state, &swarm_cmd_tx, &render_tx).await);
            {
                let s = state.lock().await;
                assert!(s.is_editing_message());
                assert_eq!(s.chat_input.lines(), ["delete me"]);
            }

            {
                let mut s = state.lock().await;
                s.begin_edit_message(saved.id, false, "new body".to_string(), TabContent::Chat);
            }
            let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
            assert!(!process_key_event(enter, &state, &swarm_cmd_tx, &render_tx).await);

            let loaded = p2p_app::load_messages("test-net", 10).expect("reload");
            assert_eq!(loaded.len(), 1);
            assert_eq!(loaded[0].content, "new body");
            assert!(swarm_cmd_rx.try_recv().is_err(), "edit is local-only");

            let s = state.lock().await;
            assert!(!s.is_editing_message());
            assert_eq!(s.input_mode, p2p_app::InputMode::Chat);
            assert_eq!(s.messages[0].text, "12:00 [TestUser] new body (edited)");
        });
    });
}

#[test]
#[serial]
fn test_edit_attachment_message_keeps_one_chip_and_a_clean_prefill() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let attachment = p2p_app::files::MessageAttachment {
                file_id: "abc123".to_string(),
                name: "notes.txt".to_string(),
                size: 5,
                mime: None,
            };
            let mut meta = saved_broadcast_meta();
            meta.attachment = Some(attachment.clone());
            let saved =
                p2p_app::save_message_with_meta("body", None, "test-net", false, None, meta)
                    .expect("save attached message");
            p2p_app::messages::mark_message_sent(saved.id).expect("mark sent");

            let mut state = sent_broadcast_state(saved.id);
            state.messages[0].text = format!(
                "12:00 [TestUser] {}",
                p2p_app::files::render_message_body("body", Some(&attachment))
            );
            let state = Arc::new(Mutex::new(state));
            let (swarm_cmd_tx, _) = mpsc::channel(4);
            let (render_tx, _) = mpsc::channel(4);

            let ctrl_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::CONTROL);
            assert!(!process_key_event(ctrl_e, &state, &swarm_cmd_tx, &render_tx).await);
            {
                let mut s = state.lock().await;
                assert_eq!(
                    s.chat_input.lines(),
                    ["body"],
                    "the editor starts from the stored text, not the rendered chip"
                );
                s.begin_edit_message(saved.id, false, "edited".to_string(), TabContent::Chat);
            }
            let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::empty());
            assert!(!process_key_event(enter, &state, &swarm_cmd_tx, &render_tx).await);

            let loaded = p2p_app::load_messages("test-net", 10).expect("reload");
            assert_eq!(loaded[0].content, "edited");
            let s = state.lock().await;
            let chip = p2p_app::files::file_chip(&attachment);
            assert_eq!(
                s.messages[0].text.matches(&chip).count(),
                1,
                "the chip must not be duplicated in memory or on reload"
            );
            let expected = p2p_app::fmt::with_edited_marker(&p2p_app::files::render_message_body(
                "edited",
                Some(&attachment),
            ));
            assert_eq!(s.messages[0].text, format!("12:00 [TestUser] {expected}"));
        });
    });
}

#[test]
#[serial]
fn test_delete_confirmation_tombstones_group_row_and_removes_line() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let saved = p2p_app::groups::save_outgoing_group_message(
                "g-del",
                "group delete me",
                p2p_app::groups::GroupMessageMeta {
                    sender_nickname: Some("TestUser".to_string()),
                    msg_id: Some("group-wire".to_string()),
                    sent_at: Some(p2p_app::current_timestamp()),
                    attachment: None,
                    reply_to: None,
                },
            )
            .expect("save group message");

            let mut s = test_app_state();
            let tab = s
                .dynamic_tabs
                .add_group_tab("g-del".to_string(), "Group".to_string());
            s.active_tab = tab;
            s.group_messages.insert(
                "g-del".to_string(),
                std::collections::VecDeque::from(["group delete me".to_string()]),
            );
            s.group_message_ids.insert(
                "g-del".to_string(),
                std::collections::VecDeque::from([Some("group-wire".to_string())]),
            );
            s.group_message_db_ids.insert(
                "g-del".to_string(),
                std::collections::VecDeque::from([Some(saved.id)]),
            );
            s.group_message_peer_ids.insert(
                "g-del".to_string(),
                std::collections::VecDeque::from([None]),
            );
            s.begin_delete_message(
                saved.id,
                true,
                "group delete me".into(),
                TabContent::GroupChat("g-del".into()),
            );
            let state = Arc::new(Mutex::new(s));
            let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
            let (render_tx, _) = mpsc::channel(4);

            let _ = process_key_event(confirm_key(), &state, &swarm_cmd_tx, &render_tx).await;

            assert!(
                p2p_app::groups::load_group_messages("g-del", 10)
                    .expect("reload")
                    .is_empty()
            );
            assert!(swarm_cmd_rx.try_recv().is_err());
            let s = state.lock().await;
            assert!(s.group_messages["g-del"].is_empty());
            assert!(s.group_message_db_ids["g-del"].is_empty());
            assert!(s.group_message_peer_ids["g-del"].is_empty());
            assert!(s.delete_message_prompt.is_none());
        });
    });
}

#[test]
#[serial]
fn test_delete_confirmation_failure_surfaces_popup_and_keeps_line() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let mut s = sent_broadcast_state(5);
            s.begin_delete_message(9999, false, "delete me".into(), TabContent::Chat);
            let state = Arc::new(Mutex::new(s));
            let (swarm_cmd_tx, _) = mpsc::channel(4);
            let (render_tx, _) = mpsc::channel(4);

            let _ = process_key_event(confirm_key(), &state, &swarm_cmd_tx, &render_tx).await;

            let s = state.lock().await;
            assert!(s.delete_message_prompt.is_none());
            assert_eq!(
                s.message_db_ids,
                std::collections::VecDeque::from([Some(5)])
            );
            assert_eq!(s.messages.len(), 1);
            assert!(
                s.popup
                    .as_deref()
                    .is_some_and(|p| p.starts_with("Could not delete message:")),
                "DB failure must be surfaced: {:?}",
                s.popup
            );
        });
    });
}

// ── persisted network peers (Settings b/r/x) ─────────────────────────────

#[test]
fn test_settings_shortcut_begins_network_peer_prompts() {
    let mut state = test_app_state();

    handle_settings_shortcut('b', &mut state);
    assert!(state.is_adding_network_peer());
    assert_eq!(
        state.network_peer_kind,
        Some(p2p_app::network_config::NetworkPeerKind::Bootstrap)
    );
    state.cancel_add_network_peer();

    handle_settings_shortcut('r', &mut state);
    assert!(state.is_adding_network_peer());
    assert_eq!(
        state.network_peer_kind,
        Some(p2p_app::network_config::NetworkPeerKind::Relay)
    );
}

#[test]
fn test_settings_shortcut_x_without_a_selection_is_a_noop() {
    let mut state = test_app_state();
    handle_settings_shortcut('x', &mut state);
    assert_eq!(state.network_peers, Vec::new());
    assert!(state.popup.is_none());
}

#[test]
fn test_network_peer_submission_ignores_empty_input() {
    let mut state = test_app_state();
    state.begin_add_network_peer(p2p_app::network_config::NetworkPeerKind::Bootstrap);
    handle_add_network_peer_submission(&mut state);
    assert!(!state.is_adding_network_peer());
    assert_eq!(state.network_peers, Vec::new());
    assert!(state.popup.is_none());
}

#[test]
#[serial]
fn test_network_peer_submission_persists_and_removes() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let mut state = test_app_state();
    state.begin_add_network_peer(p2p_app::network_config::NetworkPeerKind::Bootstrap);
    state.chat_input.insert_str(" /ip4/1.2.3.4/tcp/4001 ");
    handle_add_network_peer_submission(&mut state);

    assert!(!state.is_adding_network_peer(), "prompt closes on submit");
    assert!(state.popup.is_none(), "valid input has no error popup");
    assert_eq!(state.network_peers.len(), 1);
    assert_eq!(state.network_peers[0].multiaddr, "/ip4/1.2.3.4/tcp/4001");
    assert_eq!(
        state.network_peer_selection, 0,
        "the saved row is highlighted"
    );
    assert_eq!(
        p2p_app::network_config::list_network_peers()
            .expect("list")
            .len(),
        1
    );

    remove_selected_network_peer(&mut state);
    assert_eq!(state.network_peers, Vec::new());
    assert_eq!(
        p2p_app::network_config::list_network_peers()
            .expect("list")
            .len(),
        0
    );

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
#[serial]
fn test_network_peer_submission_reports_validation_errors() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let mut state = test_app_state();
    state.begin_add_network_peer(p2p_app::network_config::NetworkPeerKind::Relay);
    // A relay needs a /p2p/<peer-id> suffix.
    state.chat_input.insert_str("/ip4/1.2.3.4/tcp/4001");
    handle_add_network_peer_submission(&mut state);

    assert!(!state.is_adding_network_peer());
    assert_eq!(state.network_peers, Vec::new(), "nothing persisted");
    assert_eq!(
        p2p_app::network_config::list_network_peers()
            .expect("list")
            .len(),
        0
    );
    let popup = state.popup.expect("validation popup");
    assert!(
        popup.contains("relay address needs a /p2p/<peer-id> suffix"),
        "got: {popup}"
    );

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── persisted network peers: the add prompt owns printable keys ──────────

fn settings_tab_index(state: &AppState) -> usize {
    state.dynamic_tabs.total_tab_count().saturating_sub(1)
}

#[tokio::test]
async fn test_settings_add_prompt_owns_printable_keys() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.active_tab = settings_tab_index(&s);
    }
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    // `b` opens the bootstrap prompt. Each handled key renders once, so the
    // single-slot render channel must be drained between presses or the
    // second `send` blocks forever.
    let key = KeyEvent::new(KeyCode::Char('b'), KeyModifiers::empty());
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;
    while render_rx.try_recv().is_ok() {}

    // A second `b` must be captured as text, not restart the prompt.
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;
    while render_rx.try_recv().is_ok() {}

    let s = state.lock().await;
    assert_eq!(
        s.network_peer_kind,
        Some(p2p_app::network_config::NetworkPeerKind::Bootstrap)
    );
    assert_eq!(s.chat_input.lines().join("\n"), "b");
}

// ── Ctrl+O open received attachment (Flutter parity) ───────────────────────

#[tokio::test]
async fn test_ctrl_o_without_attachment_shows_popup_and_does_not_type() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert_eq!(
        s.popup.as_deref(),
        Some("No downloaded attachment in this conversation")
    );
    // The modifier must keep `o` out of the composer.
    assert_eq!(s.chat_input.lines().join(""), "");
}

#[tokio::test]
async fn test_ctrl_o_is_inert_on_non_chat_tabs() {
    let mut s = test_app_state();
    s.active_tab = 1; // the Contacts fixed tab (fixed order: Chat, Contacts, Groups)
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.popup.is_none());
}

// ── Reaction editor (US-008) ────────────────────────────────────────────
//
// These drive the real interactive key path (`process_key_event`), mirroring
// the Flutter widget tests for the same user story: Ctrl+Y opens the shared
// editor on the newest message, Enter validates + persists + refreshes chips,
// Esc cancels, and invalid tokens are rejected without touching stored data.

/// Point the active tab at the DM tab the helper created, so key dispatch
/// resolves the transcript (and its wire ids) the reaction editor reads.
fn focus_dm_tab(state: &mut AppState) {
    let idx = (0..state.dynamic_tabs.total_tab_count())
        .find(|&i| {
            matches!(
                state.dynamic_tabs.tab_index_to_content(i),
                TabContent::Direct(_)
            )
        })
        .expect("helper created a DM tab");
    state.active_tab = idx;
}

#[tokio::test]
async fn test_ctrl_y_opens_reaction_editor_on_newest_message() {
    let mut state = app_state_with_dm_messages("react-peer", 3);
    focus_dm_tab(&mut state);
    let state = Arc::new(Mutex::new(state));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;
    assert!(!exited);

    let s = state.lock().await;
    assert!(s.is_adding_reaction(), "editor opened");
    assert_eq!(s.input_mode, p2p_app::InputMode::AddingReaction);
    assert_eq!(
        s.adding_reaction.as_ref().map(|p| p.msg_id.as_str()),
        Some("dm-2"),
        "targets the newest wire id in the DM transcript"
    );
    assert_eq!(s.chat_input.lines().join(""), "", "editor starts empty");
}

#[tokio::test]
async fn test_ctrl_y_without_a_message_shows_a_popup() {
    // The Chat tab of a fresh state has no transcript, so there is nothing to
    // react to; the shortcut must explain that instead of opening an editor.
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(!s.is_adding_reaction());
    assert_eq!(
        s.popup.as_deref(),
        Some("No message to react to in this conversation")
    );
}

#[tokio::test]
async fn test_esc_cancels_the_reaction_editor() {
    let mut state = app_state_with_dm_messages("react-peer", 1);
    focus_dm_tab(&mut state);
    let state = Arc::new(Mutex::new(state));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let open = KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL);
    let _ = process_key_event(open, &state, &swarm_cmd_tx, &render_tx).await;
    assert!(state.lock().await.is_adding_reaction(), "precondition");

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(!s.is_adding_reaction(), "editor closed");
    assert_eq!(s.input_mode, p2p_app::InputMode::Chat);
    assert_eq!(s.chat_input.lines().join(""), "");
    assert!(s.reactions.is_empty(), "cancelling stored nothing");
}

#[test]
#[serial]
fn test_enter_persists_reaction_and_refreshes_chips() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");
    let local_peer = p2p_app::peers::local_peer_id_string().expect("local peer id");
    // Seed the wire message so reaction fan-out derives the DM partner from the
    // message's scope, exactly as a real received DM would be persisted.
    p2p_app::messages::save_message_with_meta(
        "hello",
        Some("react-peer"),
        "dm",
        true,
        None,
        p2p_app::messages::MessageMeta {
            msg_id: Some("dm-0".to_string()),
            ..Default::default()
        },
    )
    .expect("seed dm message");

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = app_state_with_dm_messages("react-peer", 1);
        focus_dm_tab(&mut state);
        let state = Arc::new(Mutex::new(state));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
        let (render_tx, _render_rx) = mpsc::channel(4);

        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;
        // Whitespace around a pasted token is trimmed by the canonical rule.
        state.lock().await.chat_input.insert_str("  🚀  ");
        let _ = process_key_event(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;

        // The network half: a `SendReaction` must fan out to the DM partner
        // with the trimmed token and this install's peer id, not a broadcast.
        let cmd = tokio::time::timeout(std::time::Duration::from_millis(100), swarm_cmd_rx.recv())
            .await
            .expect("timeout")
            .expect("expected SendReaction");
        match cmd {
            SwarmCommand::SendReaction { peer_ids, signal } => {
                assert_eq!(peer_ids, vec!["react-peer".to_string()]);
                assert_eq!(signal.msg_id, "dm-0");
                assert_eq!(signal.emoji, "🚀");
                assert_eq!(signal.actor, local_peer);
                assert!(!signal.remove, "first reaction is an add");
            }
            _ => panic!("expected SendReaction"),
        }

        let s = state.lock().await;
        assert!(!s.is_adding_reaction(), "editor consumed on success");
        assert_eq!(s.input_mode, p2p_app::InputMode::Chat);
        assert!(s.popup.is_none(), "unexpected popup: {:?}", s.popup);
        assert!(
            p2p_app::reactions::has_reaction("dm-0", &local_peer, "🚀").expect("has_reaction"),
            "reaction persisted under the trimmed token"
        );
        let chips = s.reactions.get("dm-0").expect("chips refreshed");
        assert_eq!(chips.len(), 1);
        assert_eq!(chips[0].emoji, "🚀");
        assert_eq!(chips[0].count, 1);
        assert!(chips[0].mine, "own reaction is marked mine");
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
#[serial]
fn test_reapplying_reaction_toggles_off_and_signals_remove() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");
    let local_peer = p2p_app::peers::local_peer_id_string().expect("local peer id");
    p2p_app::messages::save_message_with_meta(
        "hello",
        Some("react-peer"),
        "dm",
        true,
        None,
        p2p_app::messages::MessageMeta {
            msg_id: Some("dm-0".to_string()),
            ..Default::default()
        },
    )
    .expect("seed dm message");

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = app_state_with_dm_messages("react-peer", 1);
        focus_dm_tab(&mut state);
        let state = Arc::new(Mutex::new(state));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(4);
        let (render_tx, _render_rx) = mpsc::channel(4);

        // Apply the same emoji twice through the real key path; the second
        // activation must toggle it off, not inflate the count.
        for expected_remove in [false, true] {
            let _ = process_key_event(
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
                &state,
                &swarm_cmd_tx,
                &render_tx,
            )
            .await;
            state.lock().await.chat_input.insert_str("👍");
            let _ = process_key_event(
                KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
                &state,
                &swarm_cmd_tx,
                &render_tx,
            )
            .await;

            let cmd =
                tokio::time::timeout(std::time::Duration::from_millis(100), swarm_cmd_rx.recv())
                    .await
                    .expect("timeout")
                    .expect("expected SendReaction");
            match cmd {
                SwarmCommand::SendReaction { peer_ids, signal } => {
                    assert_eq!(peer_ids, vec!["react-peer".to_string()]);
                    assert_eq!(signal.emoji, "👍");
                    assert_eq!(signal.actor, local_peer);
                    assert_eq!(signal.remove, expected_remove);
                }
                _ => panic!("expected SendReaction"),
            }
        }

        let s = state.lock().await;
        assert!(
            !p2p_app::reactions::has_reaction("dm-0", &local_peer, "👍").expect("has_reaction"),
            "re-applying removed the reaction"
        );
        assert!(
            s.reactions.get("dm-0").is_none_or(Vec::is_empty),
            "chips refreshed to empty after toggle-off"
        );
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
#[serial]
fn test_enter_rejects_invalid_reaction_and_keeps_editor() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");
    let local_peer = p2p_app::peers::local_peer_id_string().expect("local peer id");

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = app_state_with_dm_messages("react-peer", 1);
        focus_dm_tab(&mut state);
        let state = Arc::new(Mutex::new(state));
        let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
        let (render_tx, _render_rx) = mpsc::channel(4);

        let _ = process_key_event(
            KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;
        let too_long = "x".repeat(p2p_app::reactions::MAX_EMOJI_LEN + 1);
        state.lock().await.chat_input.insert_str(&too_long);
        let _ = process_key_event(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()),
            &state,
            &swarm_cmd_tx,
            &render_tx,
        )
        .await;

        let s = state.lock().await;
        assert!(s.is_adding_reaction(), "editor stays open for correction");
        assert!(
            s.popup
                .as_deref()
                .is_some_and(|p| p.starts_with("Invalid reaction:")),
            "validation error surfaced: {:?}",
            s.popup
        );
        assert!(s.reactions.is_empty(), "nothing stored on rejection");
        assert!(
            !p2p_app::reactions::has_reaction("dm-0", &local_peer, &too_long)
                .expect("has_reaction"),
            "invalid token never reached the database"
        );
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}
