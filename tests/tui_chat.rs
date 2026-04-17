#[cfg(feature = "tui")]
mod tests {
    use p2p_app::tui::{DmTab, NotificationTarget, TabId, TuiTestState};

    #[test]
    fn test_layout_rows_no_notification() {
        let state = TuiTestState::new();
        assert_eq!(state.list_header_start_row(), 3);
        assert_eq!(state.first_message_row(), 5);
    }

    #[test]
    fn test_layout_rows_with_notification() {
        let mut state = TuiTestState::new();
        state.unread_broadcasts = 5;
        assert_eq!(state.list_header_start_row(), 4);
        assert_eq!(state.first_message_row(), 6);
    }

    #[test]
    fn test_handle_mouse_click_first_message() {
        let state = TuiTestState::new();
        let first_msg_row = state.first_message_row();

        let peer = state.handle_mouse_click(first_msg_row, 5);
        assert!(
            peer.is_some(),
            "Should click first message at row {}",
            first_msg_row
        );
    }

    #[test]
    fn test_handle_mouse_click_second_message() {
        let custom = std::collections::VecDeque::from(vec![
            "[You] msg0".to_string(),
            "[Peer1] msg1".to_string(),
            "[You] msg2".to_string(),
        ]);
        let state = TuiTestState::with_messages(custom);
        let first_msg_row = state.first_message_row();

        let peer = state.handle_mouse_click(first_msg_row, 5);
        assert_eq!(peer, Some("You".to_string()), "First message is from You");

        let peer2 = state.handle_mouse_click(first_msg_row + 1, 5);
        assert_eq!(
            peer2,
            Some("Peer1".to_string()),
            "Second message is from Peer1"
        );
    }

    #[test]
    fn test_handle_mouse_click_outside_bounds() {
        let state = TuiTestState::new();
        let list_header_row = state.list_header_start_row();

        let peer = state.handle_mouse_click(list_header_row - 1, 5);
        assert!(peer.is_none(), "Click above list header should return None");
    }

    #[test]
    fn test_handle_mouse_click_with_scroll_offset() {
        let custom = std::collections::VecDeque::from(vec![
            "[You] msg0".to_string(),
            "[Peer1] msg1".to_string(),
            "[You] msg2".to_string(),
            "[Peer2] msg3".to_string(),
        ]);
        let state = TuiTestState::with_messages(custom);
        let first_msg_row = state.first_message_row();

        let mut scrolled = state.clone();
        scrolled.chat_list_state_offset = 2;

        let peer = scrolled.handle_mouse_click(first_msg_row, 5);
        assert_eq!(
            peer,
            Some("You".to_string()),
            "Scrolled view: first visible is msg2"
        );
    }

    #[test]
    fn test_handle_mouse_click_multiline_message() {
        let custom = std::collections::VecDeque::from(vec![
            "[Peer1] line1\nline2\nline3".to_string(),
            "[Peer2] msg2".to_string(),
        ]);
        let state = TuiTestState::with_messages(custom);
        let first_msg_row = state.first_message_row();

        let peer_line1 = state.handle_mouse_click(first_msg_row, 5);
        assert_eq!(
            peer_line1,
            Some("Peer1".to_string()),
            "First line of multi-line message"
        );

        let peer_line2 = state.handle_mouse_click(first_msg_row + 1, 5);
        assert_eq!(
            peer_line2,
            Some("Peer1".to_string()),
            "Second line of multi-line message"
        );

        let peer_line4 = state.handle_mouse_click(first_msg_row + 3, 5);
        assert_eq!(
            peer_line4,
            Some("Peer2".to_string()),
            "Fourth row is second message"
        );
    }

    #[test]
    fn test_calculate_content_start_row() {
        let state = TuiTestState::new();
        let start_row = state.calculate_content_start_row();

        let mut state_with_notifs = state.clone();
        state_with_notifs.unread_broadcasts = 5;
        let start_row_with_notifs = state_with_notifs.calculate_content_start_row();
        assert!(start_row_with_notifs > start_row);
    }

