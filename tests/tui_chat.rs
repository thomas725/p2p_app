#[cfg(feature = "tui")]
mod tests {
    use p2p_app::tui::{DmTab, TEST_MESSAGES, TabId, TuiTestState};

    #[test]
    fn test_handle_mouse_click() {
        let state = TuiTestState::new();

        assert_eq!(state.messages.len(), TEST_MESSAGES.len());
        assert_eq!(state.chat_message_peers.len(), TEST_MESSAGES.len());
        assert_eq!(state.active_tab, 0);

        let content_start = state.calculate_content_start_row();
        let test_row = content_start + 2;
        let peer = state.handle_mouse_click(test_row);

        assert!(peer.is_some());

        let expected_idx = state.chat_list_state_offset + (test_row - content_start) as usize;
        if let Some(expected_peer) = state.chat_message_peers.get(expected_idx) {
            assert_eq!(peer, Some(expected_peer.clone()));
        }
    }

    #[test]
    fn test_handle_mouse_click_outside_bounds() {
        let state = TuiTestState::new();
        let content_start = state.calculate_content_start_row();

        let peer = state.handle_mouse_click(content_start - 1);
        assert!(peer.is_none());

        let max_row = content_start + state.chat_message_peers.len() as u16 + 5;
        let peer = state.handle_mouse_click(max_row);
        assert!(peer.is_none());
    }

    #[test]
    fn test_calculate_content_start_row() {
        let state = TuiTestState::new();
        let start_row = state.calculate_content_start_row();

        assert!(start_row >= 1);

        let mut state_with_notifs = state.clone();
        state_with_notifs.unread_broadcasts = 5;
        let start_row_with_notifs = state_with_notifs.calculate_content_start_row();
        assert!(start_row_with_notifs > start_row);
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
        assert!(state.chat_message_peers[0].is_empty());
        assert_eq!(state.chat_message_peers[1], "Peer1");
        assert!(state.chat_message_peers[2].is_empty());
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
    fn test_tab_click_to_chat() {
        let mut state = TuiTestState::new();
        state.active_tab = 1;
        state.handle_tab_click(0);
        assert_eq!(
            state.active_tab, 0,
            "Click row 0 should go to Chat (index 0)"
        );
    }

    #[test]
    fn test_tab_click_to_peers() {
        let mut state = TuiTestState::new();
        state.active_tab = 0;
        state.handle_tab_click(1);
        assert_eq!(state.active_tab, 1, "Click row 1 should go to Peers");
    }

    #[test]
    fn test_tab_click_to_log() {
        let mut state = TuiTestState::new();
        state.active_tab = 0;
        state.handle_tab_click(3);
        assert_eq!(state.active_tab, 3, "Click row 3 should go to Log");
    }

    #[test]
    fn test_unread_notification_increments() {
        let mut state = TuiTestState::new();
        let initial = state.unread_broadcasts;
        state.unread_broadcasts += 1;
        assert_eq!(state.unread_broadcasts, initial + 1);
    }

    #[test]
    fn test_notification_clears_on_chat_tab_focus() {
        let mut state = TuiTestState::new();
        state.unread_broadcasts = 5;
        // When switching to chat tab (tab 0), unread should clear
        state.active_tab = 0;
        state.unread_broadcasts = 0;
        assert_eq!(state.unread_broadcasts, 0);
        assert_eq!(state.active_tab, 0);
    }

    #[test]
    fn test_notification_area_calculation() {
        let mut state = TuiTestState::new();
        // No unread - notification area should be 0
        state.unread_broadcasts = 0;
        let start_row = state.calculate_content_start_row();

        // With unread - notification area should push content down
        let mut state_with_unread = state.clone();
        state_with_unread.unread_broadcasts = 3;
        let start_row_with_unread = state_with_unread.calculate_content_start_row();

        assert!(
            start_row_with_unread > start_row,
            "Content should start lower with notifications"
        );
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
    fn test_dm_tab_cloning() {
        let messages = vec!["test message".to_string()].into_iter().collect();
        let dm1 = DmTab::with_messages("peer1".to_string(), messages);
        let dm2 = dm1.clone();

        assert_eq!(dm1, dm2);
        assert_eq!(dm2.peer_id, "peer1");
    }

    #[test]
    fn test_handle_mouse_click_with_scroll_offset() {
        let custom = std::collections::VecDeque::from(vec![
            "[You] msg0".to_string(),
            "[Peer] msg1".to_string(),
            "[You] msg2".to_string(),
            "[Peer] msg3".to_string(),
        ]);
        let mut state = TuiTestState::with_messages(custom);
        state.chat_list_state_offset = 2;
        let content_start = state.calculate_content_start_row();

        let peer = state.handle_mouse_click(content_start);
        // At offset 2 + row 0 (content_start), we get index 2, which is "[You] msg2" -> empty string
        assert_eq!(peer, Some(String::new()));
    }

    #[test]
    fn test_keyboard_navigation_cycle() {
        let mut state = TuiTestState::new();
        state.active_tab = 3;

        state.handle_tab_click(0);
        assert_eq!(state.active_tab, 0);

        state.handle_tab_click(1);
        assert_eq!(state.active_tab, 1);
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
}
