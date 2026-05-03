//! Tests for public API modules

#[test]
fn test_tui_test_state_new() {
    let state = p2p_app::tui_test_state::TuiTestState::new();

    assert!(!state.messages.is_empty());
    assert_eq!(state.active_tab, 0);
}

#[test]
fn test_tui_test_state_with_messages() {
    let messages = std::collections::VecDeque::from(vec![
        "[You] test1".to_string(),
        "[Peer] test2".to_string(),
    ]);
    let state = p2p_app::tui_test_state::TuiTestState::with_messages(messages.clone());

    assert_eq!(state.messages.len(), 2);
}

#[test]
fn test_tui_test_state_with_width() {
    let messages = std::collections::VecDeque::from(vec!["[You] test".to_string()]);
    let state = p2p_app::tui_test_state::TuiTestState::with_messages_and_width(messages, 40);

    assert_eq!(state.terminal_width, 40);
}

#[test]
fn test_dm_tab_new() {
    let dm = p2p_app::tui_tabs::DmTab::new("peer1".to_string());
    assert_eq!(dm.peer_id, "peer1");
    assert!(dm.messages.is_empty());
}

#[test]
fn test_dynamic_tabs_new() {
    let tabs = p2p_app::tui_tabs::DynamicTabs::new();
    assert_eq!(tabs.total_tab_count(), 3);
    assert_eq!(tabs.dm_tab_count(), 0);
}

#[test]
fn test_dynamic_tabs_add_dm() {
    let mut tabs = p2p_app::tui_tabs::DynamicTabs::new();
    let idx = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx, 2);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_dm() {
    let mut tabs = p2p_app::tui_tabs::DynamicTabs::new();
    tabs.add_dm_tab("peer1".to_string());
    let removed = tabs.remove_dm_tab("peer1");
    assert!(removed.is_some());
}

#[test]
fn test_tab_content_chat() {
    let content = p2p_app::tui_tabs::TabContent::Chat;
    assert_eq!(content.peer_id(), None);
    assert!(content.is_input_enabled());
}

#[test]
fn test_tab_content_direct() {
    let content = p2p_app::tui_tabs::TabContent::Direct("peer1".to_string());
    assert_eq!(content.peer_id(), Some("peer1"));
    assert!(content.is_input_enabled());
}

#[test]
fn test_notification_target_broadcasts() {
    let target = p2p_app::tui_test_state::NotificationTarget::Broadcasts;
    assert!(matches!(
        target,
        p2p_app::tui_test_state::NotificationTarget::Broadcasts
    ));
}

#[test]
fn test_notification_target_dm() {
    let target = p2p_app::tui_test_state::NotificationTarget::Dm("peer1".to_string());
    match target {
        p2p_app::tui_test_state::NotificationTarget::Dm(id) => assert_eq!(id, "peer1"),
        _ => panic!("Expected Dm"),
    }
}
