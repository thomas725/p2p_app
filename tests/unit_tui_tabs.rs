    use super::*;

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
    fn test_dm_tab_new() {
        let dm = DmTab::new("12D3KooWABCDEFGH".to_string());
        assert_eq!(dm.peer_id, "12D3KooWABCDEFGH");
        assert!(dm.messages.is_empty());
    }

    #[test]
    fn test_dm_tab_with_messages() {
        let msgs = VecDeque::from(vec!["msg1".to_string(), "msg2".to_string()]);
        let dm = DmTab::with_messages("peer1".to_string(), msgs.clone());
        assert_eq!(dm.peer_id, "peer1");
        assert_eq!(dm.messages.len(), 2);
    }

    #[test]
    fn test_dm_tab_short_id() {
        let dm = DmTab::new("12D3KooWABCDEFGH".to_string());
        assert_eq!(dm.short_id(), "ABCDEFGH");
    }

    #[test]
    fn test_dm_tab_short_id_short_peer() {
        let dm = DmTab::new("short".to_string());
        assert_eq!(dm.short_id(), "short");
    }

    #[test]
    fn test_dynamic_tabs_new() {
        let tabs = DynamicTabs::new();
        assert_eq!(tabs.dm_tab_count(), 0);
        assert_eq!(tabs.total_tab_count(), 3);
    }

    #[test]
    fn test_dynamic_tabs_add_dm_tab() {
        let mut tabs = DynamicTabs::new();
        let idx = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);
    }

    #[test]
    fn test_dynamic_tabs_remove_dm_tab() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let idx = tabs.remove_dm_tab("peer1");
        assert_eq!(idx, Some(2));
        assert_eq!(tabs.dm_tab_count(), 0);
    }

    #[test]
    fn test_dynamic_tabs_get_dm_tab() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let dm = tabs.get_dm_tab("peer1");
        assert!(dm.is_some());
        assert_eq!(dm.unwrap().peer_id, "peer1");
    }

    #[test]
    fn test_dynamic_tabs_dm_tab_titles() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        tabs.add_dm_tab("peer2".to_string());
        let titles = tabs.dm_tab_titles();
        assert_eq!(titles.len(), 2);
    }

    #[test]
    fn test_tab_content_peer_id() {
        assert_eq!(
            TabContent::Direct("peer1".to_string()).peer_id(),
            Some("peer1")
        );
        assert_eq!(TabContent::Chat.peer_id(), None);
        assert_eq!(TabContent::Peers.peer_id(), None);
        assert_eq!(TabContent::Log.peer_id(), None);
    }

    #[test]
    fn test_tab_content_is_input_enabled() {
        assert!(TabContent::Chat.is_input_enabled());
        assert!(TabContent::Direct("peer1".to_string()).is_input_enabled());
        assert!(!TabContent::Peers.is_input_enabled());
        assert!(!TabContent::Log.is_input_enabled());
    }

    #[test]
    fn test_dynamic_tabs_get_dm_tab_mut() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let dm = tabs.get_dm_tab_mut("peer1");
        assert!(dm.is_some());
        dm.unwrap().messages.push_back("Hello".to_string());
        let dm2 = tabs.get_dm_tab("peer1");
        assert_eq!(dm2.unwrap().messages.len(), 1);
    }

    #[test]
    fn test_dynamic_tabs_total_tab_count() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.total_tab_count(), 3);
        tabs.add_dm_tab("peer1".to_string());
        assert_eq!(tabs.total_tab_count(), 4);
    }

    #[test]
    fn test_dynamic_tabs_tab_index_to_content() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.tab_index_to_content(0), TabContent::Chat);
        assert_eq!(tabs.tab_index_to_content(1), TabContent::Peers);
        tabs.add_dm_tab("peer1".to_string());
        assert_eq!(
            tabs.tab_index_to_content(2),
            TabContent::Direct("peer1".to_string())
        );
        assert_eq!(tabs.tab_index_to_content(3), TabContent::Log);
    }

    #[test]
    fn test_remove_dm_tab_nonexistent() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.remove_dm_tab("nobody"), None);
    }

    #[test]
    fn test_get_dm_tab_nonexistent() {
        let tabs = DynamicTabs::new();
        assert_eq!(tabs.get_dm_tab("nobody"), None);
    }

    #[test]
    fn test_get_dm_tab_mut_nonexistent() {
        let mut tabs = DynamicTabs::new();
        assert_eq!(tabs.get_dm_tab_mut("nobody"), None);
    }

    #[test]
    fn test_add_dm_tab_existing_peer() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        let idx = tabs.add_dm_tab("peer1".to_string());
        assert_eq!(idx, 2);
        assert_eq!(tabs.dm_tab_count(), 1);
    }

    #[test]
    fn test_tab_index_to_content_multiple_dms() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        tabs.add_dm_tab("peer2".to_string());
        assert_eq!(
            tabs.tab_index_to_content(2),
            TabContent::Direct("peer1".to_string())
        );
        assert_eq!(
            tabs.tab_index_to_content(3),
            TabContent::Direct("peer2".to_string())
        );
        assert_eq!(tabs.tab_index_to_content(4), TabContent::Log);
    }

    #[test]
    fn test_tab_index_to_content_out_of_bounds_dm() {
        let mut tabs = DynamicTabs::new();
        tabs.add_dm_tab("peer1".to_string());
        // Index 3 is Log (2 + 1 DM = 3), index 4+ should be Chat
        assert_eq!(tabs.tab_index_to_content(5), TabContent::Chat);
    }

    #[test]
    fn test_all_titles_with_dms() {
        let mut tabs = DynamicTabs::new();
        let titles = tabs.all_titles();
        assert_eq!(titles, vec!["Chat", "Peers", "Log"]);
        tabs.add_dm_tab("peer1".to_string());
        let titles = tabs.all_titles();
        assert_eq!(titles.len(), 4);
        assert_eq!(titles[0], "Chat");
        assert_eq!(titles[1], "Peers");
        assert!(titles[2].contains("peer1"));
        assert_eq!(titles[3], "Log");
    }

    #[test]
    fn test_dynamic_tabs_default_is_empty() {
        let tabs = DynamicTabs::default();
        assert_eq!(tabs.dm_tab_count(), 0);
        assert_eq!(tabs.total_tab_count(), 3);
    }
