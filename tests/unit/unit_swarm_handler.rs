use super::*;
use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    crate::db::set_db_url(db_path.to_str().expect("db path"));
    crate::db::init_database().expect("init db");
    f();
    crate::db::release_db_lock();
    crate::db::reset_db_url();
}

#[test]
#[serial]
fn apply_inbound_reaction_reports_rejection() {
    // A rejected reaction (empty emoji) must return false so the caller does
    // not ack it as accepted — otherwise the sender keeps a chip nobody stored.
    with_test_db(|| {
        assert!(apply_inbound_reaction("m1", "peer-a", "👍", false));
        assert!(!apply_inbound_reaction("m1", "peer-a", "   ", false));
    });
}

/// A canonical content address derived from a per-test seed. The file store
/// validates ids as 64 lowercase hex chars, so these chunk/transfer tests must
/// use real-looking addresses rather than human-readable placeholders.
fn test_file_id(seed: &str) -> String {
    crate::files::hash_bytes(seed.as_bytes())
}

#[test]
fn announced_connections_balance_connects_and_disconnects() {
    // A blocked peer is never announced, so its close must not emit an
    // unmatched PeerDisconnected. Multi-connection peers must stay balanced:
    // each announced connect is released by exactly one disconnect.
    let mut announced = AnnouncedConnections::default();
    assert!(
        !announced.on_disconnected("blocked-peer"),
        "a peer we never announced cannot emit a disconnect"
    );

    announced.on_connected("peer-a");
    announced.on_connected("peer-a");
    assert!(announced.on_disconnected("peer-a"));
    assert!(
        announced.on_disconnected("peer-a"),
        "two announced connects need two disconnects"
    );
    assert!(
        !announced.on_disconnected("peer-a"),
        "a third close has no matching connect"
    );
}

#[test]
fn test_gossipsub_author_prefers_signed_publisher_over_relay() {
    // Gossipsub signs messages (`MessageAuthenticity::Signed`), so `source` is
    // the authenticated publisher. A relayed message must be attributed to the
    // publisher, not the last hop that forwarded it to us.
    let publisher = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    let relay = libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id();
    assert_ne!(publisher, relay);

    assert_eq!(gossipsub_author(Some(publisher), relay), publisher);
    // Unsigned/local messages have no source; the relay is the best available
    // attribution (and the peer that can serve any attachment).
    assert_eq!(gossipsub_author(None, relay), relay);
}

#[test]
fn test_newly_persisted_gates_duplicate_and_failed_saves() {
    // A mesh retransmit (Ok(None)) or a failed write must never produce a live
    // frontend event: the TUI appends group events to in-memory history without
    // its own dedup, and an unsaved row would vanish on the next reload.
    assert!(newly_persisted(&Ok::<Option<u8>, ()>(Some(1))));
    assert!(!newly_persisted(&Ok::<Option<u8>, ()>(None)));
    assert!(!newly_persisted(&Err::<Option<u8>, ()>(())));
}

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
    let id = test_file_id("apply-chunk-no-data-test");
    let file_id = id.as_str();
    assert!(begin_receive("peer-a", file_id, "no-data.bin", None, 10));
    assert!(
        receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );

    let response = crate::FileTransferResponse {
        kind: "chunk".to_string(),
        file_id: file_id.to_string(),
        ..Default::default()
    };
    assert!(matches!(
        apply_chunk(file_id, response),
        ChunkOutcome::Failed
    ));

    // The in-flight entry (and its partial blob) must be gone.
    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );
}

fn chunk_response(
    file_id: &str,
    chunk_index: Option<u64>,
    data: Option<Vec<u8>>,
) -> crate::FileTransferResponse {
    crate::FileTransferResponse {
        kind: "chunk".to_string(),
        file_id: file_id.to_string(),
        chunk_index,
        data,
        ..Default::default()
    }
}

/// Clean up an in-flight transfer so these tests stay idempotent against the
/// process-wide `RECEIVING_FILES` map.
fn clear_transfer(file_id: &str) {
    let removed = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(file_id);
    if let Some(transfer) = removed {
        transfer.file.discard();
    }
}

