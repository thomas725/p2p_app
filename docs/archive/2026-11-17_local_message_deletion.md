# Local-only sent-message deletion (2026-11-17)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

This closes the deletion half of **US-005**. A user can hide a message they
sent from broadcast chat, a DM, or a group chat; nothing is retracted from the
network. Editing a sent message remains open in [the backlog](../backlog.md).

## Shape of the slice

- `messages.deleted_at` and `group_messages.deleted_at` are nullable
  `DOUBLE` columns added by `2026-11-17-000000_local_message_tombstones`.
  `NULL` means visible; a Unix timestamp means locally hidden.
- `messages::mark_message_deleted(id)` and
  `groups::mark_group_message_deleted(id)` stamp the column only when
  `id = ? AND sent = 1 AND deleted_at IS NULL`. Incoming rows can never be
  deleted, and a second delete is a no-op.
- Every local history loader filters tombstones: `messages::load_messages`,
  `messages::load_direct_messages`, `groups::load_group_messages`, and
  `groups::load_group_messages_after`. Group backfill therefore never leaks a
  local deletion onto the wire. `groups::latest_group_sent_at` deliberately
  stays **unfiltered**, so peers do not resend a tombstoned row after a gap.
- `messages::get_all_peer_stats` excludes tombstones from DM counts.
  `broadcast_sent_to_peer` deliberately does not change: deletion is a view
  concern, not a retraction of the fact that a broadcast reached a peer.
- Receipt rows and attachment blobs are untouched. A hidden row simply never
  reaches the DTO, so `ChatMessage.confirmed`, the live `latency` decoration,
  and the per-recipient delivery tick keep their existing semantics.

## Surface

One new FRB/web method serves all three frontends:

```text
delete_message(message_id: i32, is_group: bool) -> Result<(), String>
deleteMessage({messageId, isGroup})                 // Dart / JSON
```

It returns a clear error when no deletable sent row exists. No wire type,
`SwarmEvent`, `SwarmCommand`, `ChatMessage` field, or generated Dart DTO
changed — `is_group` only picks which local table to stamp.

- **TUI:** aligned DB-id queues let a chat, DM, or group-chat tab target the
  most recent message the local user sent. `Delete` opens a confirmation;
  `y`/Enter confirms, anything else cancels. Confirming tombstones the row and
  drops the matching line from the in-memory history. Outgoing TUI messages
  are now explicitly `mark_message_sent` after persistence, which is what
  makes them eligible for local deletion.
- **Flutter/Web:** double-tap selection's toolbar gained a delete action,
  enabled only when the selection contains sent messages. The confirmation
  says plainly that the message is hidden only on this device and that other
  people keep their copies. `NodeApi`, `FrbNodeApi`, `HttpNodeApi`, and the
  test fake all carry `deleteMessage`.
- The Flutter help/controls text and [`KEYBOARD_SHORTCUTS.md`](../../KEYBOARD_SHORTCUTS.md)
  document the new affordances.

## Why a tombstone instead of a delete

Rows are referenced by receipts, broadcast-recipient records, and the group
backfill watermark. Physically deleting them would either cascade away those
facts or leave a row resurfacing from another peer. A local tombstone keeps
the database as the single source of truth while making deletion genuinely
local: no new protocol handshake, no remote state to reconcile, and no change
to the FRB-visible message shape.

## Tests

- Rust: defaults and filtering for broadcast/DM/group loads, group-backfill
  exclusion with an unfiltered watermark, incoming-row rejection, DM-stat
  exclusion, and a web JSON frame reaching the local helper (including the
  invalid-params drift assertion).
- TUI: queue alignment, last-own-message targeting, prompt cancel/confirm,
  and the no-own-row case.
- Flutter: generated wrapper presence, `HttpNodeApi` frame/params, local DM
  delete, dialog cancel, and the group `isGroup: true` path.

## Verification

Green: `cargo fmt --all -- --check` · `cargo ct` · stable-clippy gate ·
`cargo doc --no-deps --all-features --lib` with `RUSTDOCFLAGS='-D warnings -W
missing_docs'` · `flutter analyze` / `flutter test` · `cargo audit` ·
`python3 scripts/check_docs_links.py`. The two network-dependent mDNS tests in
`tests/p2p_integration.rs` remain environmental flakes in this container.
