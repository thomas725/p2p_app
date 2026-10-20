# Delivery receipts in Flutter + web (2026-10-20) — a `confirmed` tick from the DB

## Gap

The TUI has always shown a delivery confirmation prefix on a message as soon as a recipient
acknowledges it (libp2p request/response: broadcasts are acked by the propagation source, DMs
by their recipient). Flutter/web persisted those receipts (`WS-063`: `message_receipts` rows,
`kind` 0 = broadcast ack, 1 = DM ack resolved via `messages::message_is_dm`, plus the
`broadcast_recipients.confirmed_at` back-fill) but never *rendered* them: the Receipt swarm
event was persisted and then forgotten, so no frontend other than the TUI showed "delivered".

## Decision

The DB is the single source of truth for delivery state, and history loads compute it:

- `ChatMessage` (the FRB/web message model) gained `confirmed: bool`.
- `messages::confirmed_message_ids(&[String]) -> HashSet<String>` answers "which of these
  message ids has at least one `message_receipts` row?" in **one** batched query, so a
  history load stays a constant number of round trips.
- `mobile_node::mark_confirmed(&mut [ChatMessage])` flags outgoing messages (`sent` only — an
  inbound message never shows *our* tick even if a receipt row mentions its id) from that set.
  A DB failure leaves every flag false and logs, so history still renders.
- Live updates: both frontends key on the existing `receipt` swarm event (whose `msgId` is the
  acked message id) and flip the flag in memory rather than reloading history.

This was chosen over mirroring the TUI's in-memory-only receipt maps, which the mobile/web
process may not have (it starts with an empty map on every launch while the DB survives).

## Frontends

- **Flutter/web** `MessageBubble` renders `Icons.done_all` next to the timestamp when
  `isOwn && message.confirmed` (mirrors the TUI's confirmed prefix). `HomeScreen._handleSwarmEvent`
  gains a `receipt` arm (`_markConfirmed(event.msgId)`) for the broadcast chat, and
  `DmChatScreen._handleDmEvent` ticks its own copy **and** still forwards the event to the
  previous sink so the broadcast chat below sees it too.
- `ChatMessage` has no `copyWith` under FRB 2.13, so `lib/src/data/chat_message_ext.dart`
  (hand-written; the generated files are overwritten on every regen) provides
  `withConfirmed(bool)`. A field change on `ChatMessage` turns it into a compile error, which
  is the intent.
- `HttpNodeApi.chatMessageFromJson` reads `confirmed` (`as bool? ?? false`, so an older server
  omits it safely); group messages hardcode `confirmed: false` — receipts are per-recipient and
  group messages have no such tracking.
- The TUI is unchanged (it already renders its own confirmation).

## Tests

- Rust: `messages::tests::{confirmed_message_ids_matches_receipts,
  message_is_dm_resolves_kind_and_unknown_ids,
  save_receipt_backfills_broadcast_recipient_confirmation}` and
  `mobile_node::tests::{test_load_broadcast_messages_marks_confirmed_receipts,
  test_load_dm_messages_marks_confirmed_receipts}` (acked / unacked / inbound rows).
- Dart: `test/widget/messages_test.dart` (bubble tick render, DM receipt event, unknown-id
  no-op, broadcast receipt event through `HomeScreen`) and new `FakeNodeApi` seams
  (`loadDmMessagesOverride`, `loadBroadcastMessagesOverride`, `mobilePeerStatus`,
  `sortContacts` passthrough with `sortContactsOverride`).
