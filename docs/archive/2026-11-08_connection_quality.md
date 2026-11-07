# Per-peer connection quality (2026-11-08)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

The last open P2 network item (US-052): a per-peer round-trip time and packet
loss read, shown on the Peer Info page in both frontends. The data source was
already there — the swarm ran a `libp2p-ping` behaviour whose events nobody
consumed — so the work was to record those samples and render them once.

## What feeds it

- `swarm_handler` handles `AppEv::Ping(ping::Event)` and calls
  `net_diag::record_ping_rtt(peer_id, rtt)` for a successful outbound result or
  `net_diag::record_ping_failure(peer_id)` for a failure. Only outbound pings
  carry an RTT, and libp2p-ping swallows the first failure of a connection.
- `net_diag::peer_quality(peer_id) -> PeerQuality` reads the sample:
  `{ peer_id, rtt_ms, ping_successes, ping_failures, ping_loss, rtt_label,
  loss_label }`.

## The rule: live-only, never persisted

A quality sample describes the *current* connection, so it is not database
state. `swarm_handler::handle_connection_closed` clears the entry when the
peer's last connection goes away (`swarm.is_connected` is false), which means a
reconnect starts from a clean `n/a` rather than showing a stale RTT as if it
were current. Nothing about ping quality touches SQLite — matching the way
per-message `latency` is a live decoration only.

## One formatter, two frontends

Rust composes the display strings so the two frontends cannot disagree:

- `fmt::format_rtt(Option<f64>)` → `12 ms` (whole milliseconds), `0.4 ms` below
  1 ms, `n/a` when no ping has completed;
- `fmt::format_ping_loss(failures, total)` → `0% (0/13)`, `3% (1/32)`, `n/a`
  before the first ping.

- **TUI**: `render_peer_info_content` prints `Round-trip time:` / `Ping loss:`
  rows on the Peer Info tab, next to the message-stat rows.
- **Flutter**: `api::get_peer_quality` → `NodeApi.getPeerQuality` (FRB and the
  web JSON method `getPeerQuality`) feeds a "Connection quality" card in
  `PeerInfoScreen`. The callback is optional (`loadQuality`), injected by
  `home.dart` / `dm_chat.dart` as `nodeApi.getPeerQuality`, so the screen stays
  node-free for widget tests.

## Verification

- `tests/unit/unit_net_diag.rs`: latest-RTT/loss recording, per-peer isolation,
  clearing on disconnect.
- `tests/fmt.rs`: `format_rtt` unit boundaries and `format_ping_loss` before
  the first ping.
- `tests/tui_render_integration.rs`:
  `test_render_peer_info_shows_ping_quality`.
- `apps/flutter_app/test/widget/peer_info_test.dart`: the quality card renders
  the Rust labels, and is absent without `loadQuality`.
- `src/web/methods.rs` dispatch test covers the `getPeerQuality` arm's param
  validation.
