#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]
#![allow(clippy::used_underscore_binding, clippy::significant_drop_tightening)]

use super::*;
use crate::tui::test_helpers::test_app_state;
use p2p_app::{MessageEvent, PeerRecord, SwarmEvent};
use std::collections::VecDeque;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Mutex, mpsc};

// ── sort_peers_by_last_seen ──────────────────────────────────────────────

#[test]
fn test_sort_peers_empty() {
    let mut state = test_app_state();
    sort_peers_by_last_seen(&mut state);
    assert!(state.peers.is_empty());
}

#[test]
fn test_sort_peers_sorts_by_last_seen_desc() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "old".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "new".to_string(),
        first_seen: "2024-06-01 12:00:00".into(),
        last_seen: "2024-06-01 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "mid".to_string(),
        first_seen: "2024-03-01 12:00:00".into(),
        last_seen: "2024-03-01 12:00:00".into(),
    });
    sort_peers_by_last_seen(&mut state);
    let ids: Vec<&str> = state.peers.iter().map(|p| p.peer_id.as_str()).collect();
    assert_eq!(ids, vec!["new", "mid", "old"]);
}

#[test]
fn test_sort_peers_same_last_seen() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "a".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "b".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    sort_peers_by_last_seen(&mut state);
    // Stable sort preserves original order for equal last_seen
    let ids: Vec<&str> = state.peers.iter().map(|p| p.peer_id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b"]);
}

// ── upsert_peer_last_seen ────────────────────────────────────────────────

#[test]
fn test_upsert_peer_adds_new_peer() {
    let mut state = test_app_state();
    let seen =
        chrono::NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S").unwrap();
    upsert_peer_last_seen(&mut state, "peer-new", seen);
    assert_eq!(state.peers.len(), 1);
    assert_eq!(state.peers[0].peer_id, "peer-new");
    assert_eq!(state.peers[0].last_seen, "2024-06-15 14:30:00");
}

#[test]
fn test_upsert_peer_updates_existing() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "peer-exist".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    let later =
        chrono::NaiveDateTime::parse_from_str("2024-12-01 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    upsert_peer_last_seen(&mut state, "peer-exist", later);
    assert_eq!(state.peers.len(), 1);
    assert_eq!(state.peers[0].last_seen, "2024-12-01 18:00:00");
}

#[test]
fn test_upsert_peer_moves_to_front_when_latest() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "old".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "new".to_string(),
        first_seen: "2024-06-01 12:00:00".into(),
        last_seen: "2024-06-01 12:00:00".into(),
    });
    let later =
        chrono::NaiveDateTime::parse_from_str("2024-12-01 18:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    upsert_peer_last_seen(&mut state, "old", later);
    // "old" should now be first (latest last_seen)
    assert_eq!(state.peers[0].peer_id, "old");
    assert_eq!(state.peers[0].last_seen, "2024-12-01 18:00:00");
}

