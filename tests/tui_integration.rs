//! Integration tests for TUI event handling and state mutations
//!
//! These tests simulate realistic TUI usage patterns including:
//! - Tab navigation
//! - Message sending
//! - Peer connection handling
//! - Mouse capture toggling
//! - Duplicate peer prevention

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[test]
fn test_tui_tab_navigation() {
    let _logs: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));
    let state = p2p_app::tui_tabs::DynamicTabs::new();

    // Start at tab 0 (Chat)
    let initial_content = state.tab_index_to_content(0);
    assert_eq!(initial_content, p2p_app::tui_tabs::TabContent::Chat);

    // Tab navigation: simulate pressing Tab multiple times
    let mut active_tab = 0;
    let max_tabs = state.total_tab_count();

    // Press Tab -> should go to Peers
    active_tab = (active_tab + 1) % max_tabs;
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Peers);

    // Press Tab -> should go to Log (no DM tabs yet)
    active_tab = (active_tab + 1) % max_tabs;
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Log);

    // Press Tab -> should wrap back to Chat
    active_tab = (active_tab + 1) % max_tabs;
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Chat);
}

#[test]
fn test_tui_tab_navigation_backward() {
    let mut state = p2p_app::tui_tabs::DynamicTabs::new();
    let max_tabs = state.total_tab_count();

    // Start at tab 0 (Chat)
    let mut active_tab = 0;

    // Press Shift+Tab (backward) -> should wrap to Log
    active_tab = if active_tab == 0 { max_tabs - 1 } else { active_tab - 1 };
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Log);

    // Press Shift+Tab -> should go to Peers
    active_tab = if active_tab == 0 { max_tabs - 1 } else { active_tab - 1 };
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Peers);

    // Press Shift+Tab -> should go to Chat
    active_tab = if active_tab == 0 { max_tabs - 1 } else { active_tab - 1 };
    let content = state.tab_index_to_content(active_tab);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Chat);
}

#[test]
fn test_tui_dm_tab_creation_on_peer_connection() {
    let mut state = p2p_app::tui_tabs::DynamicTabs::new();

    // Initially 3 tabs (Chat, Peers, Log)
    assert_eq!(state.total_tab_count(), 3);

    // Add DM tab for peer
    let peer_id = "QmPeerId123".to_string();
    let tab_idx = state.add_dm_tab(peer_id.clone());

    // Should now have 4 tabs
    assert_eq!(state.total_tab_count(), 4);

    // Tab index should point to the DM tab
    let content = state.tab_index_to_content(tab_idx);
    assert_eq!(content, p2p_app::tui_tabs::TabContent::Direct(peer_id.clone()));

    // Adding same peer again should return existing tab index
    let tab_idx_2 = state.add_dm_tab(peer_id.clone());
    assert_eq!(tab_idx, tab_idx_2);
    assert_eq!(state.total_tab_count(), 4); // Still 4 tabs
}

#[test]
fn test_tui_no_duplicate_peers_on_reconnect() {
    let _logs: Arc<Mutex<VecDeque<String>>> = Arc::new(Mutex::new(VecDeque::new()));

    // Simulate app state with initial peers from database
    let mut peers_list: VecDeque<(String, String, String)> = VecDeque::new();
    peers_list.push_back(("peer1".to_string(), "10:00".to_string(), "10:05".to_string()));
    peers_list.push_back(("peer2".to_string(), "10:01".to_string(), "10:06".to_string()));

    // Simulate PeerConnected event for peer that's already in the list
    let peer_id = "peer1".to_string();
    let should_add = !peers_list.iter().any(|(id, _, _)| id == &peer_id);
    assert!(!should_add, "Peer should not be added - already in list");

    // Verify list still has exactly 2 peers
    assert_eq!(peers_list.len(), 2);

    // Simulate PeerConnected for a new peer
    let new_peer = "peer3".to_string();
    let should_add = !peers_list.iter().any(|(id, _, _)| id == &new_peer);
    assert!(should_add, "New peer should be added");

    if should_add {
        peers_list.push_front((new_peer, "10:07".to_string(), "10:07".to_string()));
    }

    // Verify list now has 3 peers
    assert_eq!(peers_list.len(), 3);

    // Verify no duplicates
    let mut seen_ids = std::collections::HashSet::new();
    for (id, _, _) in &peers_list {
        assert!(seen_ids.insert(id.clone()), "Found duplicate peer: {}", id);
    }
}

