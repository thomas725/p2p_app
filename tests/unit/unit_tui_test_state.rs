use super::*;
use std::collections::VecDeque;

#[test]
fn constructors_and_defaults() {
    let state = TuiTestState::new();
    assert!(!state.messages.is_empty());
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.chat_list_state_offset, 0);
    assert_eq!(state.terminal_width, 80);

    let default_state = TuiTestState::default();
    assert_eq!(default_state.messages.len(), state.messages.len());
}

#[test]
fn with_messages_extracts_peers() {
    let msgs = VecDeque::from([
        "[You] hello".to_string(),
        "[Alice] hi".to_string(),
        "plain".to_string(),
    ]);
    let state = TuiTestState::with_messages_and_width(msgs, 100);
    assert_eq!(state.chat_message_peers[0], "You");
    assert_eq!(state.chat_message_peers[1], "Alice");
    assert_eq!(state.chat_message_peers[2], "");
    assert_eq!(state.terminal_width, 100);
}

#[test]
fn row_calculations_and_tab_click() {
    let mut state = TuiTestState::new();
    assert_eq!(state.list_header_start_row(), 3);
    assert_eq!(state.first_message_row(), 5);
    assert_eq!(state.calculate_content_start_row(), 5);

    state.handle_tab_click(1);
    assert_eq!(state.active_tab, 0);
    state.handle_tab_click(4);
    assert_eq!(state.active_tab, 0);
}

#[test]
fn notification_click_targets() {
    let mut state = TuiTestState::new();
    assert!(state.handle_notification_click(1).is_none());

    state.unread_broadcasts = 2;
    matches!(
        state.handle_notification_click(10),
        Some(NotificationTarget::Broadcasts)
    );

    state.unread_dms.insert("peer-1".to_string(), 1);
    match state.handle_notification_click(25) {
        Some(NotificationTarget::Dm(peer)) => assert_eq!(peer, "peer-1"),
        _ => panic!("expected DM notification"),
    }
}

#[test]
fn mouse_click_peer_detection() {
    let msgs = VecDeque::from(["[You] hello".to_string(), "[Bob] hi".to_string()]);
    let state = TuiTestState::with_messages(msgs);

    assert_eq!(state.handle_mouse_click(0, 0), None);
    let row = state.first_message_row();
    assert_eq!(state.handle_mouse_click(row, 0), Some("You".to_string()));
    assert_eq!(state.handle_mouse_click(row + 100, 0), None);
}

#[test]
fn formatting_helpers_and_tab_content() {
    let mut state = TuiTestState::new();
    assert_eq!(state.tab_titles(), vec!["Chat", "Peers", "Log"]);
    assert!(state.peer_info().contains("Peers:"));
    assert!(!state.formatted_messages().is_empty());
    assert_eq!(state.formatted_peers().len(), 2);
    assert!(!state.formatted_dm_messages("x").is_empty());
    assert_eq!(state.formatted_logs().len(), 2);
    assert_eq!(state.input_text(), "");
    assert!(state.status_text().contains("Ready"));

    state.active_tab = 0;
    assert!(matches!(
        state.tab_content(),
        crate::tui_tabs::TabContent::Chat
    ));
    state.active_tab = 1;
    assert!(matches!(
        state.tab_content(),
        crate::tui_tabs::TabContent::Peers
    ));
    state.active_tab = 2;
    assert!(matches!(
        state.tab_content(),
        crate::tui_tabs::TabContent::Log
    ));
}