#[test]
fn test_apply_chunk_empty_data_aborts_transfer() {
    // `Some(vec![])` used to append zero bytes and return the same chunk index,
    // making the handler re-request that chunk forever.
    let id = test_file_id("apply-chunk-empty-data-test");
    let file_id = id.as_str();
    assert!(begin_receive("peer-a", file_id, "empty.txt", None, 10));
    let part_path = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(file_id)
        .map(|r| r.file.path.clone())
        .expect("in-flight transfer");

    let outcome = apply_chunk(file_id, chunk_response(file_id, Some(0), Some(vec![])));
    assert!(matches!(outcome, ChunkOutcome::Failed));
    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );
    assert!(!part_path.exists(), "empty chunk must discard the partial");
    clear_transfer(file_id);
}

#[test]
fn test_apply_chunk_wrong_index_aborts_transfer() {
    // A chunk for any index other than the next expected one must be rejected
    // instead of being appended out of order.
    let id = test_file_id("apply-chunk-wrong-index-test");
    let file_id = id.as_str();
    assert!(begin_receive("peer-a", file_id, "indexed.bin", None, 10));
    let outcome = apply_chunk(
        file_id,
        chunk_response(file_id, Some(4), Some(vec![1u8; 10])),
    );
    assert!(matches!(outcome, ChunkOutcome::Failed));
    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(file_id)
    );
    clear_transfer(file_id);
}

#[test]
fn test_apply_chunk_valid_chunk_completes_and_discards_partial() {
    // The strict checks must still admit a correct chunk so a transfer can
    // finish; the assembled `.part` is only consumed by the finalizer.
    let id = test_file_id("apply-chunk-valid-test");
    let file_id = id.as_str();
    let data = vec![0xABu8; 10];
    assert!(begin_receive("peer-a", file_id, "valid.bin", None, 10));
    let part_path = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(file_id)
        .map(|r| r.file.path.clone())
        .expect("in-flight transfer");

    let outcome = apply_chunk(file_id, chunk_response(file_id, Some(0), Some(data)));
    match outcome {
        ChunkOutcome::Complete(receiving) => {
            assert_eq!(receiving.received, 10);
            receiving.discard();
        }
        other => panic!("expected a complete transfer, got {other:?}"),
    }
    assert!(!part_path.exists());
    clear_transfer(file_id);
}

#[test]
fn test_abandon_transfer_discards_partial_bytes() {
    // Every abort path funnels through `abandon_transfer`: stale `.part` bytes
    // would otherwise corrupt a later transfer of the same content address.
    let id = test_file_id("abandon-transfer-test");
    let file_id = id.as_str();
    assert!(begin_receive("peer-a", file_id, "partial.bin", None, 10));
    let mut transfers = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let transfer = transfers.get_mut(file_id).expect("in-flight transfer");
    transfer.file.write_chunk(&[1u8; 5]).expect("write partial");
    let part_path = transfer.file.path.clone();
    drop(transfers);
    assert!(part_path.exists());

    assert!(abandon_transfer(file_id).is_some());
    assert!(!part_path.exists(), "abandon must remove the partial blob");
    clear_transfer(file_id);
}

#[test]
fn test_abandon_transfers_from_only_affects_the_disconnected_peer() {
    // A peer that disconnects can no longer serve its pulls. Its transfers must
    // be reclaimed while other peers' in-flight pulls survive intact.
    let id_a = test_file_id("abandon-from-peer-a");
    let id_b = test_file_id("abandon-from-peer-b");
    assert!(begin_receive("peer-a", &id_a, "a.bin", None, 10));
    assert!(begin_receive("peer-b", &id_b, "b.bin", None, 10));

    abandon_transfers_from("peer-a");

    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&id_a),
        "the disconnected peer's transfer must be abandoned"
    );
    assert!(
        receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&id_b),
        "another peer's transfer must survive"
    );
    clear_transfer(&id_b);
}

#[test]
fn test_abandon_idle_transfers_prunes_only_stale_pulls() {
    // A failed request that never reaches an "error" response would otherwise
    // wedge the transfer forever. Only pulls idle beyond the timeout are
    // reclaimed; a recently active one must survive.
    let stale_id = test_file_id("abandon-idle-stale");
    let fresh_id = test_file_id("abandon-idle-fresh");
    assert!(begin_receive("peer-a", &stale_id, "stale.bin", None, 10));
    assert!(begin_receive("peer-b", &fresh_id, "fresh.bin", None, 10));
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(600))
        .expect("600s is within Instant's range");
    receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get_mut(&stale_id)
        .expect("stale transfer")
        .last_activity = long_ago;

    abandon_idle_transfers(Duration::from_secs(300));

    assert!(
        !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&stale_id),
        "a transfer idle beyond the timeout must be abandoned"
    );
    assert!(
        receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&fresh_id),
        "a recently active transfer must survive"
    );
    clear_transfer(&fresh_id);
}

