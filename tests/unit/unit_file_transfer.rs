use super::*;

/// Attachment ids on the wire are SHA-256 hex, so these chunk/transfer tests
/// must use real-looking addresses rather than human-readable placeholders.
fn test_file_id(seed: &str) -> String {
    crate::files::hash_bytes(seed.as_bytes())
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
