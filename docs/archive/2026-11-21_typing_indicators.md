# DM typing indicators (2026-11-21)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

This closes **US-009**. A peer composing a DM can show a lightweight
"typing…" indicator without turning keystrokes into messages, receipts, or
database rows.

## Protocol shape

A dedicated, un-gated `request_response` lane
`/p2p-chat/typing/1.0.0` carries `TypingSignal { is_typing, timestamp }` and
is answered immediately with `TypingAck { ok }`. Keeping it off the DM lane
matters: a burst of typing updates must never occupy a DM request slot or delay
a real message. The wire type is deliberately tiny and `serde(default)`-friendly
so a missing timestamp is harmless.

Signals are **fire-and-forget**: there is no retry queue, no persisted row, and
no delivery receipt. The ack exists only to close the request stream promptly.
Receivers apply their own short TTL, so a reordered or dropped stop cannot leave
an indicator stuck forever. Inbound and outbound signals are blocklist-gated at
the same swarm boundary as every other lane.

The command/event pair is `SwarmCommand::SendTyping { peer_id, is_typing }` and
`SwarmEvent::Typing { peer_id, is_typing }`. On Flutter/web the event is flattened
to `event_type: "typing"`, `peer_id`, and `content: "start" | "stop"`, matching
the call-event convention. The new FRB method is
`sendTyping({peerId, isTyping})`, mirrored as the `sendTyping` JSON method over
the web WebSocket.

## Frontend behaviour

- **TUI**: edits on a Direct tab emit a throttled `true` (700 ms per
  conversation), with no signal on an empty composer. Sending a DM, clearing the
  input, or closing the tab emits `false`. Received signals refresh a 3 s TTL;
  a detached 500 ms tick expires them and requests a redraw. The DM pane title
  becomes `DM: {name} — typing…` while active. Disconnecting clears the state.
- **Flutter/web**: `MessageInputBar.onChanged` drives the same 700 ms throttle
  and 3 s TTL. The DM `AppBar` shows an italic `typing…` line under the peer
  name (key `peer-typing`). Send, attachment send, empty input and dispose all
  emit an immediate stop. Typing events for other peers are forwarded down the
  event bus, as receipts already were.

Both frontends keep the state in memory only: a restart or reload never restores
a stale indicator. The canonical TTL/throttle values are duplicated as small
frontend constants because no FRB binding owns presentation timing.

## Tests and drift guards

- `tests/unit/unit_behavior.rs`: protocol name and JSON roundtrip.
- `src/mobile_node.rs`: `event_to_json` mapping for start/stop.
- `tests/unit/unit_bin_tui_state.rs`: receive/expire/clear/throttle helpers.
- `tests/unit/unit_bin_tui_message_handlers.rs`: throttled starts and immediate
  stops on the command lane.
- `tests/tui_render_integration.rs`: the DM pane title renders the indicator.
- `apps/flutter_app/test/widget/messages_test.dart`: incoming display + TTL,
  outgoing throttle, empty-input stop and send stop.
- `src/web/mod.rs`: `sendTyping` stays in the documented-params routing test.
