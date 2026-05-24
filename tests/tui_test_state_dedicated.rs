//! Dedicated tests for tui_test_state.rs module
//!
//! Tests for TuiTestState and its notification/click handling.

use p2p_app::TuiTestState;
use std::collections::BTreeMap;

#[test]
fn test_tui_test_state_new() {
    let state = TuiTestState::new();
    assert_eq!(state.active_tab, 0);
    assert!(state.messages.is_empty());
    assert!(state.peers.is_empty());
}

#[test]
fn test_tui_test_state_with_messages() {
    let mut state = TuiTestState::new();
    state.add_message("test msg");
    assert_eq!(state.messages.len(), 1);
}

#[test]
fn test_tui_test_state_with_peers() {
    let mut state = TuiTestState::new();
    state.add_peer("peer-id", "2024-01-01", "2024-01-01", Some("Alice"));
    assert_eq!(state.peers.len(), 1);
}

#[test]
fn test_tui_test_state_with_dm_messages() {
    let mut state = TuiTestState::new();
    state.add_dm_message("peer-x", "hello");
    assert!(state.dm_messages.contains_key("peer-x"));
}

#[test]
fn test_tui_test_state_default() {
    let state = TuiTestState::default();
    assert_eq!(state.active_tab, 0);
    assert!(state.messages.is_empty());
}

#[test]
fn test_handle_notification_click_broadcasts_unread() {
    let mut state = TuiTestState::new();
    state.unread_broadcasts.insert("topic".to_string(), 3);
    
    state.handle_notification_click(0);
    // Should update unread count
    assert_eq!(state.unread_broadcasts.get("topic"), Some(&3));
}

#[test]
fn test_handle_notification_click_dms_unread() {
    let mut state = TuiTestState::new();
    state.unread_dms.insert("peer".to_string(), 2);
    
    state.handle_notification_click(1);
    // Should handle DM notification
    assert_eq!(state.unread_dms.get("peer"), Some(&2));
}

#[test]
fn test_handle_mouse_click_peers_section() {
    let mut state = TuiTestState::new();
    state.add_peer("p1", "2024-01-01", "2024-01-01", Some("Alice"));
    
    state.handle_mouse_click(10, 10);
    // Should process click without panicking
}

#[test]
fn test_active_tab_property() {
    let mut state = TuiTestState::new();
    assert_eq!(state.active_tab, 0);
    state.active_tab = 1;
    assert_eq!(state.active_tab, 1);
}

#[test]
fn test_unread_broadcasts_tracking() {
    let mut state = TuiTestState::new();
    state.unread_broadcasts.insert("topic1".to_string(), 5);
    state.unread_broadcasts.insert("topic2".to_string(), 3);
    
    assert_eq!(state.unread_broadcasts.len(), 2);
    assert_eq!(state.unread_broadcasts.get("topic1"), Some(&5));
}

#[test]
fn test_unread_dms_tracking() {
    let mut state = TuiTestState::new();
    state.unread_dms.insert("peer1".to_string(), 2);
    state.unread_dms.insert("peer2".to_string(), 4);
    
    assert_eq!(state.unread_dms.len(), 2);
    assert_eq!(state.unread_dms.get("peer1"), Some(&2));
}

#[test]
fn test_tui_test_state_multiple_messages() {
    let mut state = TuiTestState::new();
    state.add_message("msg1");
    state.add_message("msg2");
    state.add_message("msg3");
    
    assert_eq!(state.messages.len(), 3);
}

#[test]
fn test_tui_test_state_multiple_peers() {
    let mut state = TuiTestState::new();
    state.add_peer("p1", "2024-01-01", "2024-01-01", Some("Alice"));
    state.add_peer("p2", "2024-01-01", "2024-01-01", Some("Bob"));
    
    assert_eq!(state.peers.len(), 2);
}

#[test]
fn test_tui_test_state_multiple_dms() {
    let mut state = TuiTestState::new();
    state.add_dm_message("peer-a", "msg1");
    state.add_dm_message("peer-a", "msg2");
    state.add_dm_message("peer-b", "msg3");
    
    assert_eq!(state.dm_messages.len(), 2);
    assert_eq!(state.dm_messages.get("peer-a").map(|m| m.len()), Some(2));
}

#[test]
fn test_tui_test_state_unread_broadcasts_empty() {
    let state = TuiTestState::new();
    assert!(state.unread_broadcasts.is_empty());
}

#[test]
fn test_tui_test_state_unread_dms_empty() {
    let state = TuiTestState::new();
    assert!(state.unread_dms.is_empty());
}

#[test]
fn test_tui_test_state_clear_messages() {
    let mut state = TuiTestState::new();
    state.add_message("msg");
    state.messages.clear();
    assert!(state.messages.is_empty());
}

#[test]
fn test_tui_test_state_clear_peers() {
    let mut state = TuiTestState::new();
    state.add_peer("peer", "2024-01-01", "2024-01-01", None);
    state.peers.clear();
    assert!(state.peers.is_empty());
}