#[test]
fn test_begin_receive_refuses_duplicate_meta() {
    // A repeated `meta` response must not replace a live transfer: the fresh
    // `ReceivingFile` would reset `received` to 0 while the `.part` bytes stayed
    // on disk, corrupting the assembled file.
    let id = test_file_id("begin-receive-duplicate-test");
    let file_id = id.as_str();
    assert!(begin_receive("peer-a", file_id, "first.txt", None, 10));
    assert!(!begin_receive("peer-a", file_id, "second.txt", None, 10));

    {
        let transfers = receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        assert!(
            transfers
                .get(file_id)
                .is_some_and(|transfer| transfer.file.name == "first.txt"),
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
fn dispatched_dms_closes_the_async_persist_window() {
    // Two deliveries of the same DM can reach the swarm before the frontend
    // persists either. The in-memory mark must reject the second immediately
    // (while the DB still has no row), scoped per (peer, msg_id).
    let now = Instant::now();
    let mut dispatched = DispatchedDms::default();
    assert!(
        !dispatched.mark("peer-a", "msg-1", now),
        "first dispatch is accepted"
    );
    assert!(
        dispatched.mark("peer-a", "msg-1", now),
        "replay in the persist window is rejected"
    );
    assert!(
        !dispatched.mark("peer-b", "msg-1", now),
        "the same id from another peer is independent"
    );
    assert!(
        !dispatched.mark("peer-a", "msg-2", now),
        "a different id from the same peer is independent"
    );
}

#[test]
fn dispatched_dms_prunes_only_expired_markers() {
    let now = Instant::now();
    let old = now
        .checked_sub(Duration::from_secs(120))
        .expect("120s is within Instant's range");
    let mut dispatched = DispatchedDms::default();
    dispatched.mark("peer-a", "stale", old);
    dispatched.mark("peer-b", "fresh", now);

    dispatched.prune(now, Duration::from_secs(60));

    assert!(
        !dispatched.mark("peer-a", "stale", now),
        "expired marker was pruned and can be dispatched again"
    );
    assert!(
        dispatched.mark("peer-b", "fresh", now),
        "recent marker is retained"
    );
}

#[test]
fn retry_transient_recovers_from_transient_failures() {
    // Gossipsub never redelivers a seen message id, so the receive path must
    // survive a transient DB hiccup instead of losing the message forever.
    let mut calls = 0;
    let result = retry_transient(3, || {
        calls += 1;
        if calls < 3 {
            Err("locked")
        } else {
            Ok("saved")
        }
    });
    assert_eq!(result, Ok("saved"));
    assert_eq!(calls, 3, "stop as soon as the write succeeds");
}

#[test]
fn retry_transient_gives_up_after_the_last_attempt() {
    let mut calls = 0;
    let result: Result<(), &str> = retry_transient(3, || {
        calls += 1;
        Err("still locked")
    });
    assert_eq!(result, Err("still locked"));
    assert_eq!(calls, 3, "never exceed the configured attempt count");
}

#[test]
fn retry_transient_runs_once_when_configured() {
    let mut calls = 0;
    let result: Result<(), &str> = retry_transient(1, || {
        calls += 1;
        Err("failed")
    });
    assert_eq!(result, Err("failed"));
    assert_eq!(calls, 1);
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
        | SwarmCommand::DisconnectPeer { .. }
        | SwarmCommand::StartCallMedia { .. }
        | SwarmCommand::StopCallMedia
        | SwarmCommand::SendCallMedia { .. }
        | SwarmCommand::SetCallMediaMute { .. } => panic!("expected Publish"),
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
        | SwarmCommand::DisconnectPeer { .. }
        | SwarmCommand::StartCallMedia { .. }
        | SwarmCommand::StopCallMedia
        | SwarmCommand::SendCallMedia { .. }
        | SwarmCommand::SetCallMediaMute { .. } => panic!("expected SendDm"),
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
