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
use crate::tui::test_helpers::KeySession;
use crate::tui::test_helpers::app_state_with_peers;
use crate::tui::test_helpers::{
    app_state_with_chat_messages, app_state_with_dm_messages, test_app_state,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use p2p_app::tui_tabs::TabContent;
use serial_test::serial;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, mpsc};

fn wheel_event(kind: MouseEventKind) -> MouseEvent {
    MouseEvent {
        kind,
        column: 12,
        row: 7,
        modifiers: KeyModifiers::NONE,
    }
}

fn left_click_event(row: u16, column: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::NONE,
    }
}

/// Drive a wheel scroll plus a tab-title click through the real mouse path.
/// A modal surface must reject both without changing state or queuing commands,
/// and the rendered frame must still show the owning surface.
async fn assert_mouse_is_inert<F>(state: AppState, marker: &str, click_column: u16, check: F)
where
    F: Fn(&AppState),
{
    let mut session = KeySession::new(state);
    assert!(
        !session
            .press_mouse(wheel_event(MouseEventKind::ScrollDown))
            .await
    );
    assert!(!session.press_mouse(left_click_event(0, click_column)).await);

    let frame = session.frame_text().await;
    assert!(
        frame.contains(marker),
        "missing {marker:?} in rendered frame"
    );
    {
        let s = session.state.lock().await;
        check(&s);
    }
    assert!(
        session.drain_commands().is_empty(),
        "an inert mouse event must not queue a swarm command"
    );
}

// ── update_dm_transcript_labels ──────────────────────────────────────────

#[test]
fn test_update_dm_transcript_labels_changes_nick() {
    let mut state = test_app_state();
    state.dm_messages.insert(
        "peer1".to_string(),
        std::collections::VecDeque::from(vec![
            "[OldNick] hello".to_string(),
            "[OldNick] how are you".to_string(),
            "[Other] hi".to_string(),
        ]),
    );
    update_dm_transcript_labels(&mut state, "peer1", "OldNick", "NewNick");
    let msgs = state.dm_messages.get("peer1").unwrap();
    assert_eq!(msgs[0], "[NewNick] hello");
    assert_eq!(msgs[1], "[NewNick] how are you");
    assert_eq!(msgs[2], "[Other] hi");
}

#[test]
fn test_update_dm_transcript_labels_no_match_does_nothing() {
    let mut state = test_app_state();
    state.dm_messages.insert(
        "peer1".to_string(),
        std::collections::VecDeque::from(vec!["[Other] hello".to_string()]),
    );
    update_dm_transcript_labels(&mut state, "peer1", "OldNick", "NewNick");
    assert_eq!(state.dm_messages.get("peer1").unwrap()[0], "[Other] hello");
}

#[test]
fn test_update_dm_transcript_labels_unknown_peer() {
    let mut state = test_app_state();
    update_dm_transcript_labels(&mut state, "unknown", "Old", "New");
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

/// Send both wheel directions through the real dispatcher and assert the list
/// selection moves like the arrow keys, then returns, with a rendered frame on
/// the destination tab.
async fn assert_wheel_moves_list_selection<F>(
    state: AppState,
    frame_marker: &str,
    selected_marker: &str,
    selection: F,
) where
    F: Fn(&AppState) -> usize,
{
    let initial = selection(&state);
    let mut session = KeySession::new(state);

    assert!(
        !session
            .press_mouse(wheel_event(MouseEventKind::ScrollDown))
            .await
    );
    {
        let s = session.state.lock().await;
        assert_eq!(selection(&s), initial.saturating_add(1));
    }
    let frame = session.frame_text().await;
    assert!(
        frame.contains(frame_marker),
        "missing {frame_marker} in rendered frame"
    );
    assert!(
        frame.contains(selected_marker),
        "selected row {selected_marker} is not rendered"
    );

    assert!(
        !session
            .press_mouse(wheel_event(MouseEventKind::ScrollUp))
            .await
    );
    {
        let s = session.state.lock().await;
        assert_eq!(selection(&s), initial);
    }
}

#[tokio::test]
async fn test_mouse_wheel_moves_contacts_selection_through_real_dispatch() {
    let mut state = app_state_with_peers(3);
    state.active_tab = 1;
    state.peer_selection = 0;
    let selected_marker = state.contacts[1].display_name.clone();
    assert_wheel_moves_list_selection(state, "Contacts", &selected_marker, |s| s.peer_selection)
        .await;
}

#[tokio::test]
async fn test_mouse_wheel_moves_groups_selection_through_real_dispatch() {
    let mut state = groups_tab_state("g-wheel-1", "First");
    state
        .group_summaries
        .push(group_summary("g-wheel-2", "Second"));
    state.group_selection = 0;
    assert_wheel_moves_list_selection(state, "Groups", "Second", |s| s.group_selection).await;
}

#[tokio::test]
async fn test_mouse_wheel_moves_settings_selection_through_real_dispatch() {
    let mut state = test_app_state();
    state.active_tab = settings_tab_index(&state);
    state.network_peers = vec![
        p2p_app::network_config::NetworkPeer {
            id: 1,
            kind: "bootstrap".to_string(),
            multiaddr: "/ip4/127.0.0.1/tcp/4001".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        },
        p2p_app::network_config::NetworkPeer {
            id: 2,
            kind: "bootstrap".to_string(),
            multiaddr: "/ip4/127.0.0.2/tcp/4001".to_string(),
            created_at: "2026-01-01 00:00:00".to_string(),
        },
    ];
    state.network_peer_selection = 0;
    assert_wheel_moves_list_selection(state, "Settings", "/ip4/127.0.0.2/tcp/4001", |s| {
        s.network_peer_selection
    })
    .await;
}

#[tokio::test]
async fn test_mouse_wheel_moves_shares_selection_through_real_dispatch() {
    let share = |id: &str| p2p_app::shares::Share {
        share_id: id.to_string(),
        owner_user_id: "user".to_string(),
        name: id.to_string(),
        kind: p2p_app::shares::ShareKind::Private,
        public_write: false,
        local_root: None,
        revision: 1,
        manifest: None,
    };
    let mut state = test_app_state();
    state.active_tab = shares_tab_index(&state);
    state.shares = vec![share("s-wheel-1"), share("s-wheel-2")];
    state.share_selection = 0;
    assert_wheel_moves_list_selection(state, "Shares", "s-wheel-2", |s| s.share_selection).await;
}

#[test]
#[serial]
fn test_share_selection_resets_grant_cursor_through_real_mouse_input() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let share = |id: &str| p2p_app::shares::Share {
            share_id: id.to_string(),
            owner_user_id: "user".to_string(),
            name: id.to_string(),
            kind: p2p_app::shares::ShareKind::Private,
            public_write: false,
            local_root: None,
            revision: 1,
            manifest: None,
        };
        let grant = |user_id: &str| p2p_app::shares::ShareGrant {
            user_id: user_id.to_string(),
            mode: p2p_app::shares::ShareMode::ReadOnly,
        };
        let mut state = test_app_state();
        state.active_tab = shares_tab_index(&state);
        state.terminal_width = 160;
        state.chat_area_height = 20;
        state.shares = vec![share("s-grant-1"), share("s-grant-2")];
        state
            .share_grants
            .insert("s-grant-1".to_string(), vec![grant("alice"), grant("bob")]);
        state
            .share_grants
            .insert("s-grant-2".to_string(), vec![grant("carol")]);
        state.share_selection = 0;
        state.share_grant_selection = 1;
        let mut session = KeySession::new(state);

        assert!(
            !session
                .press_mouse(wheel_event(MouseEventKind::ScrollDown))
                .await
        );
        {
            let s = session.state.lock().await;
            assert_eq!(s.share_selection, 1);
            assert_eq!(
                s.share_grant_selection, 0,
                "wheel cannot carry a grant index"
            );
        }
        assert!(session.frame_text().await.contains("s-grant-2"));

        // A click back to the first share must also reset the cursor: the
        // pointer path shares the same selection method as the wheel.
        assert!(!session.press_mouse(left_click_event(2, 1)).await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.share_selection, 0);
            assert_eq!(s.share_grant_selection, 0);
        }
        assert!(session.drain_commands().is_empty());
    });
}

#[tokio::test]
async fn test_mouse_wheel_on_peer_info_is_noop_without_redraw() {
    let state = Arc::new(Mutex::new(test_app_state()));
    {
        let mut s = state.lock().await;
        s.active_tab = s.dynamic_tabs.add_peer_info_tab("peer-wheel".to_string());
    }
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);
    let (render_tx, mut render_rx) = mpsc::channel(1);

    let exited = process_input_event(
        InputEvent::Mouse(wheel_event(MouseEventKind::ScrollDown)),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert!(render_rx.try_recv().is_err(), "no-op wheel must not redraw");
    let s = state.lock().await;
    assert_eq!(s.peer_selection, 0);
    assert_eq!(s.chat_scroll_offset, 0);
}

// ── mouse ownership matches the modal/keyboard precedence ─────────────

/// A Contacts-backed state whose wheel would move the highlighted row and whose
/// row-0 click at column 1 would leave for the Chat tab without the mouse guard.
fn overlay_contacts_state() -> AppState {
    let mut s = app_state_with_peers(3);
    s.active_tab = 1;
    s.peer_selection = 0;
    s
}

#[tokio::test]
async fn test_device_binding_prompt_swallows_wheel_and_tab_click() {
    let mut s = overlay_contacts_state();
    s.device_binding_prompt = Some(crate::tui::state::DeviceBindingPrompt {
        peer_id: "target-device".to_string(),
        device_pubkey: vec![0xAB, 0xCD],
    });
    assert_mouse_is_inert(s, "Device-binding intro", 1, |s| {
        let prompt = s
            .device_binding_prompt
            .as_ref()
            .expect("prompt still owns the screen");
        assert_eq!(prompt.peer_id, "target-device");
        assert_eq!(prompt.device_pubkey, vec![0xAB, 0xCD]);
        assert_eq!(s.active_tab, 1);
        assert_eq!(s.peer_selection, 0);
    })
    .await;
}

#[tokio::test]
async fn test_group_leave_prompt_swallows_wheel_and_tab_click() {
    let mut s = groups_tab_state("g-mouse-1", "First");
    s.group_summaries.push(group_summary("g-mouse-2", "Second"));
    s.group_selection = 0;
    s.begin_group_leave("g-mouse-1".to_string(), "First".to_string());
    assert_mouse_is_inert(s, "Leave group", 1, |s| {
        let prompt = s
            .group_leave_prompt
            .as_ref()
            .expect("prompt still owns the screen");
        assert_eq!(prompt.group_id, "g-mouse-1");
        assert_eq!(s.active_tab, 2);
        assert_eq!(s.group_selection, 0);
        assert_eq!(s.group_summaries.len(), 2);
    })
    .await;
}

#[tokio::test]
async fn test_delete_prompt_swallows_wheel_and_tab_click() {
    let mut s = app_state_with_chat_messages(12);
    s.chat_area_height = 2;
    s.chat_scroll_offset = 0;
    s.begin_delete_message(5, false, "delete me".to_string(), TabContent::Chat);
    assert_mouse_is_inert(s, "Delete this sent message", 12, |s| {
        let prompt = s
            .delete_message_prompt
            .as_ref()
            .expect("prompt still owns the screen");
        assert_eq!(prompt.message_id, 5);
        assert_eq!(s.active_tab, 0);
        assert_eq!(s.chat_scroll_offset, 0);
        assert!(s.auto_scroll.chat);
    })
    .await;
}