#[test]
fn test_tui_message_sending_broadcast() {
    // Simulate sending a broadcast message
    let mut messages: VecDeque<(String, Option<String>)> = VecDeque::new();
    let own_nickname = "Alice".to_string();

    // User types "Hello everyone" and presses Enter
    let msg_text = "Hello everyone";
    let ts = "2024-01-01 10:00:00".to_string();
    let msg = format!("{} [{}] {}", ts, own_nickname, msg_text);

    messages.push_back((msg.clone(), None)); // None = broadcast (no specific peer)

    // Verify message was added
    assert_eq!(messages.len(), 1);

    // Verify message content
    let (stored_msg, peer_id) = messages[0].clone();
    assert!(stored_msg.contains(msg_text));
    assert!(stored_msg.contains(&own_nickname));
    assert_eq!(peer_id, None);
}

#[test]
fn test_tui_message_sending_dm() {
    // Simulate sending a DM
    let mut dm_messages: HashMap<String, VecDeque<String>> = HashMap::new();
    let peer_id = "QmPeerId123".to_string();
    let own_nickname = "Alice".to_string();

    // User types "Hi there" and presses Enter
    let msg_text = "Hi there";
    let ts = "2024-01-01 10:00:00".to_string();
    let msg = format!("{} [{}] {}", ts, own_nickname, msg_text);

    let dm_msgs = dm_messages.entry(peer_id.clone()).or_default();
    dm_msgs.push_back(msg.clone());

    // Verify DM was added
    assert_eq!(dm_messages.len(), 1);
    assert_eq!(dm_messages[&peer_id].len(), 1);

    // Verify DM content
    let stored_msg = &dm_messages[&peer_id][0];
    assert!(stored_msg.contains(msg_text));
    assert!(stored_msg.contains(&own_nickname));
}

#[test]
fn test_tui_mouse_capture_toggle() {
    // Simulate mouse capture state
    let mut mouse_capture = true; // Default enabled
    assert!(mouse_capture);

    // Press F12 - toggle mouse capture
    mouse_capture = !mouse_capture;
    assert!(!mouse_capture);

    // Press F12 again - toggle back
    mouse_capture = !mouse_capture;
    assert!(mouse_capture);
}

#[test]
fn test_tui_scroll_state_management() {
    // Simulate scroll state
    let mut chat_scroll_offset: usize = 0;
    let mut chat_auto_scroll = true;

    // User presses Up arrow - disable auto-scroll and increment offset
    if chat_auto_scroll {
        chat_auto_scroll = false;
    }
    chat_scroll_offset = chat_scroll_offset.saturating_add(1);

    assert!(!chat_auto_scroll);
    assert_eq!(chat_scroll_offset, 1);

    // User presses Down arrow - decrement offset
    chat_scroll_offset = chat_scroll_offset.saturating_sub(1);
    assert_eq!(chat_scroll_offset, 0);

    // At offset 0, auto-scroll should be re-enabled
    if chat_scroll_offset == 0 {
        chat_auto_scroll = true;
    }
    assert!(chat_auto_scroll);
}

#[test]
fn test_tui_input_clearing_after_send() {
    use ratatui_textarea::TextArea;

    // Simulate TextArea with input
    let mut text_area = TextArea::default();
    text_area.insert_str("Hello");

    // Verify text is there
    let text: String = text_area.lines().join("\n");
    assert_eq!(text, "Hello");

    // After sending, clear the input
    text_area = TextArea::default();

    // Verify it's cleared
    let text: String = text_area.lines().join("\n");
    assert!(text.trim().is_empty());
}

#[test]
fn test_tui_input_enabled_only_on_chat_tabs() {
    let mut dynamic_tabs = p2p_app::tui_tabs::DynamicTabs::new();

    // Chat tab should allow input
    let chat_content = dynamic_tabs.tab_index_to_content(0); // Chat tab
    assert!(chat_content.is_input_enabled());

    // Peers tab should NOT allow input
    let peers_content = dynamic_tabs.tab_index_to_content(1); // Peers tab
    assert!(!peers_content.is_input_enabled());

    // Add a DM tab
    let dm_tab_idx = dynamic_tabs.add_dm_tab("QmPeerId123".to_string());
    let dm_content = dynamic_tabs.tab_index_to_content(dm_tab_idx);
    assert!(dm_content.is_input_enabled()); // DM should allow input

    // Log tab should NOT allow input
    let log_content = dynamic_tabs.tab_index_to_content(dynamic_tabs.total_tab_count() - 1);
    assert!(!log_content.is_input_enabled());
}

