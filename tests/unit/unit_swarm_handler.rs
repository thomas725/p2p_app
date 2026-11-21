use super::*;

#[test]
fn test_group_topics_covers_main_and_backfill() {
    // Both subscribe and unsubscribe derive their topics from this helper, so
    // it must always yield exactly the main topic + the backfill topic. This
    // guards the group-leave bug where only the main topic was dropped.
    let id = "group-abc";
    let [main, backfill] = group_topics(id);
    assert_eq!(
        main.hash(),
        gossipsub::IdentTopic::new(crate::groups::group_topic(id)).hash()
    );
    assert_eq!(
        backfill.hash(),
        gossipsub::IdentTopic::new(crate::groups::group_backfill_topic(id)).hash()
    );
    assert_ne!(main.hash(), backfill.hash());
}

#[test]
fn test_apply_chunk_without_data_aborts_transfer() {
    // A malformed `chunk` response with `data: None` must abandon the transfer
    // instead of being treated as zero progress (which made the receiver ask
    // for the same chunk forever).
    let file_id = "apply-chunk-no-data-test";
    assert!(begin_receive(file_id, "no-data.bin", None, 10));
    assert!(
        receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );

    assert!(matches!(apply_chunk(file_id, None), ChunkOutcome::Failed));

    // The in-flight entry (and its partial blob) must be gone.
    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );
}

#[test]
fn test_begin_receive_refuses_duplicate_meta() {
    // A repeated `meta` response must not replace a live transfer: the fresh
    // `ReceivingFile` would reset `received` to 0 while the `.part` bytes stayed
    // on disk, corrupting the assembled file.
    let file_id = "begin-receive-duplicate-test";
    assert!(begin_receive(file_id, "first.txt", None, 10));
    assert!(!begin_receive(file_id, "second.txt", None, 10));

    {
        let transfers = receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            transfers
                .get(file_id)
                .is_some_and(|receiving| receiving.name == "first.txt"),
            "the first meta response must still own the in-flight transfer"
        );
        drop(transfers);
    }

    // Clean up the global in-flight map so the test is idempotent.
    let removed = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(file_id);
    assert!(removed.is_some());
}

#[test]
fn test_build_broadcast_message() {
    let msg = build_broadcast_message(
        "Hello world".to_string(),
        Some("Alice".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "Hello world");
    assert_eq!(msg.nickname, Some("Alice".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
    assert!(msg.sent_at.is_some());
}

#[test]
fn test_build_broadcast_message_empty_content() {
    let msg = build_broadcast_message(String::new(), None, None);
    assert_eq!(msg.content, "");
    assert!(msg.nickname.is_none());
    assert!(msg.msg_id.is_none());
}

#[test]
fn test_build_broadcast_message_with_all_fields() {
    let msg = build_broadcast_message(
        "test content".to_string(),
        Some("Tester".to_string()),
        Some("msg-123".to_string()),
    );
    assert_eq!(msg.content, "test content");
    assert_eq!(msg.nickname, Some("Tester".to_string()));
    assert_eq!(msg.msg_id, Some("msg-123".to_string()));
}

#[test]
fn test_make_ack_dm_with_content() {
    let dm = make_ack_dm("ok".to_string(), Some("msg-1".to_string()));
    assert_eq!(dm.content, "ok");
    assert_eq!(dm.ack_for, Some("msg-1".to_string()));
    assert!(dm.nickname.is_none());
    assert!(dm.msg_id.is_none());
    assert!(dm.sent_at.is_some());
    assert!(dm.received_at.is_some());
}

#[test]
fn test_make_ack_dm_no_ack_for() {
    let dm = make_ack_dm(String::new(), None);
    assert_eq!(dm.content, "");
    assert!(dm.ack_for.is_none());
    assert!(dm.sent_at.is_some());
    assert!(dm.received_at.is_some());
}

#[test]
fn test_swarm_command_variants_all() {
    // Test all SwarmCommand variants are constructible
    let publish = SwarmCommand::Publish {
        content: "msg".to_string(),
        nickname: None,
        msg_id: None,
        attachment: None,
        reply_to: None,
    };

    // Verify Publish variant
    match publish {
        SwarmCommand::Publish { content, .. } => {
            assert_eq!(content, "msg");
        }
        SwarmCommand::SendDm { .. }
        | SwarmCommand::PublishGroup { .. }
        | SwarmCommand::SubscribeGroup { .. }
        | SwarmCommand::UnsubscribeGroup { .. }
        | SwarmCommand::SendDeviceBinding { .. }
        | SwarmCommand::SendCallSignal { .. }
        | SwarmCommand::Dial { .. }
        | SwarmCommand::SendTyping { .. }
        | SwarmCommand::SendReaction { .. }
        | SwarmCommand::DisconnectPeer { .. } => panic!("expected Publish"),
    }

    let send_dm = SwarmCommand::SendDm {
        peer_id: "peer".to_string(),
        content: "dm msg".to_string(),
        nickname: Some("Alice".to_string()),
        msg_id: Some("msg-1".to_string()),
        ack_for: Some("prev-msg".to_string()),
        attachment: None,
        reply_to: None,
    };

    // Verify SendDm variant with all fields
    match send_dm {
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
            attachment: _,
            reply_to,
        } => {
            assert_eq!(peer_id, "peer");
            assert_eq!(content, "dm msg");
            assert_eq!(nickname, Some("Alice".to_string()));
            assert_eq!(msg_id, Some("msg-1".to_string()));
            assert_eq!(ack_for, Some("prev-msg".to_string()));
            assert!(reply_to.is_none());
        }
        SwarmCommand::Publish { .. }
        | SwarmCommand::PublishGroup { .. }
        | SwarmCommand::SubscribeGroup { .. }
        | SwarmCommand::UnsubscribeGroup { .. }
        | SwarmCommand::SendDeviceBinding { .. }
        | SwarmCommand::SendCallSignal { .. }
        | SwarmCommand::Dial { .. }
        | SwarmCommand::SendTyping { .. }
        | SwarmCommand::SendReaction { .. }
        | SwarmCommand::DisconnectPeer { .. } => panic!("expected SendDm"),
    }
}

#[cfg(feature = "kad")]
#[test]
fn test_parse_bootstrap_addrs_trims_dedupes_and_skips_garbage() {
    let parsed = parse_bootstrap_addrs(
        " /ip4/1.2.3.4/tcp/4001 ,, /ip4/1.2.3.4/tcp/4001 , not-a-multiaddr , /ip4/5.6.7.8/tcp/4001 ",
    );
    let expected: Vec<libp2p::Multiaddr> = vec![
        "/ip4/1.2.3.4/tcp/4001".parse().expect("parse"),
        "/ip4/5.6.7.8/tcp/4001".parse().expect("parse"),
    ];
    assert_eq!(parsed, expected);
    assert_eq!(parse_bootstrap_addrs("  ,  "), Vec::new());
}