#[tokio::test]
async fn test_call_overlay_swallows_wheel_and_tab_click() {
    let mut s = overlay_contacts_state();
    s.call_session = Some(p2p_app::CallSession {
        call_id: "call-mouse".to_string(),
        peer_id: "bob-d1".to_string(),
        state: p2p_app::CallState::InCall,
    });
    assert_mouse_is_inert(s, "In call with", 1, |s| {
        let call = s.call_session.as_ref().expect("call still owns the screen");
        assert_eq!(call.call_id, "call-mouse");
        assert_eq!(call.state, p2p_app::CallState::InCall);
        assert_eq!(s.active_tab, 1);
        assert_eq!(s.peer_selection, 0);
    })
    .await;
}

#[tokio::test]
async fn test_modal_editor_swallows_wheel_and_tab_click() {
    let mut s = overlay_contacts_state();
    s.begin_self_nickname_edit();
    s.chat_input.insert_str("draft");
    assert_mouse_is_inert(s, "Edit Nickname", 1, |s| {
        assert!(s.is_editing_nickname());
        assert_eq!(s.chat_input.lines().join(""), "draft");
        assert_eq!(s.active_tab, 1);
        assert_eq!(s.peer_selection, 0);
    })
    .await;
}

/// A visible validation popup is dismissed by a click even while an editor
/// owns the keyboard; with no popup left, the editor keeps ownership of the
/// pointer. The wheel never dismisses the popup and never edits the table.
#[tokio::test]
async fn test_click_dismisses_popup_before_modal_editor_keeps_pointer() {
    let mut state = overlay_contacts_state();
    state.begin_dial_peer();
    state.chat_input.insert_str("/ip4/1.2.3.4/tcp/4001");
    state.popup = Some("validation hint for click test".to_string());
    let mut session = KeySession::new(state);

    assert!(
        session
            .frame_text()
            .await
            .contains("validation hint for click test")
    );
    assert!(
        !session
            .press_mouse(wheel_event(MouseEventKind::ScrollDown))
            .await
    );
    assert!(!session.press_mouse(left_click_event(0, 1)).await);

    let frame = session.frame_text().await;
    assert!(
        frame.contains("Dial peer address"),
        "editor hint missing: {frame}"
    );
    assert!(!frame.contains("validation hint for click test"));
    {
        let s = session.state.lock().await;
        assert_eq!(s.popup, None);
        assert!(s.is_dialing_peer());
        assert_eq!(s.chat_input.lines().join(""), "/ip4/1.2.3.4/tcp/4001");
        assert_eq!(s.active_tab, 1);
        assert_eq!(s.peer_selection, 0);
    }
    assert!(session.drain_commands().is_empty());
}

/// Contacts search is an inline filter, not a modal editor: the wheel still
/// navigates the roster and a tab click still commits the query through the
/// normal focus-finalization path.
#[tokio::test]
async fn test_contacts_search_keeps_click_to_finalize_and_wheel_selection() {
    let mut state = overlay_contacts_state();
    state.begin_contact_search();
    state.set_contact_search("peer");
    let mut session = KeySession::new(state);

    assert!(session.frame_text().await.contains("Search Contacts"));
    assert!(
        !session
            .press_mouse(wheel_event(MouseEventKind::ScrollDown))
            .await
    );
    {
        let s = session.state.lock().await;
        assert_eq!(s.peer_selection, 1);
        assert!(s.is_searching_contacts());
        assert_eq!(s.contact_search, "peer");
    }

    // Column 1 is the Chat tab title; the click must switch focus and finalize
    // the inline search while keeping its applied filter.
    assert!(!session.press_mouse(left_click_event(0, 1)).await);
    let frame = session.frame_text().await;
    assert!(frame.contains("Chat"), "Chat tab missing: {frame}");
    {
        let s = session.state.lock().await;
        assert_eq!(s.active_tab, 0);
        assert_eq!(s.input_mode, p2p_app::InputMode::Chat);
        assert_eq!(s.contact_search, "peer");
        assert_eq!(s.chat_input.lines().join(""), "");
    }
    assert!(session.drain_commands().is_empty());
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

/// Drive a real Tab/BackTab press from an active Contacts search. The query
/// stays as the applied filter, the shared editor is empty on the destination
/// tab, and the rendered input title no longer claims the search box is open.
async fn assert_contact_search_finalized_on_tab_key(key: KeyCode, expected_tab: usize) {
    let mut state = app_state_with_peers(3);
    state.active_tab = 1;
    state.contacts_all = state.contacts.clone();
    let mut session = KeySession::new(state);

    assert!(!session.press_char('/').await);
    assert!(!session.type_text("peer1").await);
    {
        let s = session.state.lock().await;
        assert!(s.is_searching_contacts());
        assert_eq!(s.contact_search, "peer1");
        assert_eq!(s.chat_input.lines().join("\n"), "peer1");
        assert_eq!(s.contacts.len(), 1);
    }
    assert!(
        session.frame_text().await.contains("Search Contacts"),
        "search box should be visible before switching tabs"
    );

    assert!(!session.press_code(key).await);
    {
        let s = session.state.lock().await;
        assert_eq!(s.active_tab, expected_tab);
        assert!(!s.is_searching_contacts());
        assert_eq!(s.contact_search, "peer1", "the applied filter survives");
        assert_eq!(s.chat_input.lines().join("\n"), "");
        assert_eq!(s.contacts.len(), 1);
    }
    assert!(
        !session.frame_text().await.contains("Search Contacts"),
        "search box followed the user to the destination tab"
    );
}

#[tokio::test]
async fn test_tab_out_of_contacts_search_finalizes_the_query() {
    assert_contact_search_finalized_on_tab_key(KeyCode::Tab, 2).await;
}

#[tokio::test]
async fn test_back_tab_out_of_contacts_search_finalizes_the_query() {
    assert_contact_search_finalized_on_tab_key(KeyCode::BackTab, 0).await;
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
async fn test_ctrl_p_in_chat_opens_newest_sender_peer_info() {
    // Chat has no single partner: Ctrl+P targets the newest row with a known
    // sender, and must never type a literal 'p' into the composer.
    let state = Arc::new(Mutex::new(app_state_with_chat_messages(3)));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(
        matches!(&content, TabContent::PeerInfo(id) if id == "peer2"),
        "unexpected tab: {content:?}"
    );
    assert_eq!(s.chat_input.lines().join(""), "");
}

#[tokio::test]
async fn test_ctrl_p_in_group_chat_opens_newest_known_sender() {
    let mut initial = app_state_with_dm_messages("peer-dm", 3);
    let group_tab = initial
        .dynamic_tabs
        .add_group_tab("g-1".to_string(), "Group".to_string());
    initial.active_tab = group_tab;
    initial.group_messages.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from([
            "own".to_string(),
            "from A".to_string(),
            "from B".to_string(),
        ]),
    );
    initial.group_message_ids.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from([None, Some("m-1".to_string()), Some("m-2".to_string())]),
    );
    initial.group_message_db_ids.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from([None, None, None]),
    );
    initial.group_message_peer_ids.insert(
        "g-1".to_string(),
        std::collections::VecDeque::from([
            None,
            Some("sender-a".to_string()),
            Some("sender-b".to_string()),
        ]),
    );
    let state = Arc::new(Mutex::new(initial));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(
        matches!(&content, TabContent::PeerInfo(id) if id == "sender-b"),
        "unexpected tab: {content:?}"
    );
}

#[tokio::test]
async fn test_ctrl_p_is_noop_on_log_tab() {
    // Log lines have no sender, so Ctrl+P is consumed as a no-op and never
    // typed as literal 'p'.
    let initial = app_state_with_dm_messages("peer-dm", 3);
    let log_tab = (0..initial.dynamic_tabs.total_tab_count())
        .find(|&i| {
            matches!(
                initial.dynamic_tabs.tab_index_to_content(i),
                TabContent::Log
            )
        })
        .expect("test state has a Log tab");
    let state = Arc::new(Mutex::new(initial));
    state.lock().await.active_tab = log_tab;
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert_eq!(s.active_tab, log_tab);
    assert_eq!(s.chat_input.lines().join(""), "");
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
}

#[tokio::test]
async fn test_ctrl_p_on_settings_prefers_first_connected_peer() {
    // Settings has two peer-bearing sections. Ctrl+P targets the first
    // connected peer by ID, not the first bound device or a HashSet order.
    let initial = {
        let _guard = p2p_app::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("settings-ctrl-p.db");
        p2p_app::db::set_db_url(db_path.to_str().unwrap());
        p2p_app::db::init_database().unwrap();

        let mut s = test_app_state();
        s.active_tab = settings_tab_index(&s);
        s.connected.on_peer_connected("peer-z".to_string());
        s.connected.on_peer_connected("peer-a".to_string());
        s.own_user_devices = vec![
            ("device-z".to_string(), "later".to_string()),
            ("device-a".to_string(), "earlier".to_string()),
        ];
        s
    };
    let state = Arc::new(Mutex::new(initial));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(
        matches!(&content, TabContent::PeerInfo(id) if id == "peer-a"),
        "unexpected tab: {content:?}"
    );
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 1);
    assert_eq!(s.chat_input.lines().join(""), "");
    drop(s);
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_ctrl_p_on_settings_falls_back_to_bound_device() {
    let initial = {
        let _guard = p2p_app::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("settings-ctrl-p.db");
        p2p_app::db::set_db_url(db_path.to_str().unwrap());
        p2p_app::db::init_database().unwrap();

        let mut s = test_app_state();
        s.active_tab = settings_tab_index(&s);
        s.own_user_devices = vec![
            ("device-z".to_string(), "later".to_string()),
            ("device-a".to_string(), "earlier".to_string()),
        ];
        s
    };
    let state = Arc::new(Mutex::new(initial));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    let content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
    assert!(
        matches!(&content, TabContent::PeerInfo(id) if id == "device-a"),
        "unexpected tab: {content:?}"
    );
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 1);
    drop(s);
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_ctrl_p_on_settings_without_targets_is_noop() {
    let initial = {
        let _guard = p2p_app::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("settings-ctrl-p.db");
        p2p_app::db::set_db_url(db_path.to_str().unwrap());
        p2p_app::db::init_database().unwrap();

        let mut s = test_app_state();
        s.active_tab = settings_tab_index(&s);
        s
    };
    let state = Arc::new(Mutex::new(initial));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);

    let key = KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL);
    let _ = process_key_event(key, &state, &swarm_cmd_tx, &render_tx).await;

    let s = state.lock().await;
    assert!(matches!(
        s.dynamic_tabs.tab_index_to_content(s.active_tab),
        TabContent::Settings
    ));
    assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
    assert_eq!(s.chat_input.lines().join(""), "");
    drop(s);
    p2p_app::db::reset_db_url();
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

/// The global (Settings-tab) and per-peer "self nickname" paths used to bypass
/// the canonical validator entirely: they persisted whatever text was typed.
/// Invalid input must now keep the editor open with a hint, exactly like the
/// local-nickname path, and must not touch state or emit a nickname DM.
#[test]
fn test_nickname_submission_rejects_invalid_and_keeps_open() {
    let mut s = test_app_state();
    s.input_mode = p2p_app::InputMode::EditingNickname;
    s.own_nickname = "OldGlobal".to_string();
    s.chat_input.insert_str("bad name!");
    let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_nickname_submission(&mut s, &swarm_cmd_tx));

    assert!(
        s.is_editing_nickname(),
        "editor must stay open on invalid input"
    );
    assert_eq!(s.own_nickname, "OldGlobal", "nothing must be persisted");
    let popup = s.popup.as_deref().expect("popup");
    assert!(popup.contains("letters, numbers or dashes"));
    assert!(
        swarm_cmd_rx.try_recv().is_err(),
        "no nickname DM on invalid input"
    );
}

