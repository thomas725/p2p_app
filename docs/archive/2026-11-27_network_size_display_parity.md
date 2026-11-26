> **ARCHIVED** — historical record, not current behaviour.

# Settings network-size display parity (2026-11-27)

The Settings tab showed a "Network size" class, but the two frontends described
different things. The TUI cached `get_network_size()` once at startup — the
DB-backed average of past peer sessions used to tune gossipsub — so the label
stayed stale as peers connected and disconnected. Flutter already passed its
live `connectedCount` to `networkSizeLabel`, but the boundary helper carried a
second copy of the classification rule. This slice makes the display a single
live-value rule in Rust and leaves the gossipsub tuning input untouched.

## What shipped

- `network::network_size_label(i64)` is the canonical display classifier:
  0–3 → `Small`, 4–15 → `Medium`, 16+ → `Large`. Negative counts keep the
  historical FRB/web boundary behavior (`Large`); real callers only pass
  non-negative counts.
- The FRB/web `networkSizeLabel` wrapper delegates to that helper. Its signature
  and doc comment are unchanged, so no FRB regeneration was needed. The web
  method and its WS frame protocol are unchanged as well.
- The TUI no longer stores `AppState::network_size` or reads
  `get_network_size()` at startup. `app_state_to_render_state` derives the label
  from `state.connected.len()` on every frame, so connect/disconnect events
  update the Settings row immediately.
- `get_network_size()` and `get_average_peer_count()` remain the persisted
  historical inputs to gossipsub mesh selection. Display and tuning are now
  deliberately separate paths.

## Tests

- Rust unit tests cover the display boundaries (0/3/4/15/16/`i64::MAX`, plus a
  negative count from a synthetic FRB/web caller).
- A TUI render-state test connects and disconnects peers across both thresholds
  and asserts the rendered label follows the live tracker.
- The web WS test adds the 4/15 boundaries; the HTTP transport test now uses the
  canonical label instead of a stale free-form fake string.
- A Flutter widget test replaces `nodeApi` with a fake, asserts the live
  notifier's count is passed to `networkSizeLabel`, and checks the displayed
  value changes `4 → Medium`, `16 → Large`, `0 → Small`.

No Dart production code, Kotlin code, generated bridge file, DB migration or
wire type changed.
