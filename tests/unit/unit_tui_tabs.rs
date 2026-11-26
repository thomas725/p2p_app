use super::*;

#[test]
fn test_group_tab_new() {
    let tab = GroupTab::new("abc123".to_string(), "Rust Devs".to_string());
    assert_eq!(tab.group_id, "abc123");
    assert_eq!(tab.display_name, "Rust Devs");
}

#[test]
fn test_dynamic_tabs_new() {
    let tabs = DynamicTabs::new();
    assert_eq!(tabs.dm_tab_count(), 0);
    assert_eq!(tabs.group_tab_count(), 0);
    assert_eq!(tabs.total_tab_count(), 6); // Chat, Peers, Groups, Log, Settings, Shares
}

#[test]
fn test_dynamic_tabs_add_group_tab() {
    let mut tabs = DynamicTabs::new();
    let idx = tabs.add_group_tab("g1".to_string(), "Group One".to_string());
    assert_eq!(idx, 3);
    assert_eq!(tabs.group_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_add_group_tab_dedup() {
    let mut tabs = DynamicTabs::new();
    let a = tabs.add_group_tab("g1".to_string(), "Group One".to_string());
    let b = tabs.add_group_tab("g1".to_string(), "Group One (again)".to_string());
    assert_eq!(a, b);
    assert_eq!(tabs.group_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_group_tab() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "Group One".to_string());
    let idx = tabs.remove_group_tab("g1");
    assert_eq!(idx, Some(3));
    assert_eq!(tabs.group_tab_count(), 0);
}

#[test]
fn test_dynamic_tabs_group_tab_titles() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "Rust Devs".to_string());
    let titles = tabs.group_tab_titles();
    assert_eq!(titles, vec!["Group: Rust Devs [X]"]);
}

#[test]
fn test_dynamic_tabs_get_group_tab() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "Rust Devs".to_string());
    let tab = tabs.get_group_tab("g1");
    assert!(tab.is_some());
    assert_eq!(tab.unwrap().display_name, "Rust Devs");
}

#[test]
fn test_dynamic_tabs_add_dm_tab() {
    let mut tabs = DynamicTabs::new();
    let idx = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx, 3);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_add_dm_after_group_tab() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "G".to_string());
    let idx = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx, 4); // Chat, Peers, Groups, Group chat, then DM
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_dm_tab() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer1".to_string());
    let idx = tabs.remove_dm_tab("peer1");
    assert_eq!(idx, Some(3));
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
    assert_eq!(
        TabContent::PeerInfo("peer1".to_string()).peer_id(),
        Some("peer1")
    );
    assert_eq!(TabContent::Chat.peer_id(), None);
    assert_eq!(TabContent::Contacts.peer_id(), None);
    assert_eq!(TabContent::Groups.peer_id(), None);
    assert_eq!(TabContent::GroupChat("g1".to_string()).peer_id(), None);
    assert_eq!(TabContent::Log.peer_id(), None);
    assert_eq!(TabContent::Settings.peer_id(), None);
    assert_eq!(TabContent::Shares.peer_id(), None);
}

#[test]
fn test_tab_content_is_input_enabled() {
    assert!(TabContent::Chat.is_input_enabled());
    assert!(TabContent::Direct("peer1".to_string()).is_input_enabled());
    assert!(TabContent::GroupChat("g1".to_string()).is_input_enabled());
    assert!(!TabContent::Contacts.is_input_enabled());
    assert!(!TabContent::Groups.is_input_enabled());
    assert!(!TabContent::Log.is_input_enabled());
    assert!(!TabContent::Settings.is_input_enabled());
    assert!(!TabContent::Shares.is_input_enabled());
}

#[test]
fn test_dynamic_tabs_total_tab_count() {
    let mut tabs = DynamicTabs::new();
    assert_eq!(tabs.total_tab_count(), 6); // +Shares
    tabs.add_dm_tab("peer1".to_string());
    assert_eq!(tabs.total_tab_count(), 7);
    tabs.add_group_tab("g1".to_string(), "G".to_string());
    assert_eq!(tabs.total_tab_count(), 8);
}

#[test]
fn test_dynamic_tabs_tab_index_to_content() {
    let mut tabs = DynamicTabs::new();
    assert_eq!(tabs.tab_index_to_content(0), TabContent::Chat);
    assert_eq!(tabs.tab_index_to_content(1), TabContent::Contacts);
    assert_eq!(tabs.tab_index_to_content(2), TabContent::Groups);
    tabs.add_dm_tab("peer1".to_string());
    assert_eq!(
        tabs.tab_index_to_content(3),
        TabContent::Direct("peer1".to_string())
    );
    assert_eq!(tabs.tab_index_to_content(4), TabContent::Log);
    assert_eq!(tabs.tab_index_to_content(5), TabContent::Settings);
    assert_eq!(tabs.tab_index_to_content(6), TabContent::Shares);
}