/// A valid global nickname is trimmed before validation/persistence, matching
/// Flutter's `ctrl.text.trim()` at the FRB boundary.
#[test]
fn test_nickname_submission_trims_valid_input() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = TempDir::new().unwrap();
    let db_path = _dir.path().join("trim-nick.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let mut s = test_app_state();
    s.input_mode = p2p_app::InputMode::EditingNickname;
    s.chat_input.insert_str("  Good-Name  ");
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(handle_nickname_submission(&mut s, &swarm_cmd_tx));

    assert!(!s.is_editing_nickname());
    assert_eq!(s.own_nickname, "Good-Name");
    assert_eq!(
        p2p_app::nickname::get_self_nickname().unwrap().as_deref(),
        Some("Good-Name")
    );
    p2p_app::db::reset_db_url();
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

    assert!(
        state.is_adding_network_peer(),
        "a rejected address stays in the prompt so it can be corrected"
    );
    assert_eq!(
        state.chat_input.lines().join("\n"),
        "/ip4/1.2.3.4/tcp/4001",
        "the rejected text is preserved"
    );
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
    // Find Settings by content; it is a suffix tab but Shares sits after it.
    (0..state.dynamic_tabs.total_tab_count())
        .find(|&idx| {
            matches!(
                state.dynamic_tabs.tab_index_to_content(idx),
                TabContent::Settings
            )
        })
        .unwrap_or(0)
}

