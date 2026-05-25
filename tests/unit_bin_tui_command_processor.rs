    use super::*;
    use crate::tui::test_helpers::test_app_state;
    use std::collections::VecDeque;

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
        state.peers.push_back((
            "old".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        state.peers.push_back((
            "new".to_string(),
            "2024-06-01 12:00:00".into(),
            "2024-06-01 12:00:00".into(),
        ));
        state.peers.push_back((
            "mid".to_string(),
            "2024-03-01 12:00:00".into(),
            "2024-03-01 12:00:00".into(),
        ));
        sort_peers_by_last_seen(&mut state);
        let ids: Vec<&str> = state.peers.iter().map(|(id, _, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["new", "mid", "old"]);
    }

    #[test]
    fn test_sort_peers_same_last_seen() {
        let mut state = test_app_state();
        state.peers.push_back((
            "a".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        state.peers.push_back((
            "b".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        sort_peers_by_last_seen(&mut state);
        // Stable sort preserves original order for equal last_seen
        let ids: Vec<&str> = state.peers.iter().map(|(id, _, _)| id.as_str()).collect();
        assert_eq!(ids, vec!["a", "b"]);
    }

    // ── upsert_peer_last_seen ────────────────────────────────────────────────

    #[test]
    fn test_upsert_peer_adds_new_peer() {
        let mut state = test_app_state();
        let seen =
            chrono::NaiveDateTime::parse_from_str("2024-06-15 14:30:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        upsert_peer_last_seen(&mut state, "peer-new", seen);
        assert_eq!(state.peers.len(), 1);
        assert_eq!(state.peers[0].0, "peer-new");
        assert_eq!(state.peers[0].2, "2024-06-15 14:30:00");
    }

    #[test]
    fn test_upsert_peer_updates_existing() {
        let mut state = test_app_state();
        state.peers.push_back((
            "peer-exist".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        let later =
            chrono::NaiveDateTime::parse_from_str("2024-12-01 18:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        upsert_peer_last_seen(&mut state, "peer-exist", later);
        assert_eq!(state.peers.len(), 1);
        assert_eq!(state.peers[0].2, "2024-12-01 18:00:00");
    }

    #[test]
    fn test_upsert_peer_moves_to_front_when_latest() {
        let mut state = test_app_state();
        state.peers.push_back((
            "old".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        state.peers.push_back((
            "new".to_string(),
            "2024-06-01 12:00:00".into(),
            "2024-06-01 12:00:00".into(),
        ));
        let later =
            chrono::NaiveDateTime::parse_from_str("2024-12-01 18:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
        upsert_peer_last_seen(&mut state, "old", later);
        // "old" should now be first (latest last_seen)
        assert_eq!(state.peers[0].0, "old");
        assert_eq!(state.peers[0].2, "2024-12-01 18:00:00");
    }

    #[test]
    fn test_upsert_peer_updates_peer_selection() {
        let mut state = test_app_state();
        state.peers.push_back((
            "a".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        state.peers.push_back((
            "b".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        assert_eq!(state.peer_selection, 0);
        let seen =
            chrono::NaiveDateTime::parse_from_str("2024-06-01 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap();
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
        assert_eq!(state.messages[0].1, Some("p1".to_string()));
    }

    #[test]
    fn test_apply_broadcast_to_state_trims_history() {
        let mut state = test_app_state();
        // Override MAX_MESSAGE_HISTORY by filling messages up to 2 below the limit
        for _ in 0..MAX_MESSAGE_HISTORY.saturating_sub(2) {
            state.messages.push_back(("old".to_string(), None));
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
        state.peers.push_back((
            "old".to_string(),
            "2024-01-01 12:00:00".into(),
            "2024-01-01 12:00:00".into(),
        ));
        add_peer_to_state_list(
            &mut state,
            "new",
            "2024-06-01 12:00:00",
            "2024-06-01 12:00:00",
        );
        assert_eq!(state.peers.len(), 2);
        assert_eq!(state.peers[0].0, "new"); // newest first
    }
