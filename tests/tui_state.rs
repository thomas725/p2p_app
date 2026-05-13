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

// ── TabId ────────────────────────────────────────────────────────────────────

#[test]
fn test_tab_id_index_roundtrip() {
    use p2p_app::tui_tabs::TabId;
    assert_eq!(TabId::Chat.index(), 0);
    assert_eq!(TabId::Peers.index(), 1);
    assert_eq!(TabId::Direct.index(), 2);
    assert_eq!(TabId::Log.index(), 3);
}

#[test]
fn test_tab_id_from_index() {
    use p2p_app::tui_tabs::TabId;
    assert_eq!(TabId::from_index(0), TabId::Chat);
    assert_eq!(TabId::from_index(1), TabId::Peers);
    assert_eq!(TabId::from_index(2), TabId::Direct);
    assert_eq!(TabId::from_index(3), TabId::Log);
    assert_eq!(TabId::from_index(99), TabId::Chat); // out-of-range → Chat
}

#[test]
fn test_tab_id_default() {
    use p2p_app::tui_tabs::TabId;
    assert_eq!(TabId::default(), TabId::Chat);
}

// ── DmTab ────────────────────────────────────────────────────────────────────

#[test]
fn test_dm_tab_short_id_long_peer() {
    use p2p_app::tui_tabs::DmTab;
    let tab = DmTab::new("12D3KooWABCDEFGH".to_string());
    assert_eq!(tab.short_id(), "ABCDEFGH");
}

#[test]
fn test_dm_tab_short_id_short_peer() {
    use p2p_app::tui_tabs::DmTab;
    let tab = DmTab::new("tiny".to_string());
    assert_eq!(tab.short_id(), "tiny");
}

#[test]
fn test_dm_tab_with_messages() {
    use p2p_app::tui_tabs::DmTab;
    use std::collections::VecDeque;
    let msgs = VecDeque::from(["hello".to_string(), "world".to_string()]);
    let tab = DmTab::with_messages("peer-x".to_string(), msgs.clone());
    assert_eq!(tab.messages, msgs);
}

// ── DynamicTabs ──────────────────────────────────────────────────────────────

#[test]
fn test_dynamic_tabs_add_dm_returns_index() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    let idx = tabs.add_dm_tab("peer-1".to_string());
    assert_eq!(idx, 2); // first DM tab is at global index 2
}

#[test]
fn test_dynamic_tabs_add_existing_dm_returns_existing_index() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-1".to_string());
    let idx = tabs.add_dm_tab("peer-1".to_string()); // same peer
    assert_eq!(idx, 2);
    assert_eq!(tabs.dm_tab_count(), 1); // no duplicate
}

#[test]
fn test_dynamic_tabs_get_dm_tab_mut() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-m".to_string());
    let tab = tabs.get_dm_tab_mut("peer-m").unwrap();
    tab.messages.push_back("hi".to_string());
    assert_eq!(tabs.get_dm_tab("peer-m").unwrap().messages[0], "hi");
}

#[test]
fn test_dynamic_tabs_get_dm_tab_mut_unknown() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    assert!(tabs.get_dm_tab_mut("nobody").is_none());
}

#[test]
fn test_dynamic_tabs_dm_tab_titles() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("12D3KooWABCDEFGH".to_string());
    let titles = tabs.dm_tab_titles();
    assert_eq!(titles.len(), 1);
    assert!(titles[0].contains("ABCDEFGH"));
    assert!(titles[0].contains("(X)"));
}

#[test]
fn test_dynamic_tabs_all_titles() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-t".to_string());
    let titles = tabs.all_titles();
    assert_eq!(titles[0], "Chat");
    assert_eq!(titles[1], "Peers");
    assert_eq!(*titles.last().unwrap(), "Log");
    assert_eq!(titles.len(), 4); // Chat, Peers, 1 DM, Log
}

#[test]
fn test_dynamic_tabs_total_tab_count() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    assert_eq!(tabs.total_tab_count(), 3); // Chat, Peers, Log
    tabs.add_dm_tab("p".to_string());
    assert_eq!(tabs.total_tab_count(), 4);
}

#[test]
fn test_dynamic_tabs_tab_index_to_content() {
    use p2p_app::tui_tabs::{DynamicTabs, TabContent};
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-c".to_string());
    assert_eq!(tabs.tab_index_to_content(0), TabContent::Chat);
    assert_eq!(tabs.tab_index_to_content(1), TabContent::Peers);
    assert_eq!(
        tabs.tab_index_to_content(2),
        TabContent::Direct("peer-c".to_string())
    );
    assert_eq!(tabs.tab_index_to_content(3), TabContent::Log);
    assert_eq!(tabs.tab_index_to_content(99), TabContent::Chat); // out of range
}

// ── TabContent ───────────────────────────────────────────────────────────────

#[test]
fn test_tab_content_peer_id() {
    use p2p_app::tui_tabs::TabContent;
    assert_eq!(
        TabContent::Direct("peer-x".to_string()).peer_id(),
        Some("peer-x")
    );
    assert!(TabContent::Chat.peer_id().is_none());
    assert!(TabContent::Peers.peer_id().is_none());
    assert!(TabContent::Log.peer_id().is_none());
}

#[test]
fn test_tab_content_is_input_enabled() {
    use p2p_app::tui_tabs::TabContent;
    assert!(TabContent::Chat.is_input_enabled());
    assert!(TabContent::Direct("p".to_string()).is_input_enabled());
    assert!(!TabContent::Peers.is_input_enabled());
    assert!(!TabContent::Log.is_input_enabled());
}

// ── DynamicTabs additional methods ─────────────────────────────────────────────

#[test]
fn test_add_dm_tab_returns_index_and_deduplicates() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    let idx1 = tabs.add_dm_tab("peer-1".to_string());
    let idx2 = tabs.add_dm_tab("peer-1".to_string());
    assert_eq!(
        idx1, idx2,
        "adding same peer twice should return same index"
    );
    assert_eq!(tabs.dm_tab_count(), 1, "should have only 1 DM tab");
}

#[test]
fn test_remove_dm_tab_by_peer_id() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-to-remove".to_string());
    tabs.add_dm_tab("peer-keep".to_string());
    assert_eq!(tabs.dm_tab_count(), 2);
    let removed = tabs.remove_dm_tab("peer-to-remove");
    assert!(removed.is_some());
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_remove_dm_tab_nonexistent() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    let removed = tabs.remove_dm_tab("nonexistent");
    assert!(
        removed.is_none(),
        "removing nonexistent tab should return None"
    );
}

#[test]
fn test_dm_tab_count() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    assert_eq!(tabs.dm_tab_count(), 0);
    tabs.add_dm_tab("peer-1".to_string());
    assert_eq!(tabs.dm_tab_count(), 1);
    tabs.add_dm_tab("peer-2".to_string());
    assert_eq!(tabs.dm_tab_count(), 2);
}

#[test]
fn test_dm_tab_count_after_remove() {
    use p2p_app::tui_tabs::DynamicTabs;
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("p1".to_string());
    tabs.add_dm_tab("p2".to_string());
    tabs.add_dm_tab("p3".to_string());
    tabs.remove_dm_tab("p2");
    assert_eq!(tabs.dm_tab_count(), 2);
}
