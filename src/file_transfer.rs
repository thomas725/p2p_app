//! Receiver-side state for the attachment files lane (§12).
//!
//! The files lane is receiver-driven: each `"meta"`/`"chunk"` request is a
//! separate `request_response` round-trip, so the partial blob and its
//! bookkeeping must live outside any single swarm event. This module owns that
//! process-wide in-flight map, validates and assembles chunks, serves requests
//! for blobs we hold, and reclaims transfers whose peer disappeared. Keeping it
//! out of `swarm_handler` gives the lane one clear home and lets its invariants
//! be tested without a live `Swarm`.

use crate::types::{StringMap, SwarmEvent};
use crate::{AppBehaviour, p2plog_debug};
use libp2p::swarm::Swarm;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Process-wide receiver state for in-flight attachment pulls.
///
/// The files lane (§12) is receiver-driven: each `"chunk"` request is a
/// separate round-trip, so the partial blob and its metadata must live outside
/// any single handler frame. Keyed by content address (the `file_id`).
static RECEIVING_FILES: OnceLock<Mutex<StringMap<InFlightReceive>>> = OnceLock::new();

/// How long a transfer may make no progress before the flush tick abandons it.
/// One round-trip is a single 1 MiB chunk, so five minutes without any chunk is
/// unambiguously a failed pull (e.g. a `request_response` outbound failure that
/// never surfaces as a disconnect). A parameter on `abandon_idle_transfers` so
/// the sweep is testable without waiting.
pub(crate) const FILE_TRANSFER_IDLE_TIMEOUT: Duration = Duration::from_secs(300);

/// One in-flight receive plus the bookkeeping needed to reclaim it when its
/// source disappears. `peer` ties the pull to the connection serving it (so a
/// disconnect abandons exactly that peer's transfers); `last_activity` is the
/// backstop for requests that fail without any disconnect.
struct InFlightReceive {
    file: crate::files::ReceivingFile,
    peer: String,
    last_activity: Instant,
}

fn receiving_files() -> &'static Mutex<StringMap<InFlightReceive>> {
    RECEIVING_FILES.get_or_init(|| Mutex::new(StringMap::default()))
}

/// Garbage-collect dead attachment blobs (files lane §12): `.part` partials of
/// interrupted pulls and orphaned blobs/rows nothing references any more.
///
/// The `attachment-gc` Cargo feature is the enable/disable switch: with the
/// feature off this returns a zeroed report and the store never deletes
/// anything, so a build can opt out of blob cleanup entirely. Blobs with a
/// transfer currently in flight are excluded, and anything younger than
/// [`GC_GRACE`](crate::files::GC_GRACE) is skipped.
///
/// # Errors
/// Returns an error if a sweep fails (see [`crate::files::gc`]).
#[cfg(feature = "attachment-gc")]
pub fn gc_stale_attachments() -> color_eyre::Result<crate::files::GcStats> {
    let active: std::collections::HashSet<String> = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .keys()
        .cloned()
        .collect();
    crate::files::gc(&active, crate::files::GC_GRACE)
}

/// No-op build of [`gc_stale_attachments`] when the `attachment-gc` feature is
/// off (see the feature's doc comment in Cargo.toml).
#[cfg(not(feature = "attachment-gc"))]
pub fn gc_stale_attachments() -> color_eyre::Result<crate::files::GcStats> {
    Ok(crate::files::GcStats::default())
}

/// Remove an in-flight transfer and discard its `.part` blob. Every abort path
/// (malformed chunk, peer error, oversized meta, disconnect, idle sweep) must go
/// through here so a later transfer of the same content address cannot append
/// to stale bytes.
fn abandon_transfer(file_id: &str) -> Option<InFlightReceive> {
    let partial = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(file_id);
    if let Some(partial) = partial.as_ref() {
        partial.file.discard();
    }
    partial
}

/// Abandon every in-flight pull being served by `peer`. A disconnecting peer can
/// no longer send the remaining chunks, so leaving its entries in the map wedges
/// `maybe_start_fetch` (which skips addresses already in flight) and keeps the
/// startup GC from reclaiming the stale `.part` blob.
pub(crate) fn abandon_transfers_from(peer: &str) {
    let stale: Vec<String> = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|(_, transfer)| transfer.peer == peer)
        .map(|(file_id, _)| file_id.clone())
        .collect();
    for file_id in stale {
        p2plog_debug(format!(
            "abandoning file transfer {file_id}: peer {peer} disconnected"
        ));
        abandon_transfer(&file_id);
    }
}