#[test]
fn test_upsert_peer_updates_peer_selection() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "a".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    state.peers.push_back(PeerRecord {
        peer_id: "b".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    assert_eq!(state.peer_selection, 0);
    let seen =
        chrono::NaiveDateTime::parse_from_str("2024-06-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    upsert_peer_last_seen(&mut state, "a", seen);
    // "a" moves to front but peer_selection should still point to it
    assert_eq!(state.peer_selection, 0);
}

// ── apply_broadcast_to_state ─────────────────────────────────────────────

#[test]
fn test_apply_broadcast_to_state_adds_message() {
    let mut state = test_app_state();
    state
        .local_nicknames
        .insert("p1".to_string(), "Alice".to_string());
    let msg = apply_broadcast_to_state(&mut state, "hello", "p1", None, Some("m1".to_string()));
    assert_eq!(state.messages.len(), 1);
    assert!(msg.contains("Alice"));
    assert!(msg.contains("hello"));
    assert_eq!(state.message_ids[0], Some("m1".to_string()));
    assert_eq!(state.messages[0].sender_peer_id, Some("p1".to_string()));
}

#[test]
fn test_apply_broadcast_to_state_trims_history() {
    let mut state = test_app_state();
    // Override MAX_MESSAGE_HISTORY by filling messages up to 2 below the limit
    for _ in 0..MAX_MESSAGE_HISTORY.saturating_sub(2) {
        state.messages.push_back(DisplayMessage {
            text: "old".to_string(),
            sender_peer_id: None,
        });
        state.message_ids.push_back(None);
    }
    // Add 3 more to trigger trimming
    apply_broadcast_to_state(&mut state, "a", "p1", None, None);
    apply_broadcast_to_state(&mut state, "b", "p1", None, None);
    apply_broadcast_to_state(&mut state, "c", "p1", None, None);
    assert!(state.messages.len() <= MAX_MESSAGE_HISTORY);
    assert!(state.messages.len() >= MAX_MESSAGE_HISTORY.saturating_sub(1));
}

#[test]
fn test_apply_broadcast_to_state_with_latency() {
    let mut state = test_app_state();
    let msg = apply_broadcast_to_state(&mut state, "ping", "p1", Some("10ms"), None);
    assert!(msg.contains("10ms"));
}

#[test]
fn test_apply_broadcast_increments_unread_when_scrolled_up() {
    let mut state = test_app_state();
    state.auto_scroll.chat = false;
    state.chat_unread_count = 0;
    apply_broadcast_to_state(&mut state, "a", "p1", None, None);
    apply_broadcast_to_state(&mut state, "b", "p1", None, None);
    assert_eq!(state.chat_unread_count, 2);
}

#[test]
fn test_apply_broadcast_no_unread_when_at_bottom() {
    let mut state = test_app_state();
    state.auto_scroll.chat = true;
    state.chat_unread_count = 0;
    apply_broadcast_to_state(&mut state, "a", "p1", None, None);
    assert_eq!(state.chat_unread_count, 0);
}

// ── apply_dm_to_state ────────────────────────────────────────────────────

#[test]
fn test_apply_dm_to_state_adds_message_and_tab() {
    let mut state = test_app_state();
    state
        .received_nicknames
        .insert("p2".to_string(), "Bob".to_string());
    let dm_count_before = state.dynamic_tabs.dm_tab_count();
    let msg = apply_dm_to_state(&mut state, "hey", "p2", None, Some("dm1".to_string()));
    assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
    assert!(state.dm_messages.contains_key("p2"));
    assert_eq!(state.dm_messages["p2"].len(), 1);
    assert!(msg.contains("Bob"));
    assert!(msg.contains("hey"));
    assert_eq!(state.dm_message_ids["p2"][0], Some("dm1".to_string()));
}

#[test]
fn test_apply_dm_to_state_trims_history() {
    let mut state = test_app_state();
    for i in 0..MAX_DM_HISTORY {
        apply_dm_to_state(&mut state, &format!("msg-{i}"), "peer-dm", None, None);
    }
    assert_eq!(state.dm_messages["peer-dm"].len(), MAX_DM_HISTORY);
    apply_dm_to_state(&mut state, "overflow", "peer-dm", None, None);
    assert_eq!(state.dm_messages["peer-dm"].len(), MAX_DM_HISTORY);
    // First message should have been "msg-0" but it was popped; now it's "msg-1"
    assert!(state.dm_messages["peer-dm"][0].contains("msg-1"));
    // Last message should be "overflow"
    assert!(state.dm_messages["peer-dm"][MAX_DM_HISTORY - 1].contains("overflow"));
}

#[test]
fn test_apply_dm_to_state_multiple_peers() {
    let mut state = test_app_state();
    apply_dm_to_state(&mut state, "hi", "pa", None, None);
    apply_dm_to_state(&mut state, "ho", "pb", None, None);
    assert_eq!(state.dm_messages.len(), 2);
    assert_eq!(state.dm_messages["pa"].len(), 1);
    assert_eq!(state.dm_messages["pb"].len(), 1);
}

// ── is_dm_receipt ────────────────────────────────────────────────────────

#[test]
fn test_is_dm_receipt_true_when_msg_id_in_dm() {
    let mut state = test_app_state();
    state.dm_message_ids.insert(
        "p1".to_string(),
        VecDeque::from([Some("dm-1".to_string()), Some("dm-2".to_string())]),
    );
    assert!(is_dm_receipt(&state, "dm-1"));
    assert!(is_dm_receipt(&state, "dm-2"));
}

#[test]
fn test_is_dm_receipt_false_when_not_found() {
    let state = test_app_state();
    assert!(!is_dm_receipt(&state, "nonexistent"));
}

#[test]
fn test_is_dm_receipt_false_for_broadcast_msg_id() {
    let mut state = test_app_state();
    state.message_ids.push_back(Some("bcast-1".to_string()));
    assert!(!is_dm_receipt(&state, "bcast-1"));
}

// ── apply_receipt_to_state ───────────────────────────────────────────────

#[test]
fn test_apply_receipt_to_state_dm() {
    let mut state = test_app_state();
    apply_receipt_to_state(&mut state, "dm-1", "peer-a", 100.0, true);
    assert_eq!(
        state.dm_receipts.get("dm-1"),
        Some(&("peer-a".to_string(), 100.0))
    );
    // Also stored as broadcast receipt
    assert_eq!(
        state.broadcast_receipts.get("dm-1").unwrap().get("peer-a"),
        Some(&100.0)
    );
}

#[test]
fn test_apply_receipt_to_state_broadcast() {
    let mut state = test_app_state();
    apply_receipt_to_state(&mut state, "b-1", "peer-b", 200.0, false);
    assert!(state.dm_receipts.is_empty());
    assert_eq!(
        state.broadcast_receipts.get("b-1").unwrap().get("peer-b"),
        Some(&200.0)
    );
}

#[test]
fn test_apply_receipt_to_state_multiple_peers() {
    let mut state = test_app_state();
    apply_receipt_to_state(&mut state, "b-1", "pa", 10.0, false);
    apply_receipt_to_state(&mut state, "b-1", "pb", 20.0, false);
    assert_eq!(state.broadcast_receipts.get("b-1").unwrap().len(), 2);
}

// ── apply_peer_connected_count ───────────────────────────────────────────

#[test]
fn test_apply_peer_connected_count_increments() {
    let mut state = test_app_state();
    assert_eq!(apply_peer_connected_count(&mut state), 1);
    assert_eq!(apply_peer_connected_count(&mut state), 2);
    assert_eq!(state.concurrent_peers, 2);
}

// ── apply_peer_disconnected_count ────────────────────────────────────────

#[test]
fn test_apply_peer_disconnected_count_decrements() {
    let mut state = test_app_state();
    state.concurrent_peers = 3;
    assert_eq!(apply_peer_disconnected_count(&mut state), 2);
    assert_eq!(state.concurrent_peers, 2);
}

#[test]
fn test_apply_peer_disconnected_count_saturates_at_zero() {
    let mut state = test_app_state();
    assert_eq!(apply_peer_disconnected_count(&mut state), 0);
}

// ── add_peer_to_state_list ───────────────────────────────────────────────

#[test]
fn test_add_peer_to_state_list_appends_and_sorts() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "old".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    add_peer_to_state_list(
        &mut state,
        "new",
        "2024-06-01 12:00:00",
        "2024-06-01 12:00:00",
    );
    assert_eq!(state.peers.len(), 2);
    // The newest contact resolves to the top of the Contacts view (the live
    // peer deque no longer reorders in place — the view sorts the derived
    // contact rows through `sort_canonical_rows`).
    assert_eq!(state.contacts[0].contact_id, "new"); // newest first
}

#[test]
fn test_add_peer_to_state_list_skips_local_peer() {
    let mut state = test_app_state();
    let local = state.local_peer_id.clone();
    add_peer_to_state_list(
        &mut state,
        &local,
        "2024-06-01 12:00:00",
        "2024-06-01 12:00:00",
    );
    assert!(
        state.peers.is_empty(),
        "this install's own device is not a roster row"
    );
}

// ── handle_incoming_message ───────────────────────────────────────────

#[test]
fn test_handle_incoming_broadcast_adds_message() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: "hello all".to_string(),
                peer_id: "peer-1".to_string(),
                latency: None,
                nickname: None,
                msg_id: Some("m1".to_string()),
                attachment: None,
                encrypted: false,
            },
            false,
        );
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].text.contains("hello all"));
        assert_eq!(state.message_ids[0], Some("m1".to_string()));
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_incoming_dm_adds_message_and_tab() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        let dm_count_before = state.dynamic_tabs.dm_tab_count();
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: "secret dm".to_string(),
                peer_id: "peer-dm".to_string(),
                latency: None,
                nickname: None,
                msg_id: Some("dm1".to_string()),
                attachment: None,
                encrypted: false,
            },
            true,
        );
        assert!(state.dm_messages.contains_key("peer-dm"));
        assert!(state.dm_messages["peer-dm"][0].contains("secret dm"));
        assert_eq!(state.dm_message_ids["peer-dm"][0], Some("dm1".to_string()));
        assert_eq!(state.dynamic_tabs.dm_tab_count(), dm_count_before + 1);
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_incoming_empty_content_with_nickname_returns_early() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: String::new(),
                peer_id: "peer-2".to_string(),
                latency: None,
                nickname: Some("Nickname".to_string()),
                msg_id: None,
                attachment: None,
                encrypted: false,
            },
            false,
        );
        // Should NOT add a message (empty content + nickname is just a nick update)
        assert!(state.messages.is_empty());
        // But should update received_nicknames
        assert_eq!(
            state.received_nicknames.get("peer-2"),
            Some(&"Nickname".to_string())
        );
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_incoming_sets_received_nickname() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: "hello".to_string(),
                peer_id: "peer-3".to_string(),
                latency: None,
                nickname: Some("Alice".to_string()),
                msg_id: None,
                attachment: None,
                encrypted: false,
            },
            false,
        );
        assert_eq!(
            state.received_nicknames.get("peer-3"),
            Some(&"Alice".to_string())
        );
        // Message should still be added
        assert_eq!(state.messages.len(), 1);
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_incoming_broadcast_with_latency() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: "ping".to_string(),
                peer_id: "peer-4".to_string(),
                latency: Some("42ms".to_string()),
                nickname: None,
                msg_id: None,
                attachment: None,
                encrypted: false,
            },
            false,
        );
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].text.contains("42ms"));
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_incoming_dm_with_nickname_and_latency() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let mut state = test_app_state();
        state
            .local_nicknames
            .insert("peer-5".to_string(), "Bob".to_string());
        handle_incoming_message(
            &mut state,
            p2p_app::MessageEvent {
                content: "hey".to_string(),
                peer_id: "peer-5".to_string(),
                latency: Some("5ms".to_string()),
                nickname: Some("Bob".to_string()),
                msg_id: Some("dm-5".to_string()),
                attachment: None,
                encrypted: false,
            },
            true,
        );
        assert!(state.dm_messages.contains_key("peer-5"));
        assert!(state.dm_messages["peer-5"][0].contains("hey"));
        assert!(state.dm_messages["peer-5"][0].contains("5ms"));
        assert_eq!(state.dm_message_ids["peer-5"][0], Some("dm-5".to_string()));
    });
    p2p_app::db::reset_db_url();
}