fn shares_tab_index(state: &AppState) -> usize {
    (0..state.dynamic_tabs.total_tab_count())
        .find(|&idx| {
            matches!(
                state.dynamic_tabs.tab_index_to_content(idx),
                TabContent::Shares
            )
        })
        .unwrap_or(0)
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

// ── Ctrl+C copy newest message to clipboard (Flutter parity) ─────────────

#[tokio::test]
async fn test_ctrl_c_copies_newest_message_in_chat() {
    let state = Arc::new(Mutex::new(reply_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    let s = state.lock().await;
    assert_eq!(
        s.pending_clipboard,
        vec!["12:00 [Alice] older line".to_string()]
    );
    // The modifier must keep `c` out of the composer.
    assert_eq!(s.chat_input.lines().join(""), "");
}

#[tokio::test]
async fn test_ctrl_c_without_messages_queues_nothing() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert_eq!(state.lock().await.pending_clipboard, Vec::<String>::new());
}

#[tokio::test]
async fn test_ctrl_c_is_inert_on_settings_tab() {
    let mut s = test_app_state();
    // No dynamic tabs: Chat=0, Contacts=1, Groups=2, Log=3, Settings=4.
    s.active_tab = 4;
    assert!(matches!(
        s.dynamic_tabs.tab_index_to_content(s.active_tab),
        TabContent::Settings
    ));
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert_eq!(state.lock().await.pending_clipboard, Vec::<String>::new());
}

#[tokio::test]
#[serial]
async fn test_ctrl_c_copies_all_logs_on_log_tab() {
    p2p_app::init_logging();
    // `test_app_state` provisions a DB identity, which itself logs two lines —
    // construct it first, then clear so the snapshot is deterministic.
    let mut s = test_app_state();
    p2p_app::clear_tui_logs();
    p2p_app::p2plog_info("first log line");
    p2p_app::p2plog_error("second log line");

    s.active_tab = 3; // Log (no dynamic tabs)
    assert!(matches!(
        s.dynamic_tabs.tab_index_to_content(s.active_tab),
        TabContent::Log
    ));
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    // The pure helper owns the exact join semantics; here we only need to see
    // that dispatch captured the whole buffer. `contains` keeps this robust
    // against unrelated tests appending to the global log buffer.
    let pending = &state.lock().await.pending_clipboard;
    assert_eq!(pending.len(), 1, "{pending:?}");
    assert!(pending[0].contains("first log line"), "{pending:?}");
    assert!(pending[0].contains("second log line"), "{pending:?}");
    assert!(
        pending[0].contains('\n'),
        "lines must be joined: {pending:?}"
    );
}

#[tokio::test]
#[serial]
async fn test_ctrl_c_with_empty_logs_queues_nothing() {
    p2p_app::init_logging();
    // As above: construct state before clearing, so provisioning logs cannot
    // race into the snapshot the handler reads.
    let mut s = test_app_state();
    p2p_app::clear_tui_logs();

    s.active_tab = 3; // Log
    let state = Arc::new(Mutex::new(s));
    let (swarm_cmd_tx, _) = mpsc::channel(4);
    let (render_tx, _) = mpsc::channel(4);

    let exited = process_key_event(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    assert!(!exited);
    assert_eq!(state.lock().await.pending_clipboard, Vec::<String>::new());
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
fn test_quick_reaction_digits_apply_the_rust_palette() {
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

        // Open the editor and use both ends of the palette without typing.
        for (digit, emoji) in [('1', "👍"), ('6', "😢")] {
            let _ = process_key_event(
                KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL),
                &state,
                &swarm_cmd_tx,
                &render_tx,
            )
            .await;
            assert!(state.lock().await.is_adding_reaction(), "editor opened");

            let _ = process_key_event(
                KeyEvent::new(KeyCode::Char(digit), KeyModifiers::empty()),
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
                    assert_eq!(signal.msg_id, "dm-0");
                    assert_eq!(signal.emoji, emoji);
                    assert_eq!(signal.actor, local_peer);
                    assert!(!signal.remove, "palette digit adds a new reaction");
                }
                _ => panic!("expected SendReaction"),
            }
            let s = state.lock().await;
            assert!(!s.is_adding_reaction(), "palette consumed the editor");
            assert_eq!(s.chat_input.lines().join(""), "", "digit was not typed");
        }

        assert!(
            p2p_app::reactions::has_reaction("dm-0", &local_peer, "👍").expect("has_reaction"),
            "first palette token persisted"
        );
        assert!(
            p2p_app::reactions::has_reaction("dm-0", &local_peer, "😢").expect("has_reaction"),
            "last palette token persisted"
        );
        let chips = state
            .lock()
            .await
            .reactions
            .get("dm-0")
            .expect("chips")
            .clone();
        assert_eq!(chips.len(), 2);
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_digit_outside_the_reaction_editor_types_normally() {
    let mut state = app_state_with_dm_messages("react-peer", 1);
    focus_dm_tab(&mut state);
    let state = Arc::new(Mutex::new(state));
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(4);
    let (render_tx, _render_rx) = mpsc::channel(4);

    let _ = process_key_event(
        KeyEvent::new(KeyCode::Char('1'), KeyModifiers::empty()),
        &state,
        &swarm_cmd_tx,
        &render_tx,
    )
    .await;

    let s = state.lock().await;
    assert_eq!(s.chat_input.lines().join(""), "1");
    assert!(
        !s.is_adding_reaction(),
        "plain digits never open the editor"
    );
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

// ── Realistic interactive sessions ──────────────────────────────────────
//
// Each test below drives one continuous arc through `KeySession`, which feeds
// `process_input_event` (the event loop's real entry point) and renders the
// resulting frame through the same renderer the terminal uses. These complement
// the focused key tests above by asserting that commands, DB rows, in-memory
// state and visible text still agree after a *sequence* of user actions.

const DM_SESSION_PEER: &str = "session-peer";
const DM_SESSION_ORIGINAL_ID: &str = "dm-original";

fn seed_dm_session_db() -> (TempDir, String) {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");
    let local_peer = p2p_app::peers::local_peer_id_string().expect("local peer id");
    p2p_app::messages::save_message_with_meta(
        "hello from Alice",
        Some(DM_SESSION_PEER),
        "test-net",
        true,
        Some(&local_peer),
        p2p_app::messages::MessageMeta {
            msg_id: Some(DM_SESSION_ORIGINAL_ID.to_string()),
            ..Default::default()
        },
    )
    .expect("seed incoming DM");
    (dir, local_peer)
}

fn dm_session_state() -> AppState {
    let mut state = app_state_with_dm_messages(DM_SESSION_PEER, 0);
    state.dm_messages.insert(
        DM_SESSION_PEER.to_string(),
        std::collections::VecDeque::from(vec!["12:00 [Alice] hello from Alice".to_string()]),
    );
    state.dm_message_ids.insert(
        DM_SESSION_PEER.to_string(),
        std::collections::VecDeque::from(vec![Some(DM_SESSION_ORIGINAL_ID.to_string())]),
    );
    state.dm_message_db_ids.insert(
        DM_SESSION_PEER.to_string(),
        std::collections::VecDeque::from(vec![None]),
    );
    focus_dm_tab(&mut state);
    state
}

/// Ctrl+R → type → Enter: assert the captured quote, the outbound command, the
/// persisted row and the rendered transcript before later actions build on it.
async fn session_reply_and_send(session: &mut KeySession) -> (i32, String) {
    assert!(!session.press_ctrl_char('r').await);
    let tab = TabContent::Direct(DM_SESSION_PEER.to_string());
    let reply = {
        let s = session.state.lock().await;
        s.reply_draft(&tab).cloned().expect("reply draft")
    };
    assert_eq!(reply.msg_id, DM_SESSION_ORIGINAL_ID);
    assert_eq!(reply.sender, "Alice");
    assert_eq!(reply.preview, "hello from Alice");
    let reply_frame = session.frame_text().await;
    assert!(
        reply_frame.contains("Alice: hello from Alice"),
        "reply preview missing: {reply_frame}"
    );

    assert!(!session.type_text("reply body").await);
    assert!(!session.press_code(KeyCode::Enter).await);
    let commands = session.drain_commands();
    let (content, sent_reply, peer_id) = commands
        .iter()
        .find_map(|cmd| {
            if let SwarmCommand::SendDm {
                content,
                reply_to: Some(reply),
                peer_id,
                ..
            } = cmd
            {
                Some((content.clone(), reply.clone(), peer_id.clone()))
            } else {
                None
            }
        })
        .expect("SendDm carrying the reply snapshot");
    assert_eq!(content, "reply body");
    assert_eq!(peer_id, DM_SESSION_PEER);
    assert_eq!(sent_reply.msg_id, DM_SESSION_ORIGINAL_ID);
    assert_eq!(sent_reply.sender, "Alice");
    assert!(commands.iter().any(|cmd| matches!(
        cmd,
        SwarmCommand::SendTyping {
            is_typing: false,
            ..
        }
    )));

    let (db_id, sent_msg_id, line) = {
        let s = session.state.lock().await;
        let msgs = s.dm_messages.get(DM_SESSION_PEER).expect("dm transcript");
        let db_id = s
            .dm_message_db_ids
            .get(DM_SESSION_PEER)
            .and_then(std::collections::VecDeque::back)
            .and_then(|slot| *slot)
            .expect("sent row id");
        let sent_msg_id = s
            .dm_message_ids
            .get(DM_SESSION_PEER)
            .and_then(std::collections::VecDeque::back)
            .and_then(std::clone::Clone::clone)
            .expect("sent wire id");
        (db_id, sent_msg_id, msgs.back().cloned().expect("sent line"))
    };
    assert!(line.contains("↳ Alice: hello from Alice"));
    assert!(line.contains("reply body"));
    let reply_consumed = {
        let s = session.state.lock().await;
        s.reply_draft(&tab).is_none()
    };
    assert!(reply_consumed, "sending consumes the reply draft");

    let rows = p2p_app::messages::load_direct_messages(DM_SESSION_PEER, 20).expect("load DMs");
    let row = rows.iter().find(|row| row.id == db_id).expect("sent row");
    assert_eq!(row.content, "reply body");
    let stored = row
        .reply_to_json
        .as_deref()
        .and_then(p2p_app::reply::decode)
        .expect("stored reply snapshot");
    assert_eq!(stored.msg_id, DM_SESSION_ORIGINAL_ID);
    (db_id, sent_msg_id)
}

/// Ctrl+Y → stale popup → quick `1`: the digit must dismiss the old hint *and*
/// commit the canonical palette token, exactly as a correcting keystroke does.
async fn session_quick_reaction(session: &mut KeySession, msg_id: &str, local_peer: &str) {
    assert!(!session.press_ctrl_char('y').await);
    {
        let mut s = session.state.lock().await;
        s.popup = Some("stale validation hint".to_string());
    }
    assert!(!session.press_char('1').await);

    let token = *p2p_app::reactions::QUICK_REACTIONS
        .first()
        .expect("quick palette is not empty");
    let (peer_ids, signal) = session
        .drain_commands()
        .into_iter()
        .find_map(|cmd| match cmd {
            SwarmCommand::SendReaction { peer_ids, signal } => Some((peer_ids, *signal)),
            _ => None,
        })
        .expect("SendReaction");
    assert_eq!(peer_ids, vec![DM_SESSION_PEER.to_string()]);
    assert_eq!(signal.msg_id, msg_id);
    assert_eq!(signal.emoji, token);
    assert_eq!(signal.actor, local_peer);
    assert!(!signal.remove, "first reaction is an add");

    let s = session.state.lock().await;
    assert!(s.popup.is_none(), "successful commit clears stale hints");
    assert!(!s.is_adding_reaction());
    assert!(
        p2p_app::reactions::has_reaction(msg_id, local_peer, token).expect("has_reaction"),
        "palette token persisted"
    );
    let chips = s.reactions.get(msg_id).expect("chips refreshed");
    assert_eq!(chips.len(), 1);
    assert_eq!(chips[0].emoji, token);
    assert!(chips[0].mine);
    drop(s);
    let frame = session.frame_text().await;
    assert!(
        frame.contains(token),
        "frame did not show reaction: {frame}"
    );
}

/// Ctrl+E → clear → invalid empty commit → corrected text → Enter → Delete → y.
/// The correction key crosses the popup boundary; the successful edit clears it.
async fn session_edit_then_delete(session: &mut KeySession, db_id: i32) {
    assert!(!session.press_ctrl_char('e').await);
    assert!(!session.press_ctrl_char('k').await);
    assert!(!session.press_code(KeyCode::Enter).await);
    {
        let s = session.state.lock().await;
        assert!(s.is_editing_message());
        assert_eq!(s.popup.as_deref(), Some("Message body cannot be empty"));
    }
    assert!(!session.type_text("edited reply body").await);
    assert!(!session.press_code(KeyCode::Enter).await);
    {
        let s = session.state.lock().await;
        assert!(!s.is_editing_message());
        assert!(
            s.popup.is_none(),
            "successful edit clears the validation hint"
        );
        let line = s
            .dm_messages
            .get(DM_SESSION_PEER)
            .and_then(std::collections::VecDeque::back)
            .expect("edited line");
        assert!(line.contains("edited reply body"));
        assert!(line.contains("(edited)"));
    }
    assert_eq!(
        p2p_app::messages::message_body(db_id).expect("read edited body"),
        Some("edited reply body".to_string())
    );
    assert!(session.frame_text().await.contains("edited reply body"));

    assert!(!session.press_code(KeyCode::Delete).await);
    {
        let s = session.state.lock().await;
        assert!(s.delete_message_prompt.is_some(), "delete prompt armed");
    }
    assert!(!session.press_char('y').await);
    {
        let s = session.state.lock().await;
        assert!(s.delete_message_prompt.is_none());
        assert_eq!(
            p2p_app::messages::message_body(db_id).expect("read tombstone"),
            None,
            "local deletion is a tombstone"
        );
        let msgs = s
            .dm_messages
            .get(DM_SESSION_PEER)
            .expect("transcript after deletion");
        assert_eq!(msgs.len(), 1, "only the incoming row remains");
        assert!(!msgs.iter().any(|line| line.contains("edited reply body")));
    }
    assert!(!session.frame_text().await.contains("edited reply body"));
}

#[test]
#[serial]
fn test_dm_session_reply_reaction_edit_delete_through_real_keys() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_dir, local_peer) = seed_dm_session_db();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut session = KeySession::new(dm_session_state());
        let (db_id, sent_msg_id) = session_reply_and_send(&mut session).await;
        session_quick_reaction(&mut session, &sent_msg_id, &local_peer).await;
        session_edit_then_delete(&mut session, db_id).await;
        assert!(session.drain_commands().is_empty());
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

fn seed_contacts_db() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    p2p_app::identity::provision_my_user(None, None).expect("provision");
    let no_addrs: [String; 0] = [];
    p2p_app::peers::save_peer("peer0", &no_addrs).expect("save peer0");
    p2p_app::peers::save_peer("peer1", &no_addrs).expect("save peer1");
    dir
}

/// `/` → type `peer1` → Enter → `i` → `n` → invalid nickname → Backspace × 9 →
/// valid nickname → Enter. The filter survives navigation and editing; only the
/// name changes in state, the database and the rendered Peer Info pane.
#[test]
#[serial]
fn test_contacts_search_peer_info_local_nickname_session() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = seed_contacts_db();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = app_state_with_peers(2);
        state.active_tab = 1; // Contacts
        let mut session = KeySession::new(state);

        assert!(!session.press_char('/').await);
        assert!(!session.type_text("peer1").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        let s = session.state.lock().await;
        assert_eq!(s.contact_search, "peer1");
        assert_eq!(s.contacts.len(), 1);
        assert_eq!(s.contacts_all.len(), 2);
        assert_eq!(s.contacts[0].contact_id, "peer1");
        drop(s);
        let peer1_label = p2p_app::get_peer_display_name_or_short("peer1");
        let filtered_frame = session.frame_text().await;
        assert!(
            filtered_frame.contains(&peer1_label),
            "filtered frame missing {peer1_label}: {filtered_frame}"
        );

        assert!(!session.press_char('i').await);
        let peer_info_tab = {
            let s = session.state.lock().await;
            s.dynamic_tabs.tab_index_to_content(s.active_tab)
        };
        assert!(
            matches!(&peer_info_tab, TabContent::PeerInfo(id) if id == "peer1"),
            "unexpected tab: {peer_info_tab:?}"
        );
        assert!(!session.press_char('n').await);
        assert!(!session.type_text("bad name!").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(s.is_editing_local_nickname());
            assert_eq!(
                s.popup.as_deref(),
                Some("Local nickname: letters, numbers or dashes (max 20 characters)")
            );
        }

        // The first Backspace dismisses the validation hint and still erases a
        // character; the remaining eight clear the editor for the correction.
        for _ in 0.."bad name!".chars().count() {
            assert!(!session.press_code(KeyCode::Backspace).await);
        }
        assert!(!session.type_text("Alice-1").await);
        assert!(!session.press_code(KeyCode::Enter).await);

        let s = session.state.lock().await;
        assert!(
            s.popup.is_none(),
            "successful commit clears the validation hint"
        );
        assert!(!s.is_editing_nickname());
        assert_eq!(
            s.local_nicknames.get("peer1").map(String::as_str),
            Some("Alice-1")
        );
        assert_eq!(
            s.contact_search, "peer1",
            "filter survives Peer Info editing"
        );
        assert_eq!(s.contacts.len(), 1);
        assert!(
            matches!(
                s.dynamic_tabs.tab_index_to_content(s.active_tab),
                TabContent::PeerInfo(ref id) if id == "peer1"
            ),
            "still on the same Peer Info tab"
        );
        drop(s);

        let peers = p2p_app::peers::load_peers().expect("load peers");
        let peer = peers
            .iter()
            .find(|peer| peer.peer_id == "peer1")
            .expect("peer1 row");
        assert_eq!(peer.peer_local_nickname.as_deref(), Some("Alice-1"));
        assert!(session.frame_text().await.contains("Alice-1"));
        assert!(session.drain_commands().is_empty());
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── realistic Groups and Settings key sessions ─────────────────────────────

fn seed_group_session_db() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("group-session.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    dir
}

/// `g` → prompt owns navigation → invalid empty name → corrected name → Enter
/// → group message → `BackTab` to the list → `d`/`y` leave. Every step crosses the
/// real dispatch path and is checked against the DB and queued commands.
#[test]
#[serial]
fn test_group_create_message_leave_session_through_real_keys() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = seed_group_session_db();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = test_app_state();
        state.active_tab = p2p_app::tui_tabs::FIXED_TAB_COUNT - 1; // Groups
        let mut session = KeySession::new(state);

        assert!(!session.press_char('g').await);
        {
            let s = session.state.lock().await;
            assert!(s.is_creating_group());
            assert!(matches!(
                s.dynamic_tabs.tab_index_to_content(s.active_tab),
                TabContent::Groups
            ));
        }
        let prompt_frame = session.frame_text().await;
        assert!(
            prompt_frame.contains("Create or Join Group"),
            "group prompt title missing: {prompt_frame}"
        );

        // A modal prompt owns navigation; cycling tabs here would strand it on
        // a tab with no visible input box. Ctrl+P must not open Peer Info.
        assert!(!session.press_code(KeyCode::Tab).await);
        assert!(!session.press_code(KeyCode::Up).await);
        assert!(!session.press_ctrl_char('p').await);
        {
            let s = session.state.lock().await;
            assert!(s.is_creating_group());
            assert_eq!(s.active_tab, p2p_app::tui_tabs::FIXED_TAB_COUNT - 1);
            assert_eq!(s.dynamic_tabs.peer_info_tab_count(), 0);
        }

        // Group names are validated in Rust: an all-whitespace draft must stay
        // in the prompt with a hint, and the first correction key both clears
        // the hint and edits the text.
        assert!(!session.type_text("   ").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(s.is_creating_group());
            assert!(
                s.popup
                    .as_deref()
                    .is_some_and(|hint| hint.contains("group name must not be empty"))
            );
        }
        for _ in 0..3 {
            assert!(!session.press_code(KeyCode::Backspace).await);
        }
        assert!(!session.type_text("Team Rocket").await);
        assert!(!session.press_code(KeyCode::Enter).await);

        let groups = p2p_app::groups::list_groups().expect("list groups");
        let group = groups
            .iter()
            .find(|group| group.display_name == "Team Rocket")
            .expect("created group row");
        let group_id = group.group_id.clone();
        let commands = session.drain_commands();
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            SwarmCommand::SubscribeGroup { group_id: id } if id == &group_id
        )));
        let s = session.state.lock().await;
        assert!(!s.is_creating_group());
        assert!(s.popup.is_none());
        assert!(matches!(
            s.dynamic_tabs.tab_index_to_content(s.active_tab),
            TabContent::GroupChat(ref id) if id == &group_id
        ));
        drop(s);
        let created_frame = session.frame_text().await;
        assert!(
            created_frame.contains("Team Rocket"),
            "new group tab missing from frame: {created_frame}"
        );

        assert!(!session.type_text("hello group").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        let commands = session.drain_commands();
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            SwarmCommand::PublishGroup { group_id: id, content, .. }
                if id == &group_id && content == "hello group"
        )));
        let rows = p2p_app::groups::load_group_messages(&group_id, 20).expect("load messages");
        assert!(
            rows.iter().any(|row| row.content == "hello group"),
            "group message persisted"
        );
        assert!(
            session.frame_text().await.contains("hello group"),
            "group message rendered"
        );

        // Leave is armed from the group list, not the chat composer.
        assert!(!session.press_code(KeyCode::BackTab).await);
        assert!(!session.press_char('d').await);
        {
            let s = session.state.lock().await;
            assert!(s.group_leave_prompt.is_some(), "leave prompt armed");
        }
        assert!(!session.press_char('y').await);
        {
            let s = session.state.lock().await;
            assert!(s.group_leave_prompt.is_none());
            assert_eq!(
                s.dynamic_tabs.group_tab_count(),
                0,
                "the left group's tab is closed"
            );
            assert!(matches!(
                s.dynamic_tabs.tab_index_to_content(s.active_tab),
                TabContent::Groups
            ));
        }
        assert!(
            p2p_app::groups::find_group(&group_id)
                .expect("lookup")
                .is_none(),
            "leave deletes the local group row"
        );
        let commands = session.drain_commands();
        assert_eq!(
            commands.len(),
            1,
            "leave should queue only the unsubscribe command: {commands:?}"
        );
        assert!(matches!(
            &commands[0],
            SwarmCommand::UnsubscribeGroup { group_id: id } if id == &group_id
        ));
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