/// Abandon transfers that have made no progress for longer than `timeout`.
/// Covers failures that never reach an `"error"` response — notably a
/// `request_response` outbound failure (timeout/protocol error) with no peer
/// disconnect — which would otherwise leave a transfer wedged until restart.
pub(crate) fn abandon_idle_transfers(timeout: Duration) {
    let stale: Vec<String> = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|(_, transfer)| transfer.last_activity.elapsed() > timeout)
        .map(|(file_id, _)| file_id.clone())
        .collect();
    for file_id in stale {
        p2plog_debug(format!("abandoning idle file transfer {file_id}"));
        abandon_transfer(&file_id);
    }
}

/// What applying one chunk response should do next.
#[derive(Debug)]
enum ChunkOutcome {
    /// All bytes are in: hand the assembled transfer to the finalizer.
    Complete(crate::files::ReceivingFile),
    /// Append succeeded but the file is incomplete: pull chunk `u64` next.
    Continue(u64),
    /// The transfer cannot go on (unknown/errored); any partial is discarded.
    Failed,
}

/// Validate one chunk response against the transfer's advertised size and
/// current progress. Rejecting empty/mis-indexed/oversized chunks is what stops
/// a peer from keeping the receiver in the same request/response loop forever.
fn chunk_is_expected(
    receiving: &crate::files::ReceivingFile,
    chunk_index: Option<u64>,
    total_chunks: Option<u64>,
    chunk_size: Option<u64>,
    data_len: u64,
) -> bool {
    let expected = receiving.received.div_euclid(crate::files::FILE_CHUNK_SIZE);
    let remaining = receiving.size.saturating_sub(receiving.received);
    let advertised = crate::files::total_chunks(receiving.size);
    let expected_len = if expected.saturating_add(1) < advertised {
        crate::files::FILE_CHUNK_SIZE
    } else {
        remaining
    };
    data_len > 0
        && data_len == expected_len
        && chunk_index == Some(expected)
        && total_chunks.is_none_or(|value| value == advertised)
        && chunk_size.is_none_or(|value| value == crate::files::FILE_CHUNK_SIZE)
}

/// Append one chunk's bytes to the in-flight transfer and decide what happens
/// next. Kept fully synchronous so no `MutexGuard` (not `Send`) ever crosses an
/// `.await` in the handler task.
fn apply_chunk(file_id: &str, response: crate::FileTransferResponse) -> ChunkOutcome {
    let mut transfers = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(transfer) = transfers.get_mut(file_id) else {
        p2plog_debug(format!("chunk for unknown transfer {file_id}"));
        return ChunkOutcome::Failed;
    };
    let Some(data) = response.data else {
        p2plog_debug(format!("chunk for {file_id} carried no data"));
        drop(transfers);
        abandon_transfer(file_id);
        return ChunkOutcome::Failed;
    };
    let data_len = u64::try_from(data.len()).unwrap_or(u64::MAX);
    if !chunk_is_expected(
        &transfer.file,
        response.chunk_index,
        response.total_chunks,
        response.chunk_size,
        data_len,
    ) {
        p2plog_debug(format!("rejected unexpected chunk for {file_id}"));
        drop(transfers);
        abandon_transfer(file_id);
        return ChunkOutcome::Failed;
    }
    if let Err(e) = transfer.file.write_chunk(&data) {
        p2plog_debug(format!("failed to write chunk for {file_id}: {e}"));
        drop(transfers);
        abandon_transfer(file_id);
        return ChunkOutcome::Failed;
    }
    let complete = transfer.file.is_complete();
    let received = transfer.file.received;
    // Every accepted chunk proves the peer is still serving the pull, so the
    // idle sweep must count from the latest one, not from the meta handshake.
    transfer.last_activity = Instant::now();
    if complete {
        let partial = transfers.remove(file_id);
        drop(transfers);
        return partial.map_or(ChunkOutcome::Failed, |t| ChunkOutcome::Complete(t.file));
    }
    drop(transfers);
    ChunkOutcome::Continue(received.div_euclid(crate::files::FILE_CHUNK_SIZE))
}

