# Backlog

**Live, open work only.** Everything that ships gets archived — see
[archive/](archive/) for the closed items and their rationale.

This file is the single place to look for "what's next". It merges the two
tracks that used to drift apart: engineering items (dependency lanes, protocol
work) and the product stories from the original user-stories doc.

## How to use this file

- **One item = one commit.** Land the change, then tick the box in the same
  commit. An item with no code change is not done.
- **When an item closes**, move it to [archive/](archive/) and link the commit.
  The closed backlog lives in `archive/2026-09-02_backlog.md` (cross-frontend
  engineering items) and `archive/2026-04-17_user_stories.md` (the product
  stories, with their acceptance criteria).
- **Priorities**: `P0` blocks a release or a correctness guarantee, `P1` is
  planned work, `P2` is polish. Priority is a claim about value, not about
  effort.
- **Blocked** items name the blocker and who can clear it.

---

## P0 — correctness and release blockers

### Sibling-device claim authority (identity lane) — **implemented (2026-11-02)**

**Decision (2026-11-01): any already-bound device of the same user may add another
device to that user's set.** The account key keeps working; this only widens the
set of authorized signers.

Shipped: `verify_device_binding_claim` now accepts an optional sender peer ID and
tries the account key first, then the sender's stored device key when the sender
is already bound to the same user. `mint_device_binding` falls back to signing
with the local install key when no account key is stored (sibling device). Tests
cover sibling mint/verify, unbound signer rejection, cross-user rejection and
the account-key path.

The hostile-device invariant is unchanged: a device can never bind into a
*foreign* user's set, because a claim is only accepted when it verifies against
the claimed user's account key **or** against the device key of the sender, and
that sender must already be bound to the claimed `user_id`.

Implementation sketch:

- `identity::verify_device_binding_claim` checks the signature against
  `users.account_pubkey` only. Extend it (or add a sibling variant) to also try
  the **sender's** `device_bindings.device_pubkey`, gated on
  `find_user_id_for_peer(sender_peer_id) == Some(user_id)`. Trying every bound
  key of the user would work too but is broader than necessary.
- `swarm_handler::handle_device_binding` has the request sender's `peer_id` in
  hand and must pass it through to the verifier.
- `identity::mint_device_binding` currently errors without a stored account key
  (`get_stored_account_key`). Fall back to signing with the local install key;
  this needs a local-keypair accessor next to `get_local_peer_id`.
- The wire claim (`user_id ‖ device_pubkey ‖ claim_id`) does **not** need to name
  the signer — the receiver learns it from the request's sender.
- Tests: a claim signed by an already-bound sibling is accepted; a claim signed
  by an unbound key is rejected; a claim signed by a device bound to a *different*
  user is rejected; an account-key claim still works.
- Note the residual risk this accepts: a compromised bound device can add further
  devices (persistence). It already has the user's message history, so the
  usability trade is deliberate. Unbinding a device does not retroactively
  invalidate claims it signed.