fn seed_settings_session_db() -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("settings-session.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");
    dir
}

/// `d` → multiaddr → Enter dials; `b` → invalid address → correction → Enter
/// persists; a second `b` adds another row and `Up`/`x` removes the highlighted
/// one. Prompts keep the keyboard and rejected text stays editable.
#[test]
#[serial]
fn test_settings_dial_and_network_peer_session_through_real_keys() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = seed_settings_session_db();
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let mut state = test_app_state();
        state.active_tab = settings_tab_index(&state);
        let mut session = KeySession::new(state);

        assert!(!session.press_char('d').await);
        {
            let s = session.state.lock().await;
            assert!(s.is_dialing_peer());
        }
        assert!(
            session.frame_text().await.contains("Dial peer address"),
            "dial prompt title missing"
        );
        assert!(!session.press_code(KeyCode::Tab).await);
        assert!(!session.press_code(KeyCode::Down).await);
        {
            let s = session.state.lock().await;
            assert!(s.is_dialing_peer());
            assert_eq!(s.active_tab, settings_tab_index(&s));
        }
        assert!(!session.type_text("/ip4/127.0.0.1/tcp/1234").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        let commands = session.drain_commands();
        assert!(commands.iter().any(|cmd| matches!(
            cmd,
            SwarmCommand::Dial { address } if address == "/ip4/127.0.0.1/tcp/1234"
        )));
        {
            let s = session.state.lock().await;
            assert!(!s.is_dialing_peer());
        }

        assert!(!session.press_char('b').await);
        assert!(
            session
                .frame_text()
                .await
                .contains("Add bootstrap multiaddr"),
            "network peer prompt title missing"
        );
        assert!(!session.type_text("not-an-address").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(
                s.is_adding_network_peer(),
                "invalid address keeps the prompt open"
            );
            assert_eq!(s.chat_input.lines().join("\n"), "not-an-address");
            assert!(
                s.popup
                    .as_deref()
                    .is_some_and(|hint| hint.starts_with("Network peer not saved:"))
            );
        }
        assert!(!session.press_code(KeyCode::Esc).await);
        {
            let s = session.state.lock().await;
            assert!(s.popup.is_none(), "first Esc dismisses only the popup");
            assert!(
                s.is_adding_network_peer(),
                "the prompt survives the validation popup"
            );
            assert_eq!(s.chat_input.lines().join("\n"), "not-an-address");
        }
        for _ in 0.."not-an-address".chars().count() {
            assert!(!session.press_code(KeyCode::Backspace).await);
        }
        assert!(!session.type_text("/ip4/1.2.3.4/tcp/4001").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(!s.is_adding_network_peer());
            assert!(s.popup.is_none());
            assert_eq!(s.network_peers.len(), 1);
            assert_eq!(s.network_peer_selection, 0);
        }

        assert!(!session.press_char('b').await);
        assert!(!session.type_text("/ip4/5.6.7.8/tcp/4001").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.network_peers.len(), 2);
            assert_eq!(s.network_peer_selection, 1);
        }
        assert!(!session.press_code(KeyCode::Up).await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.network_peer_selection, 0, "Up moves the highlight");
        }
        assert!(!session.press_char('x').await);
        let stored = p2p_app::network_config::list_network_peers().expect("list peers");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].multiaddr, "/ip4/5.6.7.8/tcp/4001");
        let frame = session.frame_text().await;
        assert!(frame.contains("/ip4/5.6.7.8/tcp/4001"));
        assert!(!frame.contains("/ip4/1.2.3.4/tcp/4001"));
        assert!(session.drain_commands().is_empty());
    });
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── real-key message selection sessions ────────────────────────────────────

/// DM transcript with aligned wire/DB ids so every targeted verb has a stable
/// anchor. No rows are seeded: the edit/delete helpers only arm their prompts
/// here, and fall back to the rendered line when no DB body exists.
fn selection_dm_state() -> AppState {
    let mut state = app_state_with_dm_messages("cursor-peer", 0);
    let peer = "cursor-peer";
    state.dm_messages.insert(
        peer.to_string(),
        std::collections::VecDeque::from(vec![
            "12:00 [Alice] oldest body".to_string(),
            "12:01 [Me] middle body".to_string(),
            "12:02 [Me] newest body".to_string(),
        ]),
    );
    state.dm_message_ids.insert(
        peer.to_string(),
        std::collections::VecDeque::from(vec![
            Some("oldest-wire".to_string()),
            Some("middle-wire".to_string()),
            Some("newest-wire".to_string()),
        ]),
    );
    state.dm_message_db_ids.insert(
        peer.to_string(),
        std::collections::VecDeque::from(vec![None, Some(11), Some(13)]),
    );
    state.dm_scroll_state.insert(peer.to_string(), (0, true));
    state.chat_area_height = 8;
    focus_dm_tab(&mut state);
    state
}

#[tokio::test]
async fn test_message_cursor_moves_clamps_and_highlights_through_real_keys() {
    let mut session = KeySession::new(selection_dm_state());
    let dm = TabContent::Direct("cursor-peer".to_string());

    // With no cursor the newest row is only the *fallback target*, never a
    // highlighted selection.
    {
        let s = session.state.lock().await;
        assert_eq!(s.message_selection_index(&dm), None);
        assert_eq!(
            s.target_message_line(&dm).as_deref(),
            Some("12:02 [Me] newest body")
        );
    }

    // Bare arrows scroll the hovered section but never arm a message.
    assert!(!session.press_code(KeyCode::Up).await);
    {
        let s = session.state.lock().await;
        assert_eq!(s.message_selection_index(&dm), None);
    }

    let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
    assert!(!session.press_key(ctrl_up).await);
    assert_eq!(
        session.state.lock().await.message_selection_index(&dm),
        Some(1)
    );
    assert!(!session.press_key(ctrl_up).await);
    assert_eq!(
        session.state.lock().await.message_selection_index(&dm),
        Some(0)
    );
    // Clamped at the oldest row: no wrap to the newest one.
    assert!(!session.press_key(ctrl_up).await);
    assert_eq!(
        session.state.lock().await.message_selection_index(&dm),
        Some(0)
    );

    let highlighted = session.frame_highlighted_lines().await;
    assert!(
        highlighted.iter().any(|line| line.contains("oldest body")),
        "the cursor row is not highlighted: {highlighted:?}"
    );
    assert!(
        highlighted.iter().all(|line| !line.contains("middle body")),
        "an unselected row inherited the highlight: {highlighted:?}"
    );

    let ctrl_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
    for _ in 0..5 {
        assert!(!session.press_key(ctrl_down).await);
    }
    assert_eq!(
        session.state.lock().await.message_selection_index(&dm),
        Some(2)
    );
    let highlighted = session.frame_highlighted_lines().await;
    assert!(
        highlighted.iter().any(|line| line.contains("newest body")),
        "the clamped newest row is not highlighted: {highlighted:?}"
    );
    assert!(session.drain_commands().is_empty());
}

#[test]
#[serial]
fn test_targeted_verbs_use_the_selected_row_through_real_keys() {
    with_local_delete_db(|| {
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        rt.block_on(async {
            let mut session = KeySession::new(selection_dm_state());
            let dm = TabContent::Direct("cursor-peer".to_string());

            // Arm the middle (DB-backed) row. Every verb below must target it,
            // not the newest row that the pre-selection fallback would pick.
            assert!(
                !session
                    .press_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL))
                    .await
            );
            assert_eq!(
                session.state.lock().await.message_selection_index(&dm),
                Some(1)
            );

            assert!(!session.press_ctrl_char('c').await);
            {
                let s = session.state.lock().await;
                assert_eq!(
                    s.pending_clipboard,
                    vec!["12:01 [Me] middle body".to_string()]
                );
            }

            assert!(!session.press_ctrl_char('r').await);
            {
                let s = session.state.lock().await;
                let reply = s.reply_draft(&dm).expect("reply draft");
                assert_eq!(reply.msg_id, "middle-wire");
            }
            assert!(!session.press_code(KeyCode::Esc).await);
            {
                let s = session.state.lock().await;
                assert!(s.reply_draft(&dm).is_none());
                assert_eq!(s.message_selection_index(&dm), Some(1));
            }

            assert!(!session.press_ctrl_char('y').await);
            {
                let s = session.state.lock().await;
                assert_eq!(
                    s.adding_reaction.as_ref().map(|p| p.msg_id.as_str()),
                    Some("middle-wire")
                );
            }
            assert!(!session.press_code(KeyCode::Esc).await);

            assert!(!session.press_ctrl_char('e').await);
            {
                let s = session.state.lock().await;
                assert_eq!(
                    s.editing_message.as_ref().map(|p| p.message_id),
                    Some(11),
                    "the selected sent row is edited"
                );
            }
            assert!(!session.press_code(KeyCode::Esc).await);

            assert!(!session.press_code(KeyCode::Delete).await);
            {
                let s = session.state.lock().await;
                assert_eq!(
                    s.delete_message_prompt.as_ref().map(|p| p.message_id),
                    Some(11),
                    "the selected sent row is deleted"
                );
            }
            assert!(!session.press_code(KeyCode::Esc).await);

            assert!(!session.press_ctrl_char('o').await);
            {
                let s = session.state.lock().await;
                assert_eq!(
                    s.popup.as_deref(),
                    Some("No downloaded attachment in this conversation")
                );
            }
            assert!(!session.press_code(KeyCode::Esc).await);

            {
                let s = session.state.lock().await;
                assert_eq!(s.message_selection_index(&dm), Some(1));
                assert!(s.popup.is_none());
            }
            assert!(session.drain_commands().is_empty());
        });
    });
}

#[tokio::test]
async fn test_esc_unwinds_only_the_active_conversation_one_layer_at_a_time() {
    let mut state = selection_dm_state();
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "12:03 [Alice] broadcast body".to_string(),
        sender_peer_id: Some("alice".to_string()),
    });
    state
        .message_ids
        .push_back(Some("broadcast-wire".to_string()));
    state.message_db_ids.push_back(None);
    let chat = TabContent::Chat;
    let dm = TabContent::Direct("cursor-peer".to_string());
    assert!(state.begin_reply(&chat).is_some());
    assert!(state.begin_reply(&dm).is_some());
    assert!(state.select_message(&dm, 0));
    let mut session = KeySession::new(state);

    assert!(!session.press_code(KeyCode::Esc).await);
    {
        let s = session.state.lock().await;
        assert!(s.reply_draft(&chat).is_some(), "other draft survives");
        assert!(s.reply_draft(&dm).is_none());
        assert_eq!(s.message_selection_index(&dm), Some(0));
    }
    assert!(!session.press_code(KeyCode::Esc).await);
    {
        let s = session.state.lock().await;
        assert!(s.reply_draft(&chat).is_some());
        assert_eq!(s.message_selection_index(&dm), None);
    }
    assert!(!session.press_code(KeyCode::Esc).await);
    {
        let s = session.state.lock().await;
        assert!(matches!(
            s.dynamic_tabs.tab_index_to_content(s.active_tab),
            TabContent::Chat
        ));
        assert!(s.reply_draft(&chat).is_some());
    }
    assert!(!session.press_code(KeyCode::Esc).await);
    assert!(session.state.lock().await.reply_draft(&chat).is_none());
    assert!(session.drain_commands().is_empty());
}

