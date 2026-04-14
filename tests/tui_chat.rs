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
}
