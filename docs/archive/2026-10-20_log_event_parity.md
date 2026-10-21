# Log-event parity: `file_received` / `peer_expired` in every frontend (2026-10-20)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

## Gap

The TUI binary logs an inbound attachment (`Received file {name} ({file_id}) from {peer_id} at
{path}`) and an mDNS peer-expiry (`Peer expired: {peer_id}`) into the shared in-memory log
buffer (`logging::push_log`), so both lines show up in its Log tab. The Flutter/web node never
logged them: `mobile_node::process_event_for_mobile` had a `FileReceived { .. } => {}` no-op and
fell through `_ => {}` for `PeerExpired`, while `LogTab` polls the very same buffer
(`api::get_logs` → `logging::get_tui_logs`). Same event, same sink, one missing producer.

## Fix

`process_event_for_mobile` now logs both events with the TUI's exact wording (a `_` arm keeps the
non-mDNS build a no-op — note the `mobile` feature does **not** enable `mdns`, while the
`p2p_chat_server`/web build uses the default feature set, which does). No UI change is needed:
`LogTab` already renders the shared buffer, and the file blob is content-addressed on disk by
the time the event fires, so the attachment chip needs no "downloaded" state.

## Tests

`mobile_node::tests::test_process_event_logs_file_received` and
`test_process_event_logs_peer_expired` (`#[cfg(feature = "mdns")]`) push the event and assert the
line lands in `logging::get_tui_logs()`. Unique marker strings keep the assertions safe against
other tests writing the same global buffer in parallel.
