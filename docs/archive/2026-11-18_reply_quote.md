# Reply / quote to a specific message (2026-11-18)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

This closes **US-006**. A user can reply to a specific earlier message in a
broadcast chat, DM, or group chat, and every frontend renders the quoted header
above the new body.

## Shape of the slice

- `reply.rs` defines the canonical `ReplyRef { msg_id, sender, preview }`.
  It is an **immutable snapshot** captured when the reply is composed: the
  quote keeps rendering even if the source row is locally deleted, tombstoned,
  or belongs to a peer we no longer have. No frontend resolves `msg_id` back
  to a row.
- Previews are whitespace-collapsed and bounded to 80 characters plus an
  ellipsis by `reply::preview_line`; `reply::quoted_header` is the one renderer
  and prints `↳ {sender}: {preview}`. Attachment-only quotes reuse the
  canonical Rust file chip, so no frontend does its own byte math.
- `ack_for` stays receipt traffic only. Replies never reuse that field.
- Incoming wire ids are persisted: `messages.msg_id`,
  `group_messages.msg_id`, and the new `reply_to_json` columns (migration
  `2026-11-18-000000_reply_quote`) make reloaded history rows replyable and
  keep their quotes. Wire-less group messages use the same stable
  `nil:<fnv1a>` synthetic ids on the live event and the DB row.
- E2E envelope handling is unchanged. Replies inherit the current plaintext
  behaviour; store-and-forward remains the rationale for US-042.

## Surface

The reply snapshot rides in every outbound message payload and in the shared
FRB/web DTOs:

```text
ReplyRef { msgId, sender, preview }
ChatMessage.replyTo / MobileGroupMessage.replyTo / SwarmEventJson.replyTo
saveIncomingMessage({content, peerId, isDirect, nickname, latency, msgId, replyTo})
```

- **TUI:** `Ctrl+R` captures the newest replyable line in the active Chat, DM,
  or Group conversation into a per-conversation draft. The quoted header
  appears above the input as a preview; `Esc` clears the draft, and sending
  consumes it. Live events and DB reloads go through the same quote renderer.
- **Flutter:** the selection toolbar offers Reply when exactly one message is
  selected. The preview appears above the input bar with a cancel action; all
  three chat surfaces forward `replyTo` on text and attachment sends and clear
  the draft only after a successful send. Group events still refresh from the
  DB rather than persisting in the screen.
- **Web:** the JSON methods mirror the FRB signatures, including camelCase
  `msgId` / `replyTo` on `saveIncomingMessage` and the outgoing send methods.

## Why a snapshot instead of a lookup

A reply is a statement about a message as it was seen, not a live foreign key.
Carrying the sender label and preview in the payload avoids a DB round trip on
every render, survives local tombstones, and keeps the same wire shape for
broadcast, DM, and group traffic. The id alone is still persisted on the row so
future replies can target it.

## Tests

- Rust: `reply.rs` preview/header round trips, display-line capture, wire
  serialization, DB persistence of `msg_id` / `reply_to_json`, and a web JSON
  frame proving camelCase `msgId` / `replyTo` reach the shared helper.
- TUI: quoted headers on live and DB-loaded rows, per-conversation draft
  capture/clear, and outgoing propagation of the snapshot.
- Flutter: quote rendering with a deleted source, single-selection Reply
  action, preview/cancel, and `replyTo` forwarding on all three surfaces.

## Verification

Green: `cargo fmt --all` · `cargo ct` · `flutter analyze` / `flutter test` ·
`python3 scripts/check_docs_links.py`. `cargo test --all-features` passes except
for the two mDNS-dependent tests in `tests/p2p_integration.rs` that fail
identically on clean `HEAD` in this container, so they are environmental flakes,
not a regression.

Closed as part of the US-006 reply/quote implementation.