Affects the Contacts tab (a user's devices collapse into one contact row) and the
multi-device story in [`identity.md`](identity.md).

---

## P1 — planned work

### Gate CI properly (CI lane) — **deferred, report-then-fail shipped**

**Decision (2026-11-01): keep CI advisory for now.** The project is early-stage
and exploratory, there is no PR workflow, and the `ci-results` branch is how an
agent without GitHub credentials reads results — a *complete* report from one
push matters more than a red X. Branch protection and a required status check
would be premature.

What shipped instead (`main.yml`):

- every check stays `continue-on-error: true`, so a failing check never skips
  the later ones;
- the results step reads each log's `Exit code:`, leads the timestamped file
  with a `SUMMARY` block, labels sections `[PASS]` / `[FAIL (exit N)]`, and
  writes the fixed-name `.github/ci-results/summary.txt`;
- a final step fails the run when `overall: FAIL`, **after** the results push,
  so the report is on `ci-results` before the run goes red.

Still open when the project moves to a PR flow:

- branch protection + a required `pipeline` status check (nothing blocks a push
  today — a red run is a signal only);
- pin the CI toolchain to the dev toolchain, or the stable/nightly clippy drift
  will produce false reds the moment the check becomes required.

### Database-level at-rest encryption

Message bodies, peer data and group state currently live in plaintext SQLite.
At-rest protection should be provided at the **database level** (encrypted
SQLite / SQLCipher-style), not per message — per-message envelopes only protect
individual fields and leave metadata, indexes and the rest of the schema
readable. This is the intended home for "data at rest" protection; the
per-message E2E lane (US-042) is not it.

- **Scope**: choose an encryption approach compatible with the existing
  `sqlite`/`sqlite_bundled` features and the Android/iOS targets; key
  derivation from the install's identity (or a user passphrase); migration of
  existing plaintext databases; interaction with `ensure_columns` and the
  migrations runner.
- **Blast radius**: `db.rs` connection setup, all four cfg variants, the
  `p2p_chat_server`/TUI startup paths, the Flutter/Android DB open path.
- **Blocked on**: a decision on key management (identity-derived vs.
  passphrase) and whether to ship SQLCipher as a bundled dep.

### Call media transport (future)

Call **signaling** shipped (un-gated, mirroring device binding). The media
transport is still to come.

- `src/calls.rs` states it plainly: *"Both sides agreed; the call is live
  (media transport lands later)."*
- The `calls` Cargo feature exists but is **empty** — it is reserved for the
  media dependencies (codecs, transport) so the default and mobile builds stay
  lean. Do **not** gate the signaling behind it; that already happened once and
  was corrected.

### Public bootstrap nodes (US-050) — **stage 1 shipped (Kademlia + Identify)**

Discovery was mDNS only, so the network was LAN-scoped by construction. Stage 1
adds internet-wide discovery via a Kademlia DHT:

- New `kad` feature (default + mobile) enabling `libp2p/kad` + `libp2p/identify`.
- `AppBehaviour` gains `kad` (MemoryStore) and `identify` behaviours.
- `handle_swarm_event` feeds Identify-received listen addresses into the DHT
  and persists them (so they can be re-dialed later).
- On connection, `kad.bootstrap()` is invoked.
- On startup, peers listed in the `P2P_BOOTSTRAP` env var (comma-separated
  multiaddrs, ideally with `/p2p/<peerid>`) are dialed.

**Stages 2-4 (shipped 2026-11-02):** NAT traversal is now wired:
- `relay` feature: circuit-relay v2 server (`relay::Behaviour`) + client
  (`relay::client::Behaviour`), with the transport wrapped via
  `with_relay_client` in `build_swarm`.
- `nat` feature: AutoNAT (`autonat::Behaviour`) and DCUtR (`dcutr::Behaviour`).
- All four behaviours are in `AppBehaviour` (cfg-gated) and have catch-all
  arms in `handle_swarm_event`.

**Stage 5 (shipped 2026-11-03): view network details.** Both frontends now
show where the node stands on the network:
- `src/net_diag.rs` records live listen/external addresses, AutoNAT status and
  probe outcomes, DCUtR hole punches, relay reservations/circuits, Kademlia
  mode/routing size/bootstrap results, and DHT announcement state.
- OS interfaces (name, MAC, IPv4/IPv6) are enumerated via `network-interface`
  and cached 5 s.
- The node publishes a rate-limited (60 s) `/p2p-chat/peer/<peer_id>` DHT
  record once it has routing peers + reachable addresses; `PutRecord`'s result
  is the announced/failed status.
- TUI: Settings-tab "Network details" block; Flutter: Settings "Network
  details" card, refreshed on open/events and every 5 s while visible. Web
  mirrors it as the `getNetworkDetails` JSON method.

Progress (2026-11-03): joined the public IPFS Kademlia DHT for discovery without
hosting our own bootstrap nodes:

- Default bootstrap list now falls back to the public IPFS bootstrap nodes when
  `P2P_BOOTSTRAP` is unset (set it to empty to disable).
- Kademlia is forced into `Mode::Client` so we route through the public DHT
  without storing/serving its record set.
- Identify protocol renamed to `/ipfs/id/1.0.0` for interoperability with
  public bootstrap nodes.
- Relay server no longer accepts reservations unless `P2P_RELAY_SERVER=1`, so
  we don't become an open relay on the public internet.

Remaining follow-ups (detailed 2026-11-03):

1. **Config surface for bootstrap/relay peers — shipped 2026-11-12**
   (`b11d03a` + `61f9f2a`). Relay peers are configurable with the `P2P_RELAY`
   env var (comma-separated multiaddrs, each needing `/p2p/<peer-id>`);
   bootstrap peers stay on `P2P_BOOTSTRAP`. On top of that, both lists are now
   persisted in the `network_peers` table and editable from Settings in the
   TUI (`b` / `r` / `x`), Flutter (a "Network peers" card) and the web
   frontend (the same JSON methods). Entries are validated as `Multiaddr`, relay
   entries must carry `/p2p/<peerid>`, and both lists are read at node start
   only. Bootstrap precedence: env var (set ⇒ wins, even empty) → persisted
   rows → public IPFS nodes with `P2P_PUBLIC_DHT=1`; relays are the union of
   env and persisted rows. The manual one-off dial is US-053 (Settings `d` /
   Flutter "Connect to address" / `dialPeer`). Record:
   [`archive/2026-11-12_network_peer_config_surface.md`](archive/2026-11-12_network_peer_config_surface.md).
2. **Relay reservation triggering — shipped 2026-11-08.** `src/relay.rs` parses
   `P2P_RELAY` and builds the `/p2p-circuit` listen address; `swarm_handler`
   listens on every configured relay at node start (which dials the relay and
   requests the reservation) and again when a configured relay connects.
   `ListenerClosed`/`ListenerError` drop the reservation and free the relay for
   a later retry, so the "Reserved via …" / "Reservation via … ended" status
   lines are accurate. Renewal is the relay client's own (the listener stays
   open), so no extra renewal code is needed.
