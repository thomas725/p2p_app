> **ARCHIVED** — historical record, not current behaviour.

# Rust-owned last-connection label (2026-11-27)

Flutter Settings' "Last connection lost" row used to resolve the disconnected
peer by looking it up in the current roster. A peer that left the roster was
printed as a raw peer ID, while the TUI named it through Rust's canonical
`nickname::get_peer_display_name_or_short`. This closes the deferred item in
[`2026-11-27_display_parity_details.md`](2026-11-27_display_parity_details.md)
without a new FRB binding: the existing `SwarmEventJson.content` field was
already carried by both transports.

## What shipped

- `mobile_node::event_to_json` fills `content` on `peer_disconnected` with the
  canonical Rust display label. The resolver prefers local nickname, then
  received nickname, then a stable generated petname, each with the short-ID
  suffix, and falls back to the short ID if the database fails.
- `LiveStatus` carries `lastConnectionLabel` alongside the peer ID. Home copies
  the event's label into that snapshot and clears both on node stop/start.
- Settings prefers the Rust label and keeps the roster lookup (then the raw ID)
  as a fallback for older or synthetic events that have no label.
- Web needs no change: `SwarmEventJson` is serialized to JSON directly, so the
  same `content` reaches the browser event bus.

## Why this shape

Naming belongs in Rust: the TUI already renders the same value at read time,
and duplicating petname generation in Dart would let the two frontends drift.
Using the existing event envelope avoids FRB regeneration and a matching web
method. The roster fallback is deliberate so events injected by tests or an
older server still produce a useful label.

## Coverage

- `mobile_node::tests::test_event_to_json_peer_disconnected` asserts the exact
  label with an isolated database and cleared display-name cache.
- `settings_test.dart` renders the label with no roster row and checks that the
  raw peer ID is not printed.
- `home_last_connection_test.dart` delivers a real `peer_disconnected` event
  through `FakeNodeApi` into Home, opens Settings through the bottom navigation
  and checks the label reaches the rendered row.