#[tokio::test]
async fn test_stale_rendered_selection_still_consumes_one_esc_layer() {
    let mut state = selection_dm_state();
    let dm = TabContent::Direct("cursor-peer".to_string());
    state.message_selection.insert(
        "dm:cursor-peer".to_string(),
        crate::tui::state::MessageSelection::Rendered {
            text: "12:00 [Alice] oldest body".to_string(),
            occurrence: 0,
            loaded_len: 99,
        },
    );
    assert_eq!(state.message_selection_index(&dm), None);
    let active = state.active_tab;
    let mut session = KeySession::new(state);

    assert!(!session.press_code(KeyCode::Esc).await);
    let s = session.state.lock().await;
    assert!(
        !s.message_selection.contains_key("dm:cursor-peer"),
        "the stale anchor is dropped"
    );
    assert_eq!(
        s.active_tab, active,
        "Esc stops here instead of leaving the tab"
    );
    assert_eq!(s.message_selection_index(&dm), None);
}

#[tokio::test]
async fn test_group_message_cursor_highlights_row_through_real_keys() {
    let mut state = test_app_state();
    let group_id = "g-selection";
    let group_tab = state
        .dynamic_tabs
        .add_group_tab(group_id.to_string(), "Group".to_string());
    state.active_tab = group_tab;
    state.group_messages.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![
            "12:00 [Alice] group oldest".to_string(),
            "12:01 [Me] group middle".to_string(),
            "12:02 [Me] group newest".to_string(),
        ]),
    );
    state.group_message_ids.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![
            Some("group-oldest".to_string()),
            Some("group-middle".to_string()),
            Some("group-newest".to_string()),
        ]),
    );
    state.group_message_db_ids.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![None, None, None]),
    );
    state
        .group_scroll_state
        .insert(group_id.to_string(), (0, true));
    state.chat_area_height = 8;
    let mut session = KeySession::new(state);
    let group = TabContent::GroupChat(group_id.to_string());

    assert!(
        !session
            .press_key(KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL))
            .await
    );
    assert_eq!(
        session.state.lock().await.message_selection_index(&group),
        Some(1)
    );
    let highlighted = session.frame_highlighted_lines().await;
    assert!(
        highlighted.iter().any(|line| line.contains("group middle")),
        "the group cursor row is not highlighted: {highlighted:?}"
    );
    assert!(
        highlighted
            .iter()
            .all(|line| !line.contains("group newest")),
        "an unselected group row inherited the highlight: {highlighted:?}"
    );
    assert!(session.drain_commands().is_empty());
}

// ── private-share grant editor (Flutter parity) ──────────────────────────

/// Set up one share in an isolated DB and land the real app on its Shares tab.
/// The returned `TempDir` must outlive every DB call in the test.
fn share_db_state(
    owner: &str,
    kind: p2p_app::shares::ShareKind,
    public_write: bool,
    grants: &[(&str, p2p_app::shares::ShareMode)],
) -> (TempDir, AppState, String) {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let share = p2p_app::shares::create_share("Docs", owner, kind, public_write, None)
        .expect("create share");
    for &(user_id, mode) in grants {
        p2p_app::shares::set_grant(&share.share_id, user_id, mode).expect("seed grant");
    }

    let mut state = test_app_state();
    state.active_tab = shares_tab_index(&state);
    state.own_user_id = Some("owner".to_string());
    state.reload_shares();
    (dir, state, share.share_id)
}

fn share_grant_db_state(
    kind: p2p_app::shares::ShareKind,
    grants: &[(&str, p2p_app::shares::ShareMode)],
) -> (TempDir, AppState, String) {
    share_db_state("owner", kind, false, grants)
}

/// One canonical manifest with two files, persisted through the share store and
/// reloaded into the app state. The returned `TempDir` must outlive the DB use.
fn share_manifest_db_state() -> (TempDir, AppState, String) {
    let (dir, mut state, share_id) =
        share_db_state("owner", p2p_app::shares::ShareKind::Public, false, &[]);
    let manifest = p2p_app::shares::ShareManifest {
        share_id: share_id.clone(),
        name: "Docs".to_string(),
        revision: 1,
        entries: vec![
            p2p_app::shares::ShareEntry {
                path: "docs/a.md".to_string(),
                file_id: p2p_app::files::hash_bytes(b"a"),
                size: 1,
                mime: Some("text/plain".to_string()),
            },
            p2p_app::shares::ShareEntry {
                path: "docs/b.md".to_string(),
                file_id: p2p_app::files::hash_bytes(b"b"),
                size: 1,
                mime: Some("text/plain".to_string()),
            },
        ],
    };
    p2p_app::shares::save_manifest(&share_id, &manifest).expect("seed manifest");
    state.reload_shares();
    (dir, state, share_id)
}

/// An isolated DB with a provisioned owner and no shares, landed on the Shares
/// tab. The returned `TempDir` must outlive every DB call in the test.
fn empty_share_db_state() -> (TempDir, AppState) {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let mut state = test_app_state();
    state.active_tab = shares_tab_index(&state);
    state.own_user_id = Some("owner".to_string());
    state.reload_shares();
    (dir, state)
}

fn share_kind_and_write(share_id: &str) -> (p2p_app::shares::ShareKind, bool) {
    let share = p2p_app::shares::get_share(share_id)
        .expect("get share")
        .expect("present");
    (share.kind, share.public_write)
}