3. **AutoNAT-driven addressing — shipped 2026-11-08.**
   `NetworkDetails::announce_addresses(self_peer_id)` replaces the old
   `reachable_addresses()` call in `maybe_announce`: when AutoNAT says
   `Private` and a reservation is held it advertises only the completed
   `/p2p/<relay>/p2p-circuit/p2p/<self>` circuit addresses (raw circuit
   listeners without the self id are dropped); otherwise the normal external
   list comes first with completed circuit addresses appended. Identify
   advertises the circuit listener automatically, so DHT peers and Identify
   peers see the same dialable address.

Items 2 and 3 are coupled (a reservation is required before there is anything
to advertise). All three follow-ups have shipped; nothing here is blocked on an
external decision.

---

## P2 — polish

### Dependency bumps — nothing urgent right now

The 2026-11-17 round cleared the last security advisories: diesel 2.4.0 widened
the SQLite range so `libsqlite3-sys 0.39.1` could land, alongside
`libp2p-gossipsub 0.50.1` and `quinn-proto 0.11.19`. `cargo audit` exits 0 with
three unfixable allowed warnings (`chrono`, `paste`, `yoke-derive`).

`cargo outdated --depth 1` now prints a table again (diesel no longer blocks
the probe) with eight direct rows. None is a security advisory: two are
deliberate pins (`libp2p-identity` 0.3.0, dev-only `tokio-tungstenite` 0.29),
a few are compatible patches, and the rest are semver-incompatible majors.
Re-run `cargo outdated --depth 1` before assuming this lane is empty.

Details: [`dependencies.md`](dependencies.md).

### LAN-only mode toggle (US-051)

A switch that disables any non-LAN transport. Cheap in principle, but it needs
the transport layer to be separable from discovery.

### Dark / light theme (US-025) and custom color scheme (US-026)

The TUI uses ratatui styles directly and Flutter uses Material themes. A theme
toggle means threading a palette through both renderers — the TUI is the
cheaper half.

### Notification sounds (US-027) and desktop notifications (US-028)

Neither frontend has a notification dependency today. Android already runs a
foreground service, so a system notification is the smaller half; the TUI has
no notification backend.

---

## Known limitations (by design — not backlog items)

These are deliberate properties of the current architecture. They are recorded
here so they are not mistaken for unfinished work.

| Limitation | Why |
|---|---|
| Group backfill only reaches members that were online | There is no external store-and-forward; a backfill answer is served by a history-holding member that is currently connected. See `archive/2026-09-02_backlog.md` §11.5. |
| No message sync between a user's own devices | A user's devices are separate peers; syncing them needs one of the user's own devices online within the gap window. Out of scope by decision (`archive/2026-09-02_backlog.md` §11). |
| Calls are signaling-only | Media transport is not implemented; see US-044 above. |
| The browser is a display client, never a node | The Rust core is not compiled to WASM. By decision — see [`web_frontend.md`](web_frontend.md). |
| DMs are readable by a store-and-forward holder | US-042 seals DM bodies per recipient; a holder sees only the envelope (see `e2e_encryption.md`). |
| The web frontend's HTTP Basic credentials are not encrypted on the wire | The server has no TLS of its own (it is a LAN display client). Put a TLS-terminating reverse proxy in front for any exposure beyond a trusted LAN; the browser-native credential dialog and the session cookie already keep the secret out of URLs, history and logs. |
| `cargo outdated` reports eight direct rows | Informational (exit 0 by default); none is a security advisory. Two deliberate pins, a few compatible patches, and a few semver-incompatible majors — see the dependency lane above. |

---

## Closed recently

