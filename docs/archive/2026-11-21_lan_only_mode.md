# LAN-only mode toggle (2026-11-21)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/architecture.md](../architecture.md),
> [docs/README.md](../README.md).

This closes **US-051**. A persisted switch disables every non-LAN mechanism so
an install can stay on the local network without DHT traffic, relay
reservations or internet bootstrap dials.

## Setting

`app_settings` is a new generic key/value table (`key TEXT PRIMARY KEY`,
`value TEXT`) so later toggles reuse the same storage instead of adding a
column each time. `src/settings.rs` is the only place that spells keys or
parses values; it exposes the typed `lan_only()` reader and `set_lan_only(bool)`
writer (stored as `"1"`/`"0"`). Reads are best-effort and default to `false`,
so an existing install keeps its current behaviour and a broken settings table
never stops the node.

The FRB surface (`getLanOnly` / `setLanOnly`) and the mirrored web JSON methods
let both frontends read and edit the same value. Dart calls go through
`NodeApi`, so the in-process (FRB), HTTP (web) and test (fake) transports all
share one contract.

## What it gates

The swarm is built once at node start, so the toggle applies on the **next
node start** — the UI says so in both frontends. The reader is consulted at:

- `swarm_handler::bootstrap_addrs()` — no DHT bootstrap, env or persisted;
- `relay::effective_relay_addrs()` — no relay reservations;
- `behavior::build_behaviour()` — no relay server (even with
  `P2P_RELAY_SERVER=1`);
- `swarm_handler::maybe_announce()` and the per-connection `kad.bootstrap()` —
  no DHT record publication or routing refresh.

mDNS discovery and manual direct dials are deliberately unaffected: LAN-only
means *no internet machinery*, not *no networking*.

## Frontends

- **TUI**: Settings line `LAN only: yes/no`, toggled with `l` (documented in
  the shortcut bar and `?` help). The write goes straight through
  `settings::set_lan_only`; a DB error surfaces as a popup.
- **Flutter/web**: Settings `SwitchListTile` (`lan-only-toggle` key) reads on
  open via `getLanOnly()` and persists via `setLanOnly(enabled:)`, reverting the
  visual change with a snackbar if the write fails.