/// Atomically register a fresh in-flight receive for `file_id` from `peer`.
///
/// Returns `true` when a new transfer was registered, `false` when one is
/// already in flight (a duplicate `meta` response) or the partial file could
/// not be opened. A duplicate must never replace the live transfer: a fresh
/// `ReceivingFile` would reset `received` to 0 and discard the `.part` bytes
/// the live transfer is still assembling, desyncing the chunk index from the
/// peer. (A stale `.part` from a *killed* process is handled separately by
/// [`ReceivingFile::new`](crate::files::ReceivingFile::new).)
fn begin_receive(peer: &str, file_id: &str, name: &str, mime: Option<String>, size: u64) -> bool {
    let mut transfers = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if transfers.contains_key(file_id) {
        p2plog_debug(format!("ignoring duplicate meta for transfer {file_id}"));
        return false;
    }
    let file = match crate::files::ReceivingFile::new(file_id, name, mime, size) {
        Ok(file) => file,
        Err(e) => {
            p2plog_debug(format!("failed to open receiving file for {file_id}: {e}"));
            return false;
        }
    };
    transfers.insert(
        file_id.to_string(),
        InFlightReceive {
            file,
            peer: peer.to_string(),
            last_activity: Instant::now(),
        },
    );
    true
}

/// Receiver side of the attachment pull: if we neither hold the blob in the
/// store nor have a transfer in flight, ask the source peer for its metadata
/// to begin the chunk chain. Content-addressed, so an already-stored file
/// short-circuits the whole transfer.
pub(crate) fn maybe_start_fetch(
    peer_id: &libp2p::PeerId,
    attachment: &crate::files::MessageAttachment,
    swarm: &mut Swarm<AppBehaviour>,
) {
    // The descriptor rides in on the wire; never turn an unvalidated id into a
    // request (nor, downstream, a filesystem path).
    if !crate::files::is_valid_file_id(&attachment.file_id) {
        p2plog_debug(format!(
            "ignoring attachment with invalid file id {:?}",
            attachment.file_id
        ));
        return;
    }
    if crate::files::file_path(&attachment.file_id).is_none()
        && !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&attachment.file_id)
    {
        request_file_meta(swarm, peer_id, &attachment.file_id);
    }
}