fn close_share_grant_db() {
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

fn grant_mode(share_id: &str, user_id: &str) -> Option<p2p_app::shares::ShareMode> {
    p2p_app::shares::list_grants(share_id)
        .expect("list grants")
        .into_iter()
        .find(|grant| grant.user_id == user_id)
        .map(|grant| grant.mode)
}

#[test]
#[serial]
fn test_share_grant_add_through_real_keys_persists_and_reloads() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) =
            share_grant_db_state(p2p_app::shares::ShareKind::Private, &[]);
        let mut session = KeySession::new(state);

        assert!(!session.press_char('a').await);
        {
            let s = session.state.lock().await;
            assert!(s.is_editing_share_grant());
            assert_eq!(s.chat_input.lines().join("\n"), "");
            assert!(matches!(
                s.share_grant_prompt.as_ref().map(|prompt| &prompt.action),
                Some(crate::tui::state::ShareGrantAction::Add {
                    mode: p2p_app::shares::ShareMode::ReadOnly,
                })
            ));
        }
        assert!(
            session
                .frame_text()
                .await
                .contains("Add share grant (readonly)"),
            "the prompt title must show the default canonical mode"
        );

        assert!(!session.type_text("alice").await);
        assert!(!session.press_code(KeyCode::Tab).await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.chat_input.lines().join("\n"), "alice");
            assert!(matches!(
                s.share_grant_prompt.as_ref().map(|prompt| &prompt.action),
                Some(crate::tui::state::ShareGrantAction::Add {
                    mode: p2p_app::shares::ShareMode::ReadWrite,
                })
            ));
        }
        assert!(
            session
                .frame_text()
                .await
                .contains("Add share grant (readwrite)"),
            "Tab must toggle the mode in place"
        );

        assert!(!session.press_code(KeyCode::Enter).await);
        assert_eq!(
            grant_mode(&share_id, "alice"),
            Some(p2p_app::shares::ShareMode::ReadWrite)
        );
        {
            let s = session.state.lock().await;
            assert!(!s.is_editing_share_grant());
            assert!(s.share_grant_prompt.is_none());
            assert_eq!(s.chat_input.lines().join("\n"), "");
            assert_eq!(s.share_event_notice.as_deref(), Some("Grant set for alice"));
        }
        let frame = session.frame_text().await;
        assert!(frame.contains("alice (readwrite)"), "frame: {frame}");
        assert!(frame.contains("Grant set for alice"), "frame: {frame}");
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_grant_revoke_uses_highlighted_grant_not_typed_text() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) = share_grant_db_state(
            p2p_app::shares::ShareKind::Private,
            &[
                ("alice", p2p_app::shares::ShareMode::ReadOnly),
                ("bob", p2p_app::shares::ShareMode::ReadWrite),
            ],
        );
        let mut session = KeySession::new(state);

        let ctrl_down = KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL);
        let ctrl_up = KeyEvent::new(KeyCode::Up, KeyModifiers::CONTROL);
        assert!(!session.press_key(ctrl_up).await);
        assert_eq!(session.state.lock().await.share_grant_selection, 0);
        assert!(!session.press_key(ctrl_down).await);
        assert!(!session.press_key(ctrl_down).await);
        let (target, other) = {
            let s = session.state.lock().await;
            assert_eq!(s.share_grant_selection, 1);
            let grants = s.selected_share_grants();
            let target = grants[1].user_id.clone();
            let other = grants[0].user_id.clone();
            (target, other)
        };
        assert!(
            session.frame_text().await.contains(&format!("> {target}")),
            "the grant cursor must be visible in the detail pane"
        );

        assert!(!session.press_char('x').await);
        {
            let s = session.state.lock().await;
            assert!(matches!(
                s.share_grant_prompt.as_ref().map(|prompt| &prompt.action),
                Some(crate::tui::state::ShareGrantAction::Revoke { user_id })
                    if user_id == &target
            ));
            assert_eq!(s.chat_input.lines().join("\n"), target);
        }

        // The prefilled id is a confirmation, not an editable target: even if a
        // stray key lands, the stored action still owns the DB write.
        assert!(!session.type_text("mallory").await);
        assert_eq!(
            session.state.lock().await.chat_input.lines().join("\n"),
            target
        );

        assert!(!session.press_code(KeyCode::Enter).await);
        assert_eq!(grant_mode(&share_id, &target), None);
        assert!(
            grant_mode(&share_id, &other).is_some(),
            "the other grant must survive"
        );
        let expected_notice = format!("Grant revoked for {target}");
        {
            let s = session.state.lock().await;
            assert!(!s.is_editing_share_grant());
            assert_eq!(s.share_grant_selection, 0, "cursor clamps after reload");
            assert_eq!(
                s.share_event_notice.as_deref(),
                Some(expected_notice.as_str())
            );
        }
        let frame = session.frame_text().await;
        assert!(frame.contains(&expected_notice), "frame: {frame}");
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_grant_add_rejects_empty_user_id_without_writing() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) =
            share_grant_db_state(p2p_app::shares::ShareKind::Private, &[]);
        let mut session = KeySession::new(state);

        assert!(!session.press_char('a').await);
        assert!(!session.press_code(KeyCode::Enter).await);
        assert!(
            session.state.lock().await.is_editing_share_grant(),
            "a rejected id keeps the editor open for correction"
        );
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Enter a verified user ID")
        );
        assert_eq!(
            p2p_app::shares::list_grants(&share_id).expect("list"),
            Vec::new()
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_grant_actions_refuse_public_shares_and_missing_grants() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) = share_grant_db_state(p2p_app::shares::ShareKind::Public, &[]);
        let mut session = KeySession::new(state);

        assert!(!session.press_char('a').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Grants only apply to private shares")
        );
        assert!(!session.state.lock().await.is_editing_share_grant());

        // A key dismisses the popup before the next shortcut is dispatched.
        assert!(!session.press_code(KeyCode::Esc).await);
        assert!(!session.press_char('x').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Grants only apply to private shares")
        );
        assert_eq!(
            p2p_app::shares::list_grants(&share_id).expect("list"),
            Vec::new()
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_grant_revoke_without_selected_grant_shows_hint() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) =
            share_grant_db_state(p2p_app::shares::ShareKind::Private, &[]);
        let mut session = KeySession::new(state);

        assert!(!session.press_char('x').await);
        assert!(!session.state.lock().await.is_editing_share_grant());
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("No grant selected to revoke")
        );
        assert_eq!(
            p2p_app::shares::list_grants(&share_id).expect("list"),
            Vec::new()
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_grant_editor_owns_tab_and_mouse_until_cancelled() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) = share_grant_db_state(
            p2p_app::shares::ShareKind::Private,
            &[("alice", p2p_app::shares::ShareMode::ReadOnly)],
        );
        let mut session = KeySession::new(state);

        assert!(!session.press_char('a').await);
        let active_tab = session.state.lock().await.active_tab;
        assert!(!session.type_text("bob").await);

        assert!(!session.press_code(KeyCode::Tab).await);
        assert!(
            !session
                .press_mouse(wheel_event(MouseEventKind::ScrollDown))
                .await
        );
        assert!(!session.press_mouse(left_click_event(1, 4)).await);

        let frame = session.frame_text().await;
        assert!(frame.contains("Add share grant"), "frame: {frame}");
        {
            let s = session.state.lock().await;
            assert_eq!(s.active_tab, active_tab, "Tab stayed in the prompt");
            assert!(s.is_editing_share_grant());
            assert_eq!(s.chat_input.lines().join("\n"), "bob");
            assert_eq!(s.share_selection, 0);
        }
        assert!(session.drain_commands().is_empty());

        assert!(!session.press_code(KeyCode::Esc).await);
        {
            let s = session.state.lock().await;
            assert!(!s.is_editing_share_grant());
            assert!(s.share_grant_prompt.is_none());
            assert_eq!(s.chat_input.lines().join("\n"), "");
        }
        assert_eq!(
            p2p_app::shares::list_grants(&share_id).expect("list"),
            vec![p2p_app::shares::ShareGrant {
                user_id: "alice".to_string(),
                mode: p2p_app::shares::ShareMode::ReadOnly,
            }],
            "Esc must leave the persisted grants untouched"
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_creation_through_real_keys_persists_and_selects() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state) = empty_share_db_state();
        let mut session = KeySession::new(state);

        assert!(!session.press_char('c').await);
        let active_tab = session.state.lock().await.active_tab;
        assert!(session.state.lock().await.is_creating_share());
        assert!(session.state.lock().await.is_modal_editor());
        assert_eq!(session.state.lock().await.chat_input.lines().join("\n"), "");
        let frame = session.frame_text().await;
        assert!(frame.contains("Create share (name)"), "frame: {frame}");
        assert!(
            frame.contains("c: create a private share"),
            "frame: {frame}"
        );

        // A modal prompt owns Tab, arrows and the wheel: none may move the
        // highlight or leave the Shares tab while a name is being typed.
        assert!(!session.press_code(KeyCode::Tab).await);
        assert!(!session.press_code(KeyCode::Down).await);
        assert!(
            !session
                .press_mouse(wheel_event(MouseEventKind::ScrollDown))
                .await
        );
        {
            let s = session.state.lock().await;
            assert_eq!(s.active_tab, active_tab);
            assert!(s.is_creating_share());
        }
        assert!(!session.type_text("Roadmap").await);
        assert!(!session.press_code(KeyCode::Enter).await);

        let shares = p2p_app::shares::list_shares().expect("list shares");
        assert_eq!(shares.len(), 1, "Enter must persist exactly one share");
        let share = &shares[0];
        assert_eq!(share.name, "Roadmap");
        assert_eq!(share.owner_user_id, "owner");
        assert_eq!(share.kind, p2p_app::shares::ShareKind::Private);
        assert!(!share.public_write);
        assert_eq!(share.local_root, None);

        {
            let s = session.state.lock().await;
            assert!(!s.is_creating_share());
            assert_eq!(s.chat_input.lines().join("\n"), "");
            assert_eq!(s.shares.len(), 1);
            assert_eq!(s.share_selection, 0);
            assert_eq!(
                s.share_event_notice.as_deref(),
                Some("Share created: Roadmap")
            );
        }
        let frame = session.frame_text().await;
        assert!(frame.contains("Roadmap"), "frame: {frame}");
        assert!(frame.contains("Share created: Roadmap"), "frame: {frame}");
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_creation_validation_and_cancel_leave_store_untouched() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state) = empty_share_db_state();
        let mut session = KeySession::new(state);

        assert!(!session.press_char('c').await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(
                s.is_creating_share(),
                "a rejected name keeps the editor open for correction"
            );
            assert!(
                s.popup
                    .as_deref()
                    .is_some_and(|hint| hint.starts_with("Share not created:"))
            );
        }
        assert!(
            p2p_app::shares::list_shares()
                .expect("list shares")
                .is_empty(),
            "validation must happen before any DB write"
        );

        assert!(!session.press_code(KeyCode::Esc).await);
        assert!(!session.type_text("   ").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(s.is_creating_share());
            assert_eq!(s.chat_input.lines().join("\n"), "   ");
            assert!(
                s.popup
                    .as_deref()
                    .is_some_and(|hint| hint.starts_with("Share not created:"))
            );
        }

        assert!(!session.press_code(KeyCode::Esc).await);
        assert!(!session.press_code(KeyCode::Esc).await);
        {
            let s = session.state.lock().await;
            assert!(!s.is_creating_share());
            assert_eq!(s.chat_input.lines().join("\n"), "");
        }
        assert!(
            p2p_app::shares::list_shares()
                .expect("list shares")
                .is_empty(),
            "Esc must never create a share"
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_creation_requires_a_provisioned_owner() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state) = empty_share_db_state();
        state.own_user_id = None;
        let mut session = KeySession::new(state);

        assert!(!session.press_char('c').await);
        assert!(!session.type_text("Docs").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(s.is_creating_share());
            assert_eq!(s.popup.as_deref(), Some("Local user is not provisioned"));
        }
        let shares = p2p_app::shares::list_shares().expect("list shares");
        assert_eq!(shares, Vec::new());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_visibility_keys_persist_and_reload() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) =
            share_db_state("owner", p2p_app::shares::ShareKind::Private, false, &[]);
        let mut session = KeySession::new(state);
        assert!(
            session
                .frame_text()
                .await
                .contains("Access: p: toggle visibility | w: toggle public writes")
        );

        assert!(!session.press_char('p').await);
        assert_eq!(
            share_kind_and_write(&share_id),
            (p2p_app::shares::ShareKind::Public, false)
        );
        {
            let s = session.state.lock().await;
            assert_eq!(
                s.share_event_notice.as_deref(),
                Some("Share is public (writes: no)")
            );
        }
        assert!(
            session
                .frame_text()
                .await
                .contains("Kind: public (writes: no)")
        );

        assert!(!session.press_char('w').await);
        assert_eq!(
            share_kind_and_write(&share_id),
            (p2p_app::shares::ShareKind::Public, true)
        );
        assert!(
            session
                .frame_text()
                .await
                .contains("Kind: public (writes: allowed)")
        );

        assert!(!session.press_char('p').await);
        assert_eq!(
            share_kind_and_write(&share_id),
            (p2p_app::shares::ShareKind::Private, false),
            "returning to private must clear anonymous writes"
        );
        assert!(
            session
                .frame_text()
                .await
                .contains("Kind: private (writes: no)")
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_visibility_keys_refuse_foreign_shares_and_private_writes() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state, share_id) = share_db_state(
            "peer-owner",
            p2p_app::shares::ShareKind::Private,
            false,
            &[],
        );
        let mut session = KeySession::new(state);
        assert!(!session.press_char('p').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Only the share owner can change visibility")
        );
        assert_eq!(
            share_kind_and_write(&share_id),
            (p2p_app::shares::ShareKind::Private, false)
        );

        let (_dir, state, share_id) =
            share_db_state("owner", p2p_app::shares::ShareKind::Private, false, &[]);
        let mut session = KeySession::new(state);
        assert!(!session.press_char('w').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Only public shares can allow anonymous writes")
        );
        assert_eq!(
            share_kind_and_write(&share_id),
            (p2p_app::shares::ShareKind::Private, false)
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

// ── Shares network actions ────────────────────────────────────────────────

#[test]
#[serial]
fn test_share_network_keys_select_peer_and_queue_canonical_commands() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) =
            share_db_state("owner", p2p_app::shares::ShareKind::Public, false, &[]);
        // Insertion order is deliberately the reverse of canonical order.
        state.connected.on_peer_connected("peer-b".to_string());
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        let frame = session.frame_text().await;
        assert!(frame.contains("Connected peers (2)"), "frame: {frame}");
        assert!(frame.contains("> peer-a"), "frame: {frame}");

        assert!(!session.press_code(KeyCode::Right).await);
        assert_eq!(
            session
                .state
                .lock()
                .await
                .selected_share_peer_id()
                .as_deref(),
            Some("peer-b")
        );
        let frame = session.frame_text().await;
        assert!(frame.contains("> peer-b"), "frame: {frame}");

        assert!(!session.press_char('m').await);
        let commands = session.drain_commands();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            SwarmCommand::RequestShareManifest {
                peer_id,
                share_id: id,
            } => {
                assert_eq!(peer_id, "peer-b");
                assert_eq!(id, &share_id);
            }
            other => panic!("expected manifest request, got {other:?}"),
        }
        assert_eq!(
            session.state.lock().await.share_event_notice.as_deref(),
            Some("Manifest requested from peer-b")
        );

        assert!(!session.press_char('o').await);
        let commands = session.drain_commands();
        assert_eq!(commands.len(), 1);
        match &commands[0] {
            SwarmCommand::SendShareOffer { peer_ids, offer } => {
                assert_eq!(peer_ids, &vec!["peer-b".to_string()]);
                assert_eq!(offer.share_id, share_id);
                assert_eq!(offer.name, "Docs");
                assert_eq!(offer.owner_user_id.as_deref(), Some("owner"));
            }
            other => panic!("expected share offer, got {other:?}"),
        }
        assert_eq!(
            session.state.lock().await.share_event_notice.as_deref(),
            Some("Share re-advertised to peer-b")
        );
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_network_actions_require_share_and_connected_peer() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) =
            share_db_state("owner", p2p_app::shares::ShareKind::Public, false, &[]);
        state.connected.on_peer_connected("peer-a".to_string());
        state.shares.clear();
        let mut session = KeySession::new(state);

        assert!(!session.press_char('m').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Select a share and a connected peer")
        );
        assert!(session.drain_commands().is_empty());
        assert!(!session.press_code(KeyCode::Esc).await);

        {
            let mut s = session.state.lock().await;
            s.reload_shares();
            s.connected.on_peer_disconnected("peer-a", 1.0);
        }
        assert!(!session.press_char('m').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Select a share and a connected peer")
        );
        assert!(session.drain_commands().is_empty());
        assert!(!session.press_code(KeyCode::Esc).await);

        p2p_app::shares::delete_share(&share_id).expect("delete share");
        session
            .state
            .lock()
            .await
            .connected
            .on_peer_connected("peer-a".to_string());
        assert!(!session.press_char('o').await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("Share no longer exists")
        );
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_network_keys_are_inert_while_grant_editor_is_open() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) = share_grant_db_state(
            p2p_app::shares::ShareKind::Private,
            &[("alice", p2p_app::shares::ShareMode::ReadOnly)],
        );
        state.connected.on_peer_connected("peer-b".to_string());
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        assert!(!session.press_char('a').await);
        assert!(!session.press_code(KeyCode::Right).await);
        assert!(!session.press_char('m').await);
        assert!(!session.press_char('o').await);

        {
            let s = session.state.lock().await;
            assert!(s.is_editing_share_grant());
            assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-a"));
            assert_eq!(s.chat_input.lines().join("\n"), "mo");
            assert_eq!(s.share_selection, 0);
        }
        assert!(session.drain_commands().is_empty());
        assert_eq!(
            p2p_app::shares::list_grants(&share_id)
                .expect("list grants")
                .len(),
            1,
            "modal keys must not touch the canonical store"
        );
    });
    close_share_grant_db();
}

// ── manifest deletion publishing (Flutter parity) ─────────────────────────

