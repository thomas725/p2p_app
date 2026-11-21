# Message reactions (2026-11-21)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

This closes **US-008**. Any user can react to a broadcast, direct or group
message with a short emoji token; the servers of that reaction are aggregated
into chips on the message and kept in sync across peers.

## Storage and identity

A reaction is `(msg_id, peer_id, emoji)` in the new `message_reactions` table,
with a `UNIQUE (msg_id, peer_id, emoji)` constraint and an index on `msg_id`.
Keying on the **wire** `msg_id` rather than a local row id is deliberate: the
same table serves all three message kinds, and a reaction correlates across
peers. Reactor identity is the connection's transport peer id (not the claimed
`actor` on the wire), so an account's devices stay distinct and a forged signal
cannot attribute a reaction to someone else. `src/reactions.rs` owns the model:
`validate_emoji` (trimmed, 1–32 chars), `apply`, `toggle_local`, `has_reaction`,
`recipients_for_message`, and the batched `summaries_for_many` / `hydrate_into`
read path. Re-applying a reaction or replaying a signal is idempotent.

## Wire protocol

A dedicated, un-gated `request_response` lane `/p2p-chat/reactions/1.0.0` carries
`ReactionSignal { msg_id, emoji, actor, remove }` answered by `ReactionAck { ok }`.
All fields are `serde(default)`-friendly. The `actor` field is informational
only — the receiver stamps the connection peer id. The command/event pair is
`SwarmCommand::SendReaction { peer_ids, signal }` and `SwarmEvent::Reaction {
msg_id }`.

The reactor applies its own change **before** queueing the network send, so a
failed queue only delays peer sync — the local UI is already correct. Fan-out
(`recipients_for_message`) is derived from the message kind: a DM reaction goes
only to the conversation partner, a broadcast reaction to every known peer, and
a group reaction to that group's members. The local peer is excluded, DB errors
fail open to an empty recipient list, and every recipient is blocklist-gated at
the swarm boundary like the other lanes.

## Frontend surfaces

`SwarmEvent::Reaction` carries only the `msg_id`; it never embeds a JSON
summary. Each frontend re-reads typed `ReactionSummary { emoji, count, mine }`
values from Rust, so no frontend parses reaction JSON itself. FRB exposes
`getReactions(msgId)` and `toggleReaction(msgId, emoji)`; the web JSON method
names mirror them, and the shared `ChatMessage.reactions` /
`MobileGroupMessage.reactions` fields plus history-loader hydration make chips
appear on a freshly opened chat.

- **TUI**: `Ctrl+Y` offers the shared emoji editor on the newest message with a
  stable wire id; Enter toggles and Esc cancels. Chips render as a compact
  inline suffix (`[👍 2] [❤️ 1*]`, `*` = mine). Local changes and live events
  both refresh the map from `summaries_for`.
- **Flutter/web**: a quick-reaction palette is only an affordance — canonical
  counts and `mine` flags always come from Rust. Chips render from
  `ReactionSummary`; tapping a chip or choosing a palette emoji calls
  `toggleReaction`, and a `reaction` event refreshes that message's chips.
  `ChatMessageExt.withReactions` is the hand-written copy helper (FRB 2.13 emits
  no `copyWith`).

## Tests and drift guards

- `src/reactions.rs`: emoji validation, add/remove idempotency, aggregation and
  `mine` marking, fan-out recipients, and batched hydration.
- `tests/unit/unit_behavior.rs` / `unit_swarm_handler.rs` / `unit_types.rs`:
  protocol JSON, inbound/outbound handling and exhaustive command matching.
- `apps/flutter_app/test/widget/messages_test.dart`: canonical chip render,
  chip-tap toggle, live-event refresh and `toggleReactionAt` summaries.
- `src/web/mod.rs`: `getReactions` / `toggleReaction` stay in the documented
  routing test, and the no-param drift list is unchanged.
