# Store-and-forward DM outbox (closed 2026-11-19)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md),
> [`../e2e_encryption.md`](../e2e_encryption.md).

The E2E lane (US-042) gave every DM a sealed, per-recipient envelope. This slice
puts that envelope to work for the "recipient is offline" case with a **sender-side
outbox**: a sealed DM that cannot be delivered yet is parked locally and retried
when the peer reconnects (and by a periodic sweep), then cleared on the ack.
The third-party holder protocol from the original backlog item is still a
follow-up — this slice ships the storage, expiry, retry and dedup mechanics that
such a holder would need, without adding a new wire protocol or FRB surface.

## What shipped

- `migrations/2026-11-19-000000_dm_outbox` adds the `dm_outbox` table: one row
  per `(recipient_peer_id, msg_id)`, holding the **full sealed `DirectMessage`
  as JSON** plus `created_at`, `expires_at` (retention deadline) and `attempts`.
  The unique index makes a resend or replay idempotent. The plaintext fields of
  a sealed envelope are empty, so this table never stores a readable body.
- `src/dm_outbox.rs` is the single storage surface: `queue`, `pending`,
  `pending_for`, `remove`, `record_attempt`, `prune_expired`, `count`. Diesel
  artifacts are hand-maintained in `src/generated/` (as with `network_peers`).
- `swarm_handler::send_dm` queues the envelope **after** `seal_dm_payload`, only
  when `encrypted` is present — acks, nickname greetings and DM-lane group
  control stay plaintext and are intentionally not cached.
- The `Message::Response` (ack) arm calls `dm_outbox::remove(peer, ack_for)`, so
  a delivered envelope leaves the outbox. A broadcast receipt is a harmless
  no-op.
- `ConnectionEstablished` flushes `pending_for(peer)`; a 30-second ticker in the
  handler loop calls `prune_expired()` then retries every remaining envelope.
  Retries re-send the stored sealed bytes — they never re-seal.
- Recipient-side dedup: `messages::incoming_dm_exists(peer_id, msg_id)` drops a
  replayed DM before it reaches a frontend (the ack still goes back, so the
  sender's outbox clears). All three frontends persist incoming DMs with the
  sender id and `msg_id`, so the check is authoritative across them.

## Design notes

- **No new FRB/web surface.** The outbox is internal protocol state, not a user
  setting; nothing for Dart or the web client to call. No FRB regeneration.
- **Fail-open.** A DB error in queue/flush/dedup never mutes the DM lane.
- **Retention**: 7 days (`DEFAULT_TTL_SECS`), pruned on the periodic tick. A
  malformed envelope (or a blocked recipient) is dropped rather than retried
  forever.
- Existing tests were extended with outbox DB helpers (idempotent queue, expiry,
  recipient filter, ack removal, attempt counter, sealed-only payload) and an
  end-to-end swarm test: send while offline → row parked → reconnect → deliver →
  ack → row cleared.

Validation: `cargo fmt --all --check`, `cargo ct`, strict `cargo doc`,
`cargo test --all-features` (the two known environmental mDNS failures remain the
only failures), docs link check. No Dart/FRB changes, so no Flutter run was
needed.
