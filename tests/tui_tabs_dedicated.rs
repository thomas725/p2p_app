//! Dedicated tests for tui_tabs.rs module
//!
//! Tests for tab management, DynamicTabs, and TabContent structures.

use p2p_app::tui_tabs::{DmTab, DynamicTabs, TabContent, TabId};

#[test]
fn test_tab_id_display() {
    assert_eq!(format!("{}", TabId::Chat), "Chat");
    assert_eq!(format!("{}", TabId::Peers), "Peers");
    assert_eq!(format!("{}", TabId::Log), "Log");
}

#[test]
fn test_dynamic_tabs_new() {
    let tabs = DynamicTabs::new();
    assert_eq!(tabs.dm_tab_count(), 0);
}

#[test]
fn test_dynamic_tabs_add_single() {
    let mut tabs = DynamicTabs::new();
    let idx = tabs.add_dm_tab("peer1".to_string());
    assert_eq!(idx, 3); // Chat=0, Peers=1, Log=2, first DM=3
}

#[test]
fn test_dynamic_tabs_add_multiple_unique() {
    let mut tabs = DynamicTabs::new();
    let idx1 = tabs.add_dm_tab("peer-a".to_string());
    let idx2 = tabs.add_dm_tab("peer-b".to_string());
    let idx3 = tabs.add_dm_tab("peer-c".to_string());

    assert_eq!(idx1, 3);
    assert_eq!(idx2, 4);
    assert_eq!(idx3, 5);
    assert_eq!(tabs.dm_tab_count(), 3);
}

#[test]
fn test_dynamic_tabs_add_duplicate_deduplicates() {
    let mut tabs = DynamicTabs::new();
    let idx1 = tabs.add_dm_tab("same-peer".to_string());
    let idx2 = tabs.add_dm_tab("same-peer".to_string());

    assert_eq!(idx1, idx2);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_existing() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("peer-remove".to_string());
    tabs.add_dm_tab("peer-keep".to_string());

    assert_eq!(tabs.dm_tab_count(), 2);
    let removed = tabs.remove_dm_tab("peer-remove");
    assert!(removed);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_dynamic_tabs_remove_nonexistent() {
    let mut tabs = DynamicTabs::new();
    let removed = tabs.remove_dm_tab("does-not-exist");
    assert!(!removed);
    assert_eq!(tabs.dm_tab_count(), 0);
}

#[test]
fn test_dynamic_tabs_remove_from_multiple() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("p1".to_string());
    tabs.add_dm_tab("p2".to_string());
    tabs.add_dm_tab("p3".to_string());

    tabs.remove_dm_tab("p2");
    assert_eq!(tabs.dm_tab_count(), 2);

    // p1 and p3 should still be there
    tabs.remove_dm_tab("p1");
    assert_eq!(tabs.dm_tab_count(), 1);

    tabs.remove_dm_tab("p3");
    assert_eq!(tabs.dm_tab_count(), 0);
}

#[test]
fn test_dm_tab_new() {
    let tab = DmTab::new("peer-id".to_string());
    assert_eq!(tab.peer_id, "peer-id");
}

#[test]
fn test_dm_tab_clone() {
    let tab1 = DmTab::new("peer".to_string());
    let tab2 = tab1.clone();
    assert_eq!(tab1.peer_id, tab2.peer_id);
}

#[test]
fn test_dm_tab_equality() {
    let tab1 = DmTab::new("peer".to_string());
    let tab2 = DmTab::new("peer".to_string());
    let tab3 = DmTab::new("other".to_string());

    assert_eq!(tab1, tab2);
    assert_ne!(tab1, tab3);
}

#[test]
fn test_tab_content_broadcast() {
    let content = TabContent::Broadcast(vec![
        ("msg1".to_string(), false),
        ("msg2".to_string(), true),
    ]);
    match content {
        TabContent::Broadcast(msgs) => assert_eq!(msgs.len(), 2),
        _ => panic!("Expected Broadcast variant"),
    }
}

#[test]
fn test_tab_content_peers() {
    let content = TabContent::Peers(vec![("peer-id".to_string(), "peer-name".to_string())]);
    match content {
        TabContent::Peers(peers) => assert_eq!(peers.len(), 1),
        _ => panic!("Expected Peers variant"),
    }
}

#[test]
fn test_tab_content_log() {
    let content = TabContent::Log(vec!["log line 1".to_string(), "log line 2".to_string()]);
    match content {
        TabContent::Log(logs) => assert_eq!(logs.len(), 2),
        _ => panic!("Expected Log variant"),
    }
}

#[test]
fn test_tab_content_dm() {
    let content = TabContent::Dm(vec![("sender".to_string(), "message".to_string(), false)]);
    match content {
        TabContent::Dm(msgs) => assert_eq!(msgs.len(), 1),
        _ => panic!("Expected Dm variant"),
    }
}

#[test]
fn test_dynamic_tabs_clear_and_readd() {
    let mut tabs = DynamicTabs::new();
    tabs.add_dm_tab("p1".to_string());
    tabs.add_dm_tab("p2".to_string());

    tabs.remove_dm_tab("p1");
    tabs.remove_dm_tab("p2");

    // Should be able to add peers again
    let idx = tabs.add_dm_tab("p1".to_string());
    assert!(idx > 0);
    assert_eq!(tabs.dm_tab_count(), 1);
}

#[test]
fn test_tab_content_broadcast_empty() {
    let content = TabContent::Broadcast(vec![]);
    match content {
        TabContent::Broadcast(msgs) => assert!(msgs.is_empty()),
        _ => panic!("Expected Broadcast variant"),
    }
}

#[test]
fn test_tab_id_clone_equality() {
    let id1 = TabId::Chat;
    let id2 = id1.clone();
    assert_eq!(id1, id2);
}

#[test]
fn test_tab_id_from_index_boundary() {
    assert_eq!(TabId::from_index(0), TabId::Chat);
    assert_eq!(TabId::from_index(1), TabId::Peers);
    assert_eq!(TabId::from_index(2), TabId::Log);
}