// ── process_swarm_event ───────────────────────────────────────────────

#[test]
fn test_process_swarm_event_broadcast_adds_message() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, _) = mpsc::channel(1);

        process_swarm_event(
            SwarmEvent::BroadcastMessage(MessageEvent {
                content: "hello".to_string(),
                peer_id: "p1".to_string(),
                latency: None,
                nickname: None,
                msg_id: Some("m1".to_string()),
                attachment: None,
                encrypted: false,
            }),
            &state,
            &swarm_cmd_tx,
        )
        .await;

        let s = state.lock().await;
        assert_eq!(s.messages.len(), 1);
        assert!(s.messages[0].text.contains("hello"));
        assert_eq!(s.message_ids[0], Some("m1".to_string()));
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_process_swarm_event_dm_adds_message() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, _) = mpsc::channel(1);

        process_swarm_event(
            SwarmEvent::DirectMessage(MessageEvent {
                content: "secret".to_string(),
                peer_id: "p2".to_string(),
                latency: None,
                nickname: None,
                msg_id: Some("dm1".to_string()),
                attachment: None,
                encrypted: false,
            }),
            &state,
            &swarm_cmd_tx,
        )
        .await;

        let s = state.lock().await;
        assert!(s.dm_messages.contains_key("p2"));
        assert_eq!(s.dm_message_ids["p2"][0], Some("dm1".to_string()));
    });
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_process_swarm_event_peer_disconnected_decrements() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    {
        let mut s = state.lock().await;
        s.concurrent_peers = 3;
    }

    process_swarm_event(
        SwarmEvent::PeerDisconnected("peer-x".to_string()),
        &state,
        &swarm_cmd_tx,
    )
    .await;

    let s = state.lock().await;
    assert_eq!(s.concurrent_peers, 2);
}

