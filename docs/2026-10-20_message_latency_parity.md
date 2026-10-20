# Per-message latency parity: Flutter renders the TUI's round-trip time (2026-10-20)

## Gap

When a broadcast or DM arrives, the TUI prints the measured round-trip time inline
(`apply_broadcast_to_state` / `apply_dm_to_state` do `format!("{ts} {latency} [{sender}] {content}")`,
where `latency` comes from `fmt::format_latency` → `5ms` / `<1ms` / `1.2s` / `?`). The whole
Flutter/web side ignored it: `SwarmEventJson.latency` was carried to Dart but no widget read it, so
the two frontends disagreed about the same live message.

The value is *live-only* — the TUI measures it in memory and never writes it to the database — so a
reloaded history has no latency on either frontend.

## Fix

`ChatMessage` (the canonical model both frontends render) gained an optional `latency`, set by
`mobile_node::save_incoming_message` from the event's measured round-trip time and **never
persisted** (`message_to_chat` always yields `None`, so `load_broadcast_messages` /
`load_dm_messages` return nothing for it — exactly like the TUI). The signature change flows
through `api::save_incoming_message` and the web `saveIncomingMessage` arm (new
`#[serde(default)] latency` param).

In Dart, `saveIncomingMessage` takes an optional `latency`, the broadcast (`HomeScreen`) and DM
(`DmChatScreen`) event handlers thread `event.latency` through, and `MessageBubble` renders it just
before the timestamp — mirroring the TUI's "before the sender name" placement. Group messages
have no latency on the wire, so `GroupChatScreen` maps it to `null`.

## Tests

- `tests/messages.rs::test_incoming_message_from_silent_peer_shows_generated_name` now passes
  `Some("5ms")`, asserts the returned `ChatMessage.latency` is `Some("5ms")`, and asserts a
  reloaded row is `None` (the live-only contract).
- `apps/flutter_app/test/widget/messages_test.dart`:
  `an incoming bubble renders its live latency` (bubble text present only when set) and
  `a live broadcast carries its latency into the chat` (a delivered `broadcast` event with
  `latency: '12ms'` shows `12ms` in the broadcast chat, via the new
  `FakeNodeApi.saveIncomingMessageOverride` seam).
