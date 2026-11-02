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

### End-to-end encryption for direct messages (US-042) — **partial: scaffolding only**

Transport encryption exists (libp2p `noise`), but a relay or the receiving
node sees plaintext message bodies. E2E would need a per-user key, a key
exchange over the existing DM lane, and a per-message envelope.

**What landed (2026-11-02, commit pending):** the crypto module
(`src/e2e.rs`: X25519 from the Ed25519 identity, recipient key recovered from
the peer ID, HKDF-SHA256 + ChaCha20-Poly1305 envelope, `msg_id` in the AAD),
the `DirectMessage.encrypted` wire field, the `MessageEvent.encrypted` flag,
the `messages.encrypted` column, `ChatMessage.encrypted` (mobile) and the TUI
`[e2e] ` marker plumbing. All of it compiles and the tests pass.

**What is NOT wired:** `swarm_handler::send_dm` still sends plaintext and
`handle_request_response` never opens an envelope, so no DM is encrypted end to
end today. See `docs/e2e_encryption.md`.

- **Rationale (updated 2026-11-02):** at-rest protection for message bodies is
  **not** a goal of this lane — it should be achieved at the **database level**
  (encrypted SQLite / SQLCipher-style), not per message. The remaining
  justification for the per-recipient envelope is **store-and-forward / offline
  caching**: a third node holding and forwarding a sealed blob it must not be
  able to read. That use case is tracked separately (see the new backlog item
  below); until it is implemented, this lane's value is limited to defense in
  depth. The scheme also has no forward secrecy or post-compromise security.
- **Scope sketch (if pursued):** reuse the per-install Ed25519 identity (not
  the account key — sibling devices don't hold it); seal in `send_dm`; open in
  `handle_request_response`; persist the flag; surface a lock marker in Flutter.

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

### Store-and-forward / offline caching for direct messages

A peer that is offline cannot receive a DM. Store-and-forward would let a
mutually-trusted (or untrusted) intermediate node hold a sealed blob and
deliver it when the recipient reconnects — the use case the per-recipient
envelope in `src/e2e.rs` (US-042) was designed for: the holder must not be
able to read the body.

- **Scope**: a relay/holder protocol (request_response or gossipsub-backed),
  retention/expiry policy, dedup on delivery, and the recipient-side open path
  (`e2e::open_incoming`) once US-042 is wired.
- **Depends on**: US-042 being completed (seal in `send_dm`, open in
  `handle_request_response`) so the forwarded bytes are actually sealed.
- **Trust model**: holders see only routing fields (`msg_id`, `sent_at`) and
  the ciphertext; a compromised holder can drop or delay, not read.

Call **signaling** shipped (un-gated, mirroring device binding). The media
transport is still to come.

- `src/calls.rs` states it plainly: *"Both sides agreed; the call is live
  (media transport lands later)."*
- The `calls` Cargo feature exists but is **empty** — it is reserved for the
  media dependencies (codecs, transport) so the default and mobile builds stay
  lean. Do **not** gate the signaling behind it; that already happened once and
  was corrected.

### Reply / quote to a specific message (US-006)

`content` is currently a flat string. A reply needs the replied-to message id
carried in the wire payload and rendered as a quoted header in all three
frontends (TUI, Flutter, web).

### Typing indicators (US-009)

Requires a lightweight "user X is typing" signal on the DM lane. The lane is
`request_response`; a typing signal is fire-and-forget and must not occupy a
request slot.

### Edit or delete a sent message (US-005)

Deletion is the cheaper half: tombstone the row and re-render. Note that
`confirmed`/`latency` are live decorations, not persisted columns
(see `archive/2026-10-20_delivery_receipts_parity.md`), so a tombstone has to
keep the receipt row consistent with the visible message.

### Message reactions (US-008)

A per-message reaction set. Needs a new table plus a wire type; reactions are
small, so a gossipsub topic is viable, but the DM lane is the simpler first
step.

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

Still to do (later stages): hosted relay nodes (circuit relay v2) for NAT
traversal, AutoNAT reachability detection, DCUtR hole punching, and a config
surface for bootstrap peers beyond the env var.

### Manual peer address entry (US-053)

No way to dial a peer by multiaddr today. A "connect to address" input on the
Settings tab would need a `SwarmCommand::Dial` and a peer address field.

### Peer blocklist / mute (US-054)

A muted peer should be ignored at the swarm boundary (don't persist its
messages, don't dial it) rather than merely hidden in the UI.

---

## P2 — polish

### Dependency bumps — nothing actionable right now

Nothing actionable is left. `cargo outdated --depth 1` reports two direct rows,
both **held on purpose**, not work:

- `libsqlite3-sys` is at 0.38.2, the newest release diesel 2.3.14 accepts
  (`>=0.17.2, <0.39.0`); 0.39.1 needs another upstream diesel bump, so nothing
  in this repo can move it.
- `tokio-tungstenite` stays at 0.29 to match axum's `ws` feature. 0.30 is
  dev-only but pulls a second `tungstenite` tree; the bump was built, verified
  green, and backed out because it buys nothing for a test-only client.

The last actionable row, `sha2 0.10 → 0.11.1`, closed in the
2026-10-29 → 11-01 dependency round with no source change (the digest stays
byte-identical; see `dependencies.md`). Re-run `cargo outdated --depth 1` before
assuming this lane is empty.

Details: [`dependencies.md`](dependencies.md).

### Connection quality indicators (US-052)

The TUI already has a `ConnectedTracker` and records per-peer sessions, so the
data exists. What is missing is a per-peer quality read (packet loss / RTT) and
its display.

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

### Keyboard shortcut help overlay (US-029)

What shipped: the TUI renders a tab-aware shortcut bar
(`src/tui_render/mod.rs` → `render_shortcuts`), documenting only the bindings
that are live on the current tab, and
[`KEYBOARD_SHORTCUTS.md`](../KEYBOARD_SHORTCUTS.md) is the full reference.

What is missing: a modal `?` overlay showing the whole table at once. The
status bar covers the common case; the overlay is nice-to-have, and it would
have to share the popup slot with the device-binding and group-leave prompts
(see the popup-precedence rule in `AGENTS.md`).

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
| DM bodies are plaintext to the receiving node | Transport is encrypted, but there is no E2E layer; see US-042 above. |
| The web frontend's HTTP Basic credentials are not encrypted on the wire | The server has no TLS of its own (it is a LAN display client). Put a TLS-terminating reverse proxy in front for any exposure beyond a trusted LAN; the browser-native credential dialog and the session cookie already keep the secret out of URLs, history and logs. |
| `cargo outdated` reports two direct rows | Both are held on purpose (see the dependency lane above): `libsqlite3-sys` 0.38.2 → 0.39.1 is a tracker for a future diesel release, and `tokio-tungstenite` 0.29 → 0.30 is pinned to axum's copy so the tree keeps one `tungstenite`. |

---

## Closed recently

| Item | When | Where |
|---|---|---|
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