#[test]
fn test_process_swarm_event_receipt_stored_in_state() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, _) = mpsc::channel(1);
        {
            let mut s = state.lock().await;
            s.dm_message_ids
                .insert("p1".to_string(), VecDeque::from([Some("dm-1".to_string())]));
        }

        process_swarm_event(
            SwarmEvent::Receipt {
                peer_id: "p1".to_string(),
                ack_for: "dm-1".to_string(),
                received_at: Some(100.0),
            },
            &state,
            &swarm_cmd_tx,
        )
        .await;

        let s = state.lock().await;
        assert!(s.dm_receipts.contains_key("dm-1"));
        assert!(s.broadcast_receipts.contains_key("dm-1"));
    });
    p2p_app::db::reset_db_url();
}

// ── spawn_command_processor ─────────────────────────────────────────────

#[test]
fn test_command_processor_processes_swarm_events() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (input_tx, input_rx) = mpsc::channel(1);
        let (swarm_event_tx, swarm_event_rx) = mpsc::channel(1);
        let (render_tx, mut render_rx) = mpsc::channel(1);
        let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);

        let (_handle, _) = spawn_command_processor(
            state.clone(),
            input_rx,
            swarm_event_rx,
            render_tx,
            swarm_cmd_tx,
        );

        swarm_event_tx
            .send(SwarmEvent::BroadcastMessage(MessageEvent {
                content: "from loop".to_string(),
                peer_id: "p-loop".to_string(),
                latency: None,
                nickname: None,
                msg_id: Some("m-loop".to_string()),
                attachment: None,
                encrypted: false,
            }))
            .await
            .unwrap();

        // Wait for the spawned task to process the event (signals via render_tx)
        render_rx.recv().await;

        let s = state.lock().await;
        assert_eq!(s.messages.len(), 1);
        assert!(s.messages[0].text.contains("from loop"));
        drop(s);

        // Drop senders to stop the loop
        drop(input_tx);
        drop(swarm_event_tx);
    });
    p2p_app::db::reset_db_url();
}