#[test]
#[serial]
fn test_share_manifest_deletion_publishes_canonical_revision_through_real_keys() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) = share_manifest_db_state();
        // Reverse insertion order proves the bare peer cursor still comes from
        // the canonical sorted list and is independent of the manifest cursor.
        state.connected.on_peer_connected("peer-b".to_string());
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        let frame = session.frame_text().await;
        assert!(frame.contains("> [ ] docs/a.md"), "frame: {frame}");
        assert!(frame.contains("  [ ] docs/b.md"), "frame: {frame}");

        assert!(
            !session
                .press_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL))
                .await
        );
        {
            let s = session.state.lock().await;
            assert_eq!(s.share_manifest_selection, 1);
            assert_eq!(s.share_peer_selection, 0);
        }
        assert!(!session.press_code(KeyCode::Right).await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.share_manifest_selection, 1);
            assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-b"));
        }

        assert!(!session.press_char('d').await);
        let frame = session.frame_text().await;
        assert!(frame.contains("> [x] docs/b.md"), "frame: {frame}");
        assert_eq!(
            session.state.lock().await.share_deleted_paths.len(),
            1,
            "repeating d must toggle one path, not append it"
        );

        assert!(!session.press_ctrl_char('d').await);
        let commands = session.drain_commands();
        assert_eq!(commands.len(), 1, "commands: {commands:?}");
        match &commands[0] {
            SwarmCommand::PublishShareSnapshot {
                peer_id,
                base_revision,
                manifest,
            } => {
                assert_eq!(peer_id, "peer-b");
                assert_eq!(*base_revision, 1);
                assert_eq!(manifest.share_id, share_id);
                assert_eq!(manifest.name, "Docs");
                assert_eq!(manifest.revision, 2);
                assert_eq!(manifest.entries.len(), 1);
                assert_eq!(manifest.entries[0].path, "docs/a.md");
            }
            other => panic!("unexpected command: {other:?}"),
        }
        {
            let s = session.state.lock().await;
            assert!(s.share_deleted_paths.is_empty());
            assert_eq!(s.share_manifest_selection, 0);
            assert_eq!(s.share_manifest_revision, Some(1));
            assert_eq!(
                s.share_event_notice.as_deref(),
                Some("Deletion of 1 file submitted to peer-b")
            );
        }
        // Publishing only queues a snapshot; the canonical row still advances
        // when the owner accepts it over the share lane.
        let stored = p2p_app::shares::get_share(&share_id)
            .expect("get share")
            .expect("present");
        assert_eq!(stored.revision, 1);
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_manifest_deletion_draft_resets_when_a_new_revision_is_reloaded() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) = share_manifest_db_state();
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        assert!(
            !session
                .press_key(KeyEvent::new(KeyCode::Left, KeyModifiers::CONTROL))
                .await
        );
        assert!(!session.press_char('d').await);
        assert_eq!(session.state.lock().await.share_deleted_paths.len(), 1);

        let mut newer = p2p_app::shares::load_manifest(&share_id).expect("manifest");
        newer.revision = 2;
        p2p_app::shares::save_manifest(&share_id, &newer).expect("new revision");

        assert!(!session.press_char('r').await);
        {
            let s = session.state.lock().await;
            assert!(s.share_deleted_paths.is_empty());
            assert_eq!(s.share_manifest_selection, 0);
            assert_eq!(s.share_manifest_revision, Some(2));
        }

        assert!(!session.press_ctrl_char('d').await);
        assert!(session.drain_commands().is_empty());
        assert!(session.state.lock().await.popup.is_some());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_manifest_deletion_refuses_publish_when_the_store_moved_after_the_view_loaded() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, share_id) = share_manifest_db_state();
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        // The cached view is still on revision 1, but the canonical store has
        // moved on. The rebuilt candidate would be revision 3, so base 1 can
        // never be valid and no command may reach the owner.
        assert!(!session.press_char('d').await);
        let mut newer = p2p_app::shares::load_manifest(&share_id).expect("manifest");
        newer.revision = 2;
        p2p_app::shares::save_manifest(&share_id, &newer).expect("new revision");

        assert!(!session.press_ctrl_char('d').await);
        assert!(session.drain_commands().is_empty());
        let s = session.state.lock().await;
        assert_eq!(
            s.popup.as_deref(),
            Some("Manifest changed in the store: reload it with r")
        );
        assert_eq!(s.share_deleted_paths.len(), 1, "the draft stays editable");
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_manifest_deletion_keys_are_inert_while_a_modal_prompt_is_open() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state, _share_id) = share_manifest_db_state();
        state.connected.on_peer_connected("peer-a".to_string());
        let mut session = KeySession::new(state);

        assert!(!session.press_char('c').await);
        assert!(session.state.lock().await.is_creating_share());
        assert!(
            !session
                .press_key(KeyEvent::new(KeyCode::Right, KeyModifiers::CONTROL))
                .await
        );
        assert!(!session.press_char('d').await);
        assert!(!session.press_ctrl_char('d').await);

        {
            let s = session.state.lock().await;
            assert!(s.is_creating_share());
            assert_eq!(s.share_manifest_selection, 0);
            assert!(s.share_deleted_paths.is_empty());
            assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-a"));
        }
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

// ── peer-share discovery (Flutter parity) ─────────────────────────────────

fn discovered_offer(share_id: &str, name: &str, revision: u64) -> p2p_app::shares::ShareOffer {
    p2p_app::shares::ShareOffer {
        share_id: share_id.to_string(),
        name: name.to_string(),
        kind: p2p_app::shares::ShareKind::Private,
        revision,
        public_write: false,
        owner_user_id: None,
    }
}

/// Canonical sorted connected peers plus three discovered offers, landed on the
/// Shares tab with independent cursors at their defaults.
fn discovered_offers_state() -> AppState {
    let mut state = test_app_state();
    state.active_tab = shares_tab_index(&state);
    // Reverse insertion order proves the network target comes from the sorted list.
    for peer in ["peer-c", "peer-a", "peer-b"] {
        state.connected.on_peer_connected(peer.to_string());
    }
    for (peer, id, name, rev) in [
        ("peer-a", "a", "Alpha", 1),
        ("peer-b", "b", "Bravo", 2),
        ("peer-c", "c", "Charlie", 3),
    ] {
        state.record_discovered_share(peer.to_string(), discovered_offer(id, name, rev));
    }
    state
}

#[tokio::test]
async fn test_share_discovery_f_sends_list_shares_to_canonical_selected_peer() {
    // `f` only queues a command; it never touches the DB, so no serial guard
    // or isolated database is needed here.
    let mut session = KeySession::new(discovered_offers_state());

    assert!(!session.press_char('f').await);
    let commands = session.drain_commands();
    assert_eq!(commands.len(), 1, "commands: {commands:?}");
    match &commands[0] {
        SwarmCommand::ListShares { peer_id } => assert_eq!(peer_id, "peer-a"),
        other => panic!("unexpected command: {other:?}"),
    }
    let s = session.state.lock().await;
    assert_eq!(
        s.share_event_notice.as_deref(),
        Some("Share discovery requested from peer-a")
    );
    assert_eq!(s.discovered_shares.len(), 3, "asking never mutates offers");
}

#[tokio::test]
async fn test_share_discovery_f_without_peer_shows_hint_and_sends_nothing() {
    let mut state = test_app_state();
    state.active_tab = shares_tab_index(&state);
    let mut session = KeySession::new(state);

    assert!(!session.press_char('f').await);
    assert!(session.drain_commands().is_empty());
    assert_eq!(
        session.state.lock().await.popup.as_deref(),
        Some("Connect a peer before discovering shares")
    );
}

#[tokio::test]
async fn test_share_discovery_cursor_keys_move_only_the_offer_selection() {
    let mut session = KeySession::new(discovered_offers_state());

    let frame = session.frame_text().await;
    assert!(frame.contains("Discovered offers (3):"), "frame: {frame}");
    assert!(
        frame.contains("> Alpha [private] rev 1 from peer-a"),
        "frame: {frame}"
    );

    assert!(!session.press_char('j').await);
    assert_eq!(session.state.lock().await.share_discovery_selection, 1);
    assert!(!session.press_char('j').await);
    assert_eq!(session.state.lock().await.share_discovery_selection, 2);
    let frame = session.frame_text().await;
    assert!(
        frame.contains("> Charlie [private] rev 3 from peer-c"),
        "frame: {frame}"
    );

    assert!(!session.press_char('j').await);
    assert_eq!(
        session.state.lock().await.share_discovery_selection,
        2,
        "clamped"
    );
    assert!(!session.press_char('k').await);
    assert_eq!(session.state.lock().await.share_discovery_selection, 1);
    assert!(!session.press_char('k').await);
    assert_eq!(session.state.lock().await.share_discovery_selection, 0);
    assert!(!session.press_char('k').await);
    assert_eq!(
        session.state.lock().await.share_discovery_selection,
        0,
        "clamped"
    );

    {
        let s = session.state.lock().await;
        assert_eq!(s.share_selection, 0);
        assert_eq!(s.share_grant_selection, 0);
        assert_eq!(s.share_manifest_selection, 0);
        assert_eq!(s.share_peer_selection, 0);
    }
    assert!(session.drain_commands().is_empty());
}

#[tokio::test]
async fn test_share_discovery_keys_are_inert_while_a_modal_editor_is_open() {
    let mut session = KeySession::new(discovered_offers_state());

    assert!(!session.press_char('c').await);
    assert!(session.state.lock().await.is_creating_share());
    assert!(!session.press_char('j').await);
    assert!(!session.press_char('f').await);
    assert!(!session.press_char('k').await);

    {
        let s = session.state.lock().await;
        assert!(s.is_creating_share());
        assert_eq!(s.share_discovery_selection, 0);
        assert_eq!(s.discovered_shares.len(), 3);
        assert_eq!(s.chat_input.lines().join("\n"), "jfk");
        assert!(s.share_event_notice.is_none());
    }
    assert!(session.drain_commands().is_empty());
}

#[test]
#[serial]
fn test_share_discovery_enter_accepts_offer_through_the_canonical_store() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, mut state) = empty_share_db_state();
        state.record_discovered_share(
            "peer-a".to_string(),
            discovered_offer("remote-1", "Remote docs", 3),
        );
        let mut session = KeySession::new(state);

        assert!(!session.press_code(KeyCode::Enter).await);
        {
            let s = session.state.lock().await;
            assert!(s.discovered_shares.is_empty(), "accepted offer is consumed");
            assert_eq!(s.shares.len(), 1);
            assert_eq!(s.shares[0].share_id, "remote-1");
            assert_eq!(s.share_selection, 0);
            assert_eq!(
                s.share_event_notice.as_deref(),
                Some("Added Remote docs from peer-a")
            );
        }
        let stored = p2p_app::shares::get_share("remote-1")
            .expect("get share")
            .expect("share stored canonically");
        assert_eq!(stored.name, "Remote docs");
        // A stub starts at revision 0: the advertised revision is untrusted
        // metadata, so the canonical store never adopts it.
        assert_eq!(stored.revision, 0);
        assert!(
            p2p_app::shares::is_unknown_owner_id(&stored.owner_user_id),
            "an unclaimed owner must never become a real account id"
        );
        // Accepting is a local store write; no network command is queued.
        assert!(session.drain_commands().is_empty());
    });
    close_share_grant_db();
}

#[test]
#[serial]
fn test_share_discovery_enter_without_offer_does_not_accept_any_share() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let rt = tokio::runtime::Runtime::new().expect("runtime");
    rt.block_on(async {
        let (_dir, state) = empty_share_db_state();
        let mut session = KeySession::new(state);

        assert!(!session.press_code(KeyCode::Enter).await);
        assert_eq!(
            session.state.lock().await.popup.as_deref(),
            Some("No discovered share selected")
        );
        assert!(session.drain_commands().is_empty());
        assert_eq!(
            p2p_app::shares::list_shares().expect("list shares"),
            Vec::new()
        );
    });
    close_share_grant_db();
}