#[test]
fn test_tab_index_to_content_group_chat() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "Rust Devs".to_string());
    tabs.add_dm_tab("peer1".to_string());
    assert_eq!(tabs.tab_index_to_content(2), TabContent::Groups);
    assert_eq!(
        tabs.tab_index_to_content(3),
        TabContent::GroupChat("g1".to_string())
    );
    assert_eq!(
        tabs.tab_index_to_content(4),
        TabContent::Direct("peer1".to_string())
    );
    assert_eq!(tabs.tab_index_to_content(5), TabContent::Log);
    assert_eq!(tabs.tab_index_to_content(6), TabContent::Settings);
    assert_eq!(tabs.tab_index_to_content(7), TabContent::Shares);
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
fn test_add_dm_tab_existing_peer() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer1".to_string());
    let idx = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx, 3);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_tab_index_to_content_multiple_dms() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer1".to_string());
    tabs.add_dm_tab("peer2".to_string());
    assert_eq!(
        tabs.tab_index_to_content(3),
        TabContent::Direct("peer1".to_string())
    );
    assert_eq!(
        tabs.tab_index_to_content(4),
        TabContent::Direct("peer2".to_string())
    );
    assert_eq!(tabs.tab_index_to_content(5), TabContent::Log);
}

#[test]
fn test_tab_index_to_content_out_of_bounds_dm() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer1".to_string());
    // Indices: 0-2 fixed, 3 DM, 4 Log, 5 Settings, 6 Shares; index 7+ is Chat
    assert_eq!(tabs.tab_index_to_content(7), TabContent::Chat);
}

#[test]
fn test_dynamic_tabs_peer_info_tab() {
    let mut tabs = DynamicTabs::new();
    let idx = tabs.add_peer_info_tab("peer1".to_string());
    assert_eq!(idx, 3); // Chat, Peers, Groups, then Info
    assert_eq!(tabs.peer_info_tab_count(), 1);
    let titles = tabs.all_titles();
    assert_eq!(titles.len(), 7); // +Shares
    assert!(titles.iter().any(|t| t.starts_with("Info:")));
    assert_eq!(
        tabs.tab_index_to_content(idx),
        TabContent::PeerInfo("peer1".to_string())
    );
}

#[test]
fn test_dynamic_tabs_peer_info_tab_after_group_and_dm() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "G".to_string());
    tabs.add_dm_tab("peer1".to_string());
    let idx = tabs.add_peer_info_tab("peer1".to_string());
    assert_eq!(idx, 5); // Chat, Peers, Groups, Group chat, DM, then Info
    assert_eq!(
        tabs.tab_index_to_content(idx),
        TabContent::PeerInfo("peer1".to_string())
    );
}

#[test]
fn test_dynamic_tabs_peer_info_tab_dedup() {
    let mut tabs = DynamicTabs::new();
    let a = tabs.add_peer_info_tab("peer1".to_string());
    let b = tabs.add_peer_info_tab("peer1".to_string());
    assert_eq!(a, b);
    assert_eq!(tabs.peer_info_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_peer_info_tab() {
    let mut tabs = DynamicTabs::new();
    tabs.add_peer_info_tab("peer1".to_string());
    assert_eq!(tabs.remove_peer_info_tab("peer1"), Some(3));
    assert_eq!(tabs.peer_info_tab_count(), 0);
}

#[test]
fn test_remove_group_tab_nonexistent() {
    let mut tabs = DynamicTabs::new();
    assert_eq!(tabs.remove_group_tab("nobody"), None);
}

#[test]
fn test_remove_tab_group_chat() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "G".to_string());
    let idx = tabs.remove_tab(&TabContent::GroupChat("g1".to_string()));
    assert_eq!(idx, Some(3));
    assert_eq!(tabs.group_tab_count(), 0);
}

#[test]
fn test_all_titles_with_dms() {
    // The display-name cache is process-global while test DBs are per-thread.
    // A peer id shared with another unit test can be invalidated and re-resolved
    // against that other thread's DB between `all_titles` and the expectation
    // below, yielding a different generated petname. Use an id unique to this
    // test and pin a deterministic local nickname so cache clears are harmless.
    let peer_id = "tab-title-dm-peer";
    crate::set_peer_local_nickname(peer_id, "tab-title").unwrap();
    let mut tabs = DynamicTabs::new();
    let titles = tabs.all_titles();
    assert_eq!(
        titles,
        vec!["Chat", "Contacts", "Groups", "Log", "Settings", "Shares"]
    );
    tabs.add_dm_tab(peer_id.to_string());
    let titles = tabs.all_titles();
    assert_eq!(titles.len(), 7);
    assert_eq!(titles[0], "Chat");
    assert_eq!(titles[1], "Contacts");
    assert_eq!(titles[2], "Groups");
    let expected_label = crate::get_peer_display_name(peer_id)
        .unwrap_or_else(|_| crate::fmt::short_peer_id(peer_id));
    assert_eq!(titles[3], format!("{expected_label} [X]"));
    assert_eq!(titles[4], "Log");
    assert_eq!(titles[5], "Settings");
    assert_eq!(titles[6], "Shares");
}

#[test]
fn test_all_titles_with_group_chat() {
    let mut tabs = DynamicTabs::new();
    tabs.add_group_tab("g1".to_string(), "Rust Devs".to_string());
    let titles = tabs.all_titles();
    assert_eq!(
        titles,
        vec![
            "Chat",
            "Contacts",
            "Groups",
            "Group: Rust Devs [X]",
            "Log",
            "Settings",
            "Shares"
        ]
    );
}

#[test]
fn test_dynamic_tabs_default_is_empty() {
    let tabs = DynamicTabs::default();
    assert_eq!(tabs.dm_tab_count(), 0);
    assert_eq!(tabs.group_tab_count(), 0);
    assert_eq!(tabs.total_tab_count(), 6);
}