#[tokio::test]
async fn test_command_processor_stops_on_input_exit() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (input_tx, input_rx) = mpsc::channel(1);
    let (_swarm_event_tx, swarm_event_rx) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);

    let (handle, _) = spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        render_tx,
        swarm_cmd_tx,
    );

    // Send Ctrl+Q (exit signal)
    input_tx
        .send(crate::tui::event_source::InputEvent::Key(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('q'),
                crossterm::event::KeyModifiers::CONTROL,
            ),
        ))
        .await
        .unwrap();

    // The event loop should exit and the handle should complete
    tokio::time::timeout(std::time::Duration::from_secs(1), handle)
        .await
        .expect("handle should complete within timeout")
        .unwrap();
}

// ── process_swarm_event: PeerConnected ─────────────────────────────────

#[test]
fn test_process_swarm_event_peer_connected_increments_and_sends_dm() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap().to_string();
    p2p_app::db::set_db_url(&db_str);
    p2p_app::db::init_database().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(1);

        process_swarm_event(
            SwarmEvent::PeerConnected("new-peer".to_string()),
            &state,
            &swarm_cmd_tx,
        )
        .await;

        let s = state.lock().await;
        assert_eq!(s.concurrent_peers, 1);
        assert!(s.peers.iter().any(|p| p.peer_id == "new-peer"));
        drop(s);

        let cmd = tokio::time::timeout(std::time::Duration::from_millis(100), swarm_cmd_rx.recv())
            .await
            .expect("timeout")
            .expect("expected SendDm");
        match cmd {
            SwarmCommand::SendDm { peer_id, .. } => assert_eq!(peer_id, "new-peer"),
            _ => panic!("expected SendDm"),
        }
    });

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── process_swarm_event: ListenAddrEstablished ─────────────────────────