    #[test]
    fn test_notification_click_broadcasts() {
        let mut state = TuiTestState::new();
        state.unread_broadcasts = 5;
        state.unread_dms.insert("Peer1".to_string(), 2);

        let target = state.handle_notification_click(5);
        assert!(matches!(target, Some(NotificationTarget::Broadcasts)));
    }

    #[test]
    fn test_notification_click_dm() {
        let mut state = TuiTestState::new();
        state.unread_dms.insert("Peer1".to_string(), 2);

        let target = state.handle_notification_click(40);
        match target {
            Some(NotificationTarget::Dm(pid)) => assert_eq!(pid, "Peer1"),
            _ => panic!("Expected Dm notification target"),
        }
    }

    #[test]
    fn test_tui_test_state_custom_messages() {
        let custom_messages = std::collections::VecDeque::from(vec![
            String::from("[You] Hello world"),
            String::from("[Peer1] How are you"),
            String::from("[You] I am good"),
        ]);

        let state = TuiTestState::with_messages(custom_messages);

        assert_eq!(state.messages.len(), 3);
        assert_eq!(state.messages[0], "[You] Hello world");
        assert_eq!(state.messages[1], "[Peer1] How are you");
        assert_eq!(state.messages[2], "[You] I am good");

        assert_eq!(state.chat_message_peers.len(), 3);
        assert_eq!(state.chat_message_peers[0], "You");
        assert_eq!(state.chat_message_peers[1], "Peer1");
        assert_eq!(state.chat_message_peers[2], "You");
    }

    #[test]
    fn test_tab_id_index() {
        assert_eq!(TabId::Chat.index(), 0);
        assert_eq!(TabId::Peers.index(), 1);
        assert_eq!(TabId::Direct.index(), 2);
        assert_eq!(TabId::Log.index(), 3);
    }

    #[test]
    fn test_tab_id_from_index() {
        assert_eq!(TabId::from_index(0), TabId::Chat);
        assert_eq!(TabId::from_index(1), TabId::Peers);
        assert_eq!(TabId::from_index(2), TabId::Direct);
        assert_eq!(TabId::from_index(3), TabId::Log);
        assert_eq!(TabId::from_index(99), TabId::Chat);
    }

    #[test]
    fn test_tab_id_default() {
        let default_tab: TabId = TabId::default();
        assert_eq!(default_tab, TabId::Chat);
    }

    #[test]
    fn test_dm_tab_new() {
        let dm = DmTab::new("peer123".to_string());
        assert_eq!(dm.peer_id, "peer123");
        assert!(dm.messages.is_empty());
    }

    #[test]
    fn test_dm_tab_with_messages() {
        let messages = std::collections::VecDeque::from(vec![
            String::from("[You] Hello"),
            String::from("[Peer] Hi there"),
        ]);
        let dm = DmTab::with_messages("peer456".to_string(), messages.clone());
        assert_eq!(dm.peer_id, "peer456");
        assert_eq!(dm.messages.len(), 2);
    }

    #[test]
    fn test_dm_tab_clone() {
        let dm1 = DmTab::new("peer789".to_string());
        let dm2 = dm1.clone();
        assert_eq!(dm1.peer_id, dm2.peer_id);
    }

    #[test]
    fn test_tab_id_partial_eq() {
        assert_eq!(TabId::Chat, TabId::Chat);
        assert_ne!(TabId::Chat, TabId::Peers);
    }

    #[test]
    fn test_dm_tab_partial_eq() {
        let dm1 = DmTab::new("peer123".to_string());
        let dm2 = DmTab::new("peer123".to_string());
        let dm3 = DmTab::new("peer456".to_string());
        assert_eq!(dm1, dm2);
        assert_ne!(dm1, dm3);
    }

