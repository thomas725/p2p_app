    use super::*;
    use crate::tui::test_helpers::test_app_state;

    // ── push_outgoing_broadcast_to_state ─────────────────────────────────────

    #[test]
    fn test_push_outgoing_broadcast_adds_message() {
        let mut state = test_app_state();
        push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "hello", "m1".into(), 1.0);
        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].0.contains("[Me]"));
        assert!(state.messages[0].0.contains("hello"));
        assert_eq!(state.messages[0].1, None);
        assert_eq!(state.message_ids[0], Some("m1".to_string()));
        assert_eq!(state.sent_at_by_msg_id.get("m1"), Some(&1.0));
    }

    #[test]
    fn test_push_outgoing_broadcast_trims_history() {
        let mut state = test_app_state();
        for i in 0..MAX_MESSAGE_HISTORY {
            push_outgoing_broadcast_to_state(
                &mut state,
                "12:00",
                "Me",
                &format!("msg-{i}"),
                format!("m{i}"),
                i as f64,
            );
        }
        assert_eq!(state.messages.len(), MAX_MESSAGE_HISTORY);
        push_outgoing_broadcast_to_state(
            &mut state,
            "12:00",
            "Me",
            "newest",
            "m-last".into(),
            99.0,
        );
        assert_eq!(state.messages.len(), MAX_MESSAGE_HISTORY);
        assert!(state.messages[MAX_MESSAGE_HISTORY - 1].0.contains("newest"));
    }

    #[test]
    fn test_push_outgoing_broadcast_peer_id_is_none() {
        let mut state = test_app_state();
        push_outgoing_broadcast_to_state(&mut state, "12:00", "Me", "test", "m1".into(), 1.0);
        assert_eq!(state.messages[0].1, None);
    }

    // ── push_outgoing_dm_to_state ────────────────────────────────────────────

    #[test]
    fn test_push_outgoing_dm_adds_message() {
        let mut state = test_app_state();
        push_outgoing_dm_to_state(
            &mut state,
            "peer-1",
            "12:00",
            "Me",
            "secret",
            "dm1".into(),
            1.0,
        );
        assert!(state.dm_messages.contains_key("peer-1"));
        assert_eq!(state.dm_messages["peer-1"].len(), 1);
        assert!(state.dm_messages["peer-1"][0].contains("[Me]"));
        assert!(state.dm_messages["peer-1"][0].contains("secret"));
        assert_eq!(state.dm_message_ids["peer-1"][0], Some("dm1".to_string()));
        assert_eq!(state.sent_at_by_msg_id.get("dm1"), Some(&1.0));
    }

    #[test]
    fn test_push_outgoing_dm_trims_history() {
        let mut state = test_app_state();
        for i in 0..MAX_DM_HISTORY {
            push_outgoing_dm_to_state(
                &mut state,
                "p1",
                "12:00",
                "Me",
                &format!("msg-{i}"),
                format!("m{i}"),
                i as f64,
            );
        }
        assert_eq!(state.dm_messages["p1"].len(), MAX_DM_HISTORY);
        push_outgoing_dm_to_state(
            &mut state,
            "p1",
            "12:00",
            "Me",
            "newest",
            "m-last".into(),
            99.0,
        );
        assert_eq!(state.dm_messages["p1"].len(), MAX_DM_HISTORY);
        assert!(state.dm_messages["p1"][MAX_DM_HISTORY - 1].contains("newest"));
        assert!(state.dm_messages["p1"][0].contains("msg-1")); // msg-0 was popped
    }

    #[test]
    fn test_push_outgoing_dm_separate_peers() {
        let mut state = test_app_state();
        push_outgoing_dm_to_state(&mut state, "pa", "12:00", "Me", "hi", "m1".into(), 1.0);
        push_outgoing_dm_to_state(&mut state, "pb", "12:00", "Me", "ho", "m2".into(), 2.0);
        assert_eq!(state.dm_messages.len(), 2);
        assert_eq!(state.dm_messages["pa"].len(), 1);
        assert_eq!(state.dm_messages["pb"].len(), 1);
    }