#[tokio::test]
async fn test_process_swarm_event_listen_addr_established() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    // Should not panic or change state
    process_swarm_event(
        SwarmEvent::ListenAddrEstablished("/ip4/0.0.0.0/tcp/9000".to_string()),
        &state,
        &swarm_cmd_tx,
    )
    .await;

    let s = state.lock().await;
    assert_eq!(s.concurrent_peers, 0);
}

#[tokio::test]
async fn test_command_processor_breaks_on_channel_close() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (input_tx, input_rx) = mpsc::channel(1);
    let (swarm_event_tx, swarm_event_rx) = mpsc::channel(1);
    let (render_tx, _render_rx) = mpsc::channel(1);
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(1);

    let (handle, _) = spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        render_tx,
        swarm_cmd_tx,
    );

    // Drop all senders - loop should break
    drop(input_tx);
    drop(swarm_event_tx);

    tokio::time::timeout(std::time::Duration::from_secs(1), handle)
        .await
        .expect("handle should complete within timeout")
        .unwrap();
}

// ── apply_peer_discovered_state (mDNS) ─────────────────────────────────

#[cfg(feature = "mdns")]
#[test]
fn test_apply_peer_discovered_state_adds_peer() {
    let mut state = test_app_state();
    apply_peer_discovered_state(&mut state, "mdns-peer-1");
    assert_eq!(state.peers.len(), 1);
    assert_eq!(state.peers[0].peer_id, "mdns-peer-1");
}

#[cfg(feature = "mdns")]
#[test]
fn test_apply_peer_discovered_state_skips_duplicate() {
    let mut state = test_app_state();
    state.peers.push_back(PeerRecord {
        peer_id: "existing-peer".to_string(),
        first_seen: "2024-01-01 12:00:00".into(),
        last_seen: "2024-01-01 12:00:00".into(),
    });
    apply_peer_discovered_state(&mut state, "existing-peer");
    assert_eq!(state.peers.len(), 1);
}

// ── process_swarm_event: PeerDiscovered (mDNS) ─────────────────────────

#[cfg(feature = "mdns")]
#[tokio::test]
async fn test_process_swarm_event_peer_discovered_adds_peer() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    process_swarm_event(
        SwarmEvent::PeerDiscovered {
            peer_id: "mdns-discovered".to_string(),
            addresses: vec![],
        },
        &state,
        &swarm_cmd_tx,
    )
    .await;

    let s = state.lock().await;
    assert!(s.peers.iter().any(|p| p.peer_id == "mdns-discovered"));
}

#[cfg(feature = "mdns")]
#[tokio::test]
async fn test_process_swarm_event_peer_expired() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);

    // Should not panic
    process_swarm_event(
        SwarmEvent::PeerExpired {
            peer_id: "mdns-expired".to_string(),
        },
        &state,
        &swarm_cmd_tx,
    )
    .await;
}

// ── process_swarm_event: call signaling ────────────────────────────────

async fn assert_call_session(
    state: &Arc<Mutex<AppState>>,
    expected_state: Option<p2p_app::CallState>,
) -> p2p_app::CallSession {
    let s = state.lock().await;
    expected_state.map_or_else(
        || {
            assert!(s.call_session.is_none());
            p2p_app::CallSession::default()
        },
        |state| {
            let session = s.call_session.as_ref().unwrap();
            assert_eq!(session.state, state);
            session.clone()
        },
    )
}

fn call_event(kind: &str, call_id: &str, peer_id: &str) -> SwarmEvent {
    match kind {
        "invite" => SwarmEvent::CallInvite {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
        },
        "accept" => SwarmEvent::CallAccepted {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
        },
        "reject" => SwarmEvent::CallRejected {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
        },
        _ => SwarmEvent::CallEnded {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
        },
    }
}

#[tokio::test]
async fn test_call_invite_adopts_ringing_session() {
    let state = Arc::new(Mutex::new(test_app_state()));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("invite", "call-1", "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    let session = assert_call_session(&state, Some(p2p_app::CallState::Ringing)).await;
    assert_eq!(session.call_id, "call-1");
    assert_eq!(session.peer_id, "bob-d1");
}

#[tokio::test]
async fn test_call_invite_ignored_while_on_live_call() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession::incoming("live-call", "alice-d1"));
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("invite", "other-call", "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    let session = assert_call_session(&state, Some(p2p_app::CallState::Ringing)).await;
    assert_eq!(session.call_id, "live-call", "first call wins");
}

