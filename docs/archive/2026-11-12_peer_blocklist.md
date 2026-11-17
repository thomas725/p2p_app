# Peer blocklist / mute (2026-11-12)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

US-054 closed: a muted peer is now ignored at the **swarm boundary** rather
than merely hidden in a list. Blocking one peer id persists a row, drops
everything it sends before parsing, refuses to dial it, and disconnects it if it
still manages to connect.

## The shape

- Migration `migrations/2026-11-12-000000_blocked_peers/`: one table,
  `blocked_peers (peer_id TEXT PRIMARY KEY, blocked_at TIMESTAMP)`. The primary
  key makes blocking idempotent (`on_conflict_do_nothing`) and unblocking a
  single delete, so a frontend toggle never has to read first.
- `src/blocklist.rs` owns it: `block_peer` / `unblock_peer` / `is_blocked` /
  `list_blocked_peers`. Peer ids are canonicalized by parsing them as a
  `libp2p::PeerId` and storing the canonical base58 text, so
  `"  <peer>  "` and `<peer>` are the same row. Empty or unparseable ids are
  rejected with a human-readable error.
- Diesel artifacts stay hand-maintained (`src/generated/schema.rs` gained the
  `blocked_peers` block and the `allow_tables_to_appear_in_same_query!` entry;
  `BlockedPeerRow` / `NewBlockedPeer` were added by hand). `diesel_generate.sh`
  must not be run — `diesel_ext` is not on crates.io.

## Enforcement points (`src/swarm_handler.rs`)

| Path | Behaviour |
|---|---|
| gossipsub message | dropped before topic routing, so broadcast, group chat **and** group backfill traffic is never parsed or persisted |
| DM / device-binding / call-signaling / file-transfer request | dropped before handling |
| Identify received | ignored (no DHT record, no roster update) |
| mDNS discovered | not dialed, not announced |
| `ConnectionEstablished` | disconnected immediately, before `PeerConnected` is emitted |
| outbound `SendDm`, `SendDeviceBinding`, `SendCallSignal` | refused |
| manual `Dial` | refused when the multiaddr carries `/p2p/<blocked-id>` |
| `SwarmCommand::DisconnectPeer` | queued by a successful block (TUI and `api::block_peer`, i.e. Flutter/web) so a live connection predating the block is closed |

`is_blocked` is **fail-open**: an unreadable database returns `false`, so a
broken DB can never act as a global mute. Blocking is deliberately per peer id —
the account layer knows which devices belong to one user, the blocklist does
not, so muting one device leaves a contact's sibling devices reachable. That is
documented in the module doc rather than treated as a gap.

## Frontend surface

- **TUI**: the Peer Info tab gained `b` (block/unblock) and a
  `Blocked: yes (b: unblock)` / `Blocked: no (b: block)` row. The toggle writes
  through `blocklist::block_peer` / `unblock_peer`, refreshes
  `AppState::blocked_peers`, and surfaces failures in the usual popup. The
  shortcut bar documents `b: block/unblock` only on that tab.
- **Flutter**: `PeerInfoScreen` takes optional `loadBlocked` / `toggleBlock`
  callbacks (so widget tests need no node), renders the same `Blocked` row in
  the Activity card, and puts the action on the app bar (`block-peer-info`,
  tooltip `Block peer` / `Unblock peer`). Both Peer Info entry points — the
  Contacts tab and the DM chat's info button — pass the shared
  `util/blocklist.dart` helpers (`peerIsBlocked` / `togglePeerBlock`), which
  read first and then flip the persisted entry, so the returned state is always
  the truth.
- **Web**: unchanged code, because `isPeerBlocked` / `blockPeer` /
  `unblockPeer` / `listBlockedPeers` mirror the FRB surface 1:1 in
  `src/web/methods.rs`, with the usual no-param drift-list entry
  (`listBlockedPeers`) and params-less rejection entries
  (`blockPeer`, `unblockPeer`).

## Why the UI row lives on Peer Info

The blocklist is a per-device property, and Peer Info is the one screen that is
already device-scoped (the Contacts table's device sub-rows resolve to the same
page). Putting the toggle there kept every list view unchanged — no new column,
no extra sort key, no per-row menu — while still keeping TUI/Flutter parity.

## Tests

`tests/unit/unit_blocklist.rs` covers the roundtrip (block → list → idempotent
re-block → unblock), id canonicalization, rejected ids, and the fail-open read
against a database path that cannot be opened. The TUI toggle has a
DB-backed unit test (`test_blocklist_toggle_from_peer_info_tab`), the Peer Info
renderer asserts both states and the hint, and the Flutter widget test drives
the load/toggle callbacks (including a rejected write) — plus a unit test for
the shared toggle helper over `FakeNodeApi`.