| Item | When | Where |
|---|---|---|
| Message reactions (US-008) | 2026-11-21 | [`archive/2026-11-21_message_reactions.md`](archive/2026-11-21_message_reactions.md) |
| DM typing indicators (US-009) | 2026-11-21 | [`archive/2026-11-21_typing_indicators.md`](archive/2026-11-21_typing_indicators.md) |
| Local sent-message edits (US-005, edit half) | 2026-11-20 | [`archive/2026-11-20_local_message_edits.md`](archive/2026-11-20_local_message_edits.md) |
| Store-and-forward DM outbox (sealed envelopes, retry on reconnect, dedup, expiry) | 2026-11-19 | [`archive/2026-11-19_store_and_forward_dm_outbox.md`](archive/2026-11-19_store_and_forward_dm_outbox.md) |
| Reply / quote to a specific message (US-006) | 2026-11-18 | [`archive/2026-11-18_reply_quote.md`](archive/2026-11-18_reply_quote.md) |
| Keyboard shortcut help overlay (US-029) + Flutter help dialog | 2026-11-17 | [`archive/2026-11-17_keyboard_shortcuts_help.md`](archive/2026-11-17_keyboard_shortcuts_help.md) |
| Clear the remaining audit advisories (diesel 2.4.0, libsqlite3-sys 0.39.1, gossipsub, quinn-proto) | 2026-11-17 | `227f214` |
| CI checks report the checked command's exit code (no `tee` false PASS) | 2026-11-17 | `1835967` |
| Peer blocklist / mute (US-054) | 2026-11-12 | [`archive/2026-11-12_peer_blocklist.md`](archive/2026-11-12_peer_blocklist.md) |
| Manual peer address entry (US-053) | 2026-11-02 | `efa5e46` |
| Connection quality indicators (US-052) | 2026-11-08 | [`archive/2026-11-08_connection_quality.md`](archive/2026-11-08_connection_quality.md) |
| `sha2` 0.10 → 0.11.1 (attachment `file_id` digest, no source change) | 2026-11-01 | `deps: take sha2 0.11` |
| `tower-http` 0.6 → 0.7 (web server static files + CORS) | 2026-10-31 | `deps: take tower-http 0.7` |
| `libp2p` 0.57.0 → 0.57.1 (deliberate pin bump) | 2026-10-30 | `deps: move the libp2p pin to 0.57.1` |
| `libsqlite3-sys` 0.36 → 0.38 via diesel 2.3.14; `ctutils`/`tokio` advisory bumps | 2026-10-29 | `deps: take libsqlite3-sys 0.38 and clear two advisories` |
| Human-readable attachment sizes (`2 KiB`, `64 MiB`) | 2026-10-26 | [`archive/2026-10-26_attachment_size_labels.md`](archive/2026-10-26_attachment_size_labels.md) |
| Live-reference accuracy pass (identity, architecture, files, web, README) | 2026-10-26 | `docs: correct the live references against the code` |
| Documentation restructure: archive, live docs, one backlog, link checker | 2026-10-21 | `ce28979` … `604ff0c` |
| Contacts search in both frontends | 2026-10-21 | [`archive/2026-10-21_contacts_search.md`](archive/2026-10-21_contacts_search.md) |
| Peer Info stats rows | 2026-10-20 | [`archive/2026-10-20_peerinfo_stats_rows.md`](archive/2026-10-20_peerinfo_stats_rows.md) |
| Per-message latency in Flutter + web | 2026-10-20 | [`archive/2026-10-20_message_latency_parity.md`](archive/2026-10-20_message_latency_parity.md) |
| Log-event parity (`file_received` / `peer_expired`) | 2026-10-20 | [`archive/2026-10-20_log_event_parity.md`](archive/2026-10-20_log_event_parity.md) |
| Delivery-receipt ticks in Flutter + web | 2026-10-20 | [`archive/2026-10-20_delivery_receipts_parity.md`](archive/2026-10-20_delivery_receipts_parity.md) |
| File attachments over chat | 2026-10-04 | [`files_attachments.md`](files_attachments.md) |
| Web frontend (Flutter Web + Rust node server) | 2026-10-18 | [`web_frontend.md`](web_frontend.md) |
| Identity, device binding, contacts, calls signaling | 2026-09-21..09-26 | [`identity.md`](identity.md) |
| rustls 0.23.46 / rustls-webpki 0.103.16 (5 advisories cleared) | 2026-10-21 | `3e5beba` |
| `scripts/build-all.sh` (TUI + web in one command) | 2026-10-21 | `523790e` |