    #[test]
    fn test_dm_tab_message_persistence() {
        let messages = vec![
            "alice [10:00:00] Hello".to_string(),
            "bob [10:01:00] Hi there".to_string(),
        ]
        .into_iter()
        .collect();
        let dm = DmTab::with_messages("alice".to_string(), messages);

        assert_eq!(dm.messages.len(), 2);
        assert_eq!(dm.peer_id, "alice");
        assert!(dm.messages.front().unwrap().contains("Hello"));
    }

    #[test]
    fn test_dm_notification_tracking() {
        let mut state = TuiTestState::new();
        state.unread_dms.insert("Peer1".to_string(), 2);
        state.unread_dms.insert("Peer2".to_string(), 1);

        assert_eq!(state.unread_dms.get("Peer1"), Some(&2));
        assert_eq!(state.unread_dms.get("Peer2"), Some(&1));
    }

    #[test]
    fn test_empty_messages_handling() {
        let empty: std::collections::VecDeque<String> = std::collections::VecDeque::new();
        let state = TuiTestState::with_messages(empty);

        assert!(state.messages.is_empty());
        assert!(state.chat_message_peers.is_empty());
    }

    #[test]
    fn test_peer_extraction_no_brackets() {
        let no_bracket = std::collections::VecDeque::from(vec!["just a plain message".to_string()]);
        let state = TuiTestState::with_messages(no_bracket);

        assert_eq!(state.chat_message_peers.len(), 1);
        assert!(state.chat_message_peers[0].is_empty());
    }

    #[test]
    fn test_dynamic_tabs_add_remove() {
        let mut tabs = p2p_app::tui::DynamicTabs::new();

        assert_eq!(tabs.dm_tab_count(), 0);

        let idx = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);

        let idx2 = tabs.add_dm_tab("peer2".to_string());
        assert_eq!(idx2, 3);
        assert_eq!(tabs.dm_tab_count(), 2);

        let idx3 = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx3, 2);

        let removed = tabs.remove_dm_tab("peer1");
        assert!(removed.is_some());
        assert_eq!(tabs.dm_tab_count(), 1);

        let removed2 = tabs.remove_dm_tab("peer1");
        assert!(removed2.is_none());
    }

    #[test]
    fn test_dynamic_tabs_titles() {
        let mut tabs = p2p_app::tui::DynamicTabs::new();
        tabs.add_dm_tab("12D3KooWSkP1pEPy2".to_string());
        tabs.add_dm_tab("12D3KooWGDyE67".to_string());

        let titles = tabs.all_titles();
        assert_eq!(titles.len(), 5);
        assert_eq!(titles[0], "Chat");
        assert_eq!(titles[1], "Peers");
        assert!(titles[2].contains("(X)"));
        assert!(titles[3].contains("(X)"));
        assert_eq!(titles[4], "Log");
    }

    #[test]
    fn test_dm_tab_short_id() {
        let dm =
            p2p_app::tui::DmTab::new("12D3KooWSkP1pEPy2EETdeJBbMRju1oWAwUBngQYJ2Ai".to_string());
        let short = dm.short_id();
        assert_eq!(short.len(), 8);
    }

    #[test]
    fn test_tab_content_mapping() {
        let mut tabs = p2p_app::tui::DynamicTabs::new();
        tabs.add_dm_tab("peerA".to_string());
        tabs.add_dm_tab("peerB".to_string());

        assert_eq!(tabs.tab_index_to_content(0), p2p_app::tui::TabContent::Chat);
        assert_eq!(
            tabs.tab_index_to_content(1),
            p2p_app::tui::TabContent::Peers
        );
        assert_eq!(
            tabs.tab_index_to_content(2),
            p2p_app::tui::TabContent::Direct("peerA".to_string())
        );
        assert_eq!(
            tabs.tab_index_to_content(3),
            p2p_app::tui::TabContent::Direct("peerB".to_string())
        );
        assert_eq!(tabs.tab_index_to_content(4), p2p_app::tui::TabContent::Log);
    }
}