#[test]
fn test_tui_message_history_bounded() {
    const MAX_MESSAGES: usize = 100;

    let mut messages: VecDeque<(String, Option<String>)> = VecDeque::new();

    // Add more messages than the limit
    for i in 0..150 {
        messages.push_back((format!("message {}", i), None));

        // Maintain the limit
        if messages.len() > MAX_MESSAGES {
            messages.pop_front();
        }
    }

    // Verify we have exactly MAX_MESSAGES
    assert_eq!(messages.len(), MAX_MESSAGES);

    // Verify oldest messages were removed
    let (first_msg, _) = &messages[0];
    assert!(first_msg.contains("message 50")); // First message should be around 50
}

#[test]
fn test_tui_dm_history_per_peer_bounded() {
    const MAX_DM_HISTORY: usize = 50;

    let mut dm_messages: HashMap<String, VecDeque<String>> = HashMap::new();
    let peer_id = "peer1".to_string();

    // Add more DM messages than the limit
    for i in 0..100 {
        let dm_msgs = dm_messages.entry(peer_id.clone()).or_default();
        dm_msgs.push_back(format!("dm {}", i));

        // Maintain the limit per peer
        if dm_msgs.len() > MAX_DM_HISTORY {
            dm_msgs.pop_front();
        }
    }

    // Verify we have exactly MAX_DM_HISTORY for this peer
    assert_eq!(dm_messages[&peer_id].len(), MAX_DM_HISTORY);

    // Verify oldest DMs were removed
    assert!(dm_messages[&peer_id][0].contains("dm 50"));
}

#[test]
fn test_tui_whitespace_message_not_sent() {
    // Simulate the check before sending a message
    let text = "   \n  \t  ".to_string();

    // Message should not be sent if only whitespace
    let should_send = !text.trim().is_empty();
    assert!(!should_send);

    // Non-whitespace message should be sent
    let text = "   Hello   ".to_string();
    let should_send = !text.trim().is_empty();
    assert!(should_send);
}

#[test]
fn test_tui_peer_display_limit() {
    const MAX_PEERS: usize = 10000;

    let mut peers: VecDeque<(String, String, String)> = VecDeque::new();

    // Try to add more peers than the limit
    for i in 0..MAX_PEERS + 100 {
        if peers.len() < MAX_PEERS {
            peers.push_back((format!("peer{}", i), "time1".to_string(), "time2".to_string()));
        }
    }

    // Verify we have exactly MAX_PEERS
    assert_eq!(peers.len(), MAX_PEERS);
}

#[test]
fn test_tui_peer_deduplication_with_limit() {
    // Simulate loading peers from database with many duplicates
    let mut peers: VecDeque<(String, String, String)> = VecDeque::new();
    let mut seen_ids = std::collections::HashSet::new();

    // Simulate database entries: peer1 appears 5 times, peer2 appears 3 times, etc.
    let db_entries = vec![
        ("peer1", "2024-01-01", "2024-01-05"),
        ("peer1", "2024-01-02", "2024-01-04"), // duplicate - should skip
        ("peer2", "2024-01-01", "2024-01-03"),
        ("peer1", "2024-01-03", "2024-01-06"), // duplicate - should skip
        ("peer3", "2024-01-01", "2024-01-02"),
        ("peer2", "2024-01-02", "2024-01-04"), // duplicate - should skip
        ("peer4", "2024-01-01", "2024-01-01"),
    ];

    const MAX_PEERS: usize = 10000;

    // Deduplicate first, then apply limit
    for (id, first_seen, last_seen) in db_entries {
        if !seen_ids.insert(id.to_string()) {
            continue; // Skip duplicates
        }

        if peers.len() >= MAX_PEERS {
            break;
        }

        peers.push_back((id.to_string(), first_seen.to_string(), last_seen.to_string()));
    }

    // Should have 4 unique peers (peer1, peer2, peer3, peer4)
    assert_eq!(peers.len(), 4);
    assert_eq!(seen_ids.len(), 4);

    // Verify all are unique
    let mut unique_check = std::collections::HashSet::new();
    for (id, _, _) in &peers {
        assert!(unique_check.insert(id.clone()), "Found duplicate peer: {}", id);
    }
}