#[tokio::test]
async fn test_call_invite_adopted_after_settled_session() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession {
        call_id: "ended-call".to_string(),
        peer_id: "alice-d1".to_string(),
        state: p2p_app::CallState::Ended,
    });
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("invite", "fresh-call", "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    let session = assert_call_session(&state, Some(p2p_app::CallState::Ringing)).await;
    assert_eq!(session.call_id, "fresh-call");
}

#[tokio::test]
async fn test_call_accept_upgrades_outgoing_to_incall() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession::outgoing("bob-d1"));
    let call_id = app.call_session.as_ref().unwrap().call_id.clone();
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("accept", &call_id, "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    assert_call_session(&state, Some(p2p_app::CallState::InCall)).await;
}

#[tokio::test]
async fn test_call_accept_ignores_foreign_call_id() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession::outgoing("bob-d1"));
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("accept", "wrong-call", "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    assert_call_session(&state, Some(p2p_app::CallState::Calling)).await;
}

#[tokio::test]
async fn test_call_reject_clears_session() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession::outgoing("bob-d1"));
    let call_id = app.call_session.as_ref().unwrap().call_id.clone();
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("reject", &call_id, "bob-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    assert_call_session(&state, None).await;
}

#[tokio::test]
async fn test_call_end_clears_session() {
    let mut app = test_app_state();
    app.call_session = Some(p2p_app::CallSession::incoming("in-call", "alice-d1"));
    let state = Arc::new(Mutex::new(app));
    let (swarm_cmd_tx, _) = mpsc::channel(1);
    process_swarm_event(
        call_event("end", "in-call", "alice-d1"),
        &state,
        &swarm_cmd_tx,
    )
    .await;
    assert_call_session(&state, None).await;
}

// ── handle_group_invite ───────────────────────────────────────────────

#[test]
fn test_handle_group_invite_materializes_group_and_subscribes() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, mut swarm_cmd_rx) = mpsc::channel(8);

        handle_group_invite(
            &state,
            &swarm_cmd_tx,
            "group-abc".to_string(),
            "peer-1".to_string(),
        )
        .await;

        // An invite only carries the id, so the groups row is materialized
        // with the id as its display name (it must show up in the Groups tab).
        let group = p2p_app::groups::find_group("group-abc")
            .unwrap()
            .expect("invited group is materialized");
        assert_eq!(group.group_id, "group-abc");
        assert_eq!(group.display_name, "group-abc");
        // ...the local peer is recorded as a member...
        assert_eq!(
            p2p_app::groups::get_group_member_count("group-abc").unwrap(),
            1
        );
        // ...and we subscribed: the swarm only delivers subscribed topics.
        match swarm_cmd_rx.try_recv() {
            Ok(SwarmCommand::SubscribeGroup { group_id }) => assert_eq!(group_id, "group-abc"),
            other => panic!("expected SubscribeGroup, got {other:?}"),
        }
    });
    p2p_app::db::reset_db_url();
}

#[test]
fn test_handle_group_invite_keeps_an_existing_group_name() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let _dir = tempfile::TempDir::new().unwrap();
    let db_path = _dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // A group we already joined by name (canonical display name known).
        let created = p2p_app::groups::create_public_group("Team Rocket").unwrap();
        let state = Arc::new(Mutex::new(test_app_state()));
        let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(8);

        handle_group_invite(
            &state,
            &swarm_cmd_tx,
            created.group_id.clone(),
            "peer-1".to_string(),
        )
        .await;

        // Materializing is idempotent: the canonical name survives, and the
        // membership insert does not duplicate the row.
        let group = p2p_app::groups::find_group(&created.group_id)
            .unwrap()
            .expect("group still there");
        assert_eq!(group.display_name, "Team Rocket");
        assert_eq!(
            p2p_app::groups::get_group_member_count(&created.group_id).unwrap(),
            1
        );
    });
    p2p_app::db::reset_db_url();
}