fn request_file_meta(swarm: &mut Swarm<AppBehaviour>, peer_id: &libp2p::PeerId, file_id: &str) {
    let request = crate::FileTransfer {
        kind: "meta".to_string(),
        file_id: file_id.to_string(),
        chunk_index: None,
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_request(peer_id, request);
}

fn request_file_chunk(
    swarm: &mut Swarm<AppBehaviour>,
    peer_id: &libp2p::PeerId,
    file_id: &str,
    chunk_index: u64,
) {
    let request = crate::FileTransfer {
        kind: "chunk".to_string(),
        file_id: file_id.to_string(),
        chunk_index: Some(chunk_index),
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_request(peer_id, request);
}

/// Handle the file-transfer lane (§12).
///
/// Requests come from a peer who wants a blob we hold in the store: serve
/// either its metadata (name/size/mime plus the chunking parameters) or one
/// chunk of bytes. Responses come from a peer we are pulling from: `"meta"`
/// starts a transfer, each `"chunk"` appends its bytes (and requests the next
/// one unless the transfer just completed), and `"error"` abandons it.
pub(crate) async fn handle_file_transfer(
    message: libp2p::request_response::Message<crate::FileTransfer, crate::FileTransferResponse>,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => serve_file_transfer_request(&request, channel, swarm),
        libp2p::request_response::Message::Response { response, .. } => {
            handle_file_transfer_response(response, peer, peer_id_str, event_tx, swarm).await;
        }
    }
}

/// Serve a transfer request from a peer who wants a blob we hold: answer
/// `"meta"` with the descriptor + chunking parameters, `"chunk"` with the
/// requested slice, or `"error"` for anything else.
fn serve_file_transfer_request(
    request: &crate::FileTransfer,
    channel: libp2p::request_response::ResponseChannel<crate::FileTransferResponse>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let file_id = request.file_id.clone();
    let response = match request.kind.as_str() {
        "meta" => match crate::files::serve_file_meta(&file_id) {
            Some((name, size, mime)) => crate::FileTransferResponse {
                kind: "meta".to_string(),
                file_id,
                name: Some(name),
                size: Some(size),
                mime,
                total_chunks: Some(crate::files::total_chunks(size)),
                chunk_size: Some(crate::files::FILE_CHUNK_SIZE),
                ..Default::default()
            },
            None => crate::FileTransferResponse {
                kind: "error".to_string(),
                file_id,
                reason: Some("unknown file".to_string()),
                ..Default::default()
            },
        },
        "chunk" => {
            let chunk_index = request.chunk_index.unwrap_or_default();
            match crate::files::serve_chunk(&file_id, chunk_index) {
                Some(data) => crate::FileTransferResponse {
                    kind: "chunk".to_string(),
                    file_id,
                    chunk_index: Some(chunk_index),
                    data: Some(data),
                    ..Default::default()
                },
                None => crate::FileTransferResponse {
                    kind: "error".to_string(),
                    file_id: file_id.clone(),
                    reason: Some(format!("no chunk {chunk_index} for {file_id}")),
                    ..Default::default()
                },
            }
        }
        other => crate::FileTransferResponse {
            kind: "error".to_string(),
            file_id,
            reason: Some(format!("unknown file-transfer kind: {other}")),
            ..Default::default()
        },
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_response(channel, response);
}

/// Receiver side of the pull: `"meta"` starts a transfer (refusing oversized
/// files and finishing instantly when empty), each `"chunk"` appends its bytes
/// and pulls the next one when incomplete, and `"error"` abandons the transfer.
async fn handle_file_transfer_response(
    response: crate::FileTransferResponse,
    peer: libp2p::PeerId,
    peer_id_str: String,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match response.kind.as_str() {
        "meta" => {
            let file_id = response.file_id.clone();
            if !crate::files::is_valid_file_id(&file_id) {
                p2plog_debug(format!(
                    "refusing transfer with invalid file id {file_id:?}"
                ));
                return;
            }
            let size = response.size.unwrap_or_default();
            if size > crate::files::MAX_ATTACHMENT_SIZE {
                // Drop the partial too: a later valid meta for the same content
                // address must not append to bytes from the refused transfer.
                abandon_transfer(&file_id);
                p2plog_debug(format!(
                    "refusing oversized attachment {file_id} ({size} bytes)"
                ));
                return;
            }
            if !begin_receive(
                &peer_id_str,
                &file_id,
                response.name.as_deref().unwrap_or("attachment"),
                response.mime.clone(),
                size,
            ) {
                return;
            }
            // A zero-length file completes immediately.
            if size == 0 {
                let receiving = receiving_files()
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&file_id)
                    .map(|transfer| transfer.file);
                if let Some(receiving) = receiving {
                    finish_received_file(receiving, peer_id_str, event_tx).await;
                }
            } else {
                request_file_chunk(swarm, &peer, &file_id, 0);
            }
        }
        "chunk" => {
            let file_id = response.file_id.clone();
            match apply_chunk(&file_id, response) {
                ChunkOutcome::Complete(partial) => {
                    finish_received_file(partial, peer_id_str, event_tx).await;
                }
                ChunkOutcome::Continue(next_chunk) => {
                    request_file_chunk(swarm, &peer, &file_id, next_chunk);
                }
                ChunkOutcome::Failed => {
                    p2plog_debug(format!("file transfer {file_id} could not continue"));
                }
            }
        }
        "error" => {
            let file_id = response.file_id.clone();
            let reason = response.reason.unwrap_or_default();
            abandon_transfer(&file_id);
            p2plog_debug(format!("file transfer {file_id} failed: {reason}"));
        }
        other => p2plog_debug(format!("unknown file-transfer response kind: {other}")),
    }
}

/// Surface an assembled attachment to the frontends. The transfer is already
/// drained from the in-flight map and fully stored by the time this runs.
async fn finish_received_file(
    receiving: crate::files::ReceivingFile,
    peer_id: String,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    let file_id = receiving.file_id.clone();
    let name = receiving.name.clone();
    let path = match receiving.finalize() {
        Ok(path) => path,
        Err(e) => {
            p2plog_debug(format!("failed to finalize attachment {file_id}: {e}"));
            return;
        }
    };
    let _ = event_tx
        .send(SwarmEvent::FileReceived {
            peer_id: peer_id.clone(),
            file_id: file_id.clone(),
            name,
            path,
        })
        .await;
    // A share manifest may have been waiting on this exact blob. Applying it
    // here (rather than in the UI) keeps folder convergence independent of
    // which frontend is running.
    for applied in crate::shares::note_blob_available(&file_id, &peer_id) {
        let _ = event_tx
            .send(SwarmEvent::ShareUpdated {
                peer_id: applied.peer_id,
                share_id: applied.share_id,
                revision: applied.revision,
                written: applied.stats.written,
                removed: applied.stats.removed,
            })
            .await;
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_file_transfer.rs"]
mod tests;
