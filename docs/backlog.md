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

### Sibling-device claim authority (identity lane)

Decide **who may add a device to a user's device set** without re-provisioning:
only the account key (today's rule), or any already-bound device of that user?

- The hostile-device invariant is **not** in question — a device can never bind
  into a *foreign* user's set, because that needs the foreign account key. This
  question is only about the *same* user's set.
- Today `mint_device_binding` is account-holder-only.
- Affects the Contacts tab (a user's devices collapse into one contact row) and
  the multi-device story in [`identity.md`](identity.md).

---

## P1 — planned work

### Make CI gate a push (CI lane)

Every step in `.github/workflows/main.yml` is `continue-on-error: true`, so a
failing test, a clippy warning or a broken doc link does **not** turn the
workflow red — the consolidated results file is the artifact, not the exit
status. The policy in `AGENTS.md` ("`cargo ct` must exit 0 with no warnings")
is enforced by convention and by the local checklist, not by CI. The only
hard gate in the repo is `cargo audit` in `dependencies.yml`.

- **Decision needed**: hard-fail on push, or keep the advisory model and
  rely on branch protection + a required status check that does not exist.
- **Cost**: the workflow currently also collects results for *informational*
  steps; a hard gate means picking which steps are blocking.
- **Note**: `cargo test --all-features` now runs on push (added 2026-10-21);
  it is advisory like the rest.

### End-to-end encryption for direct messages (US-042)

Transport encryption exists (libp2p `noise`), but a relay or the receiving
node sees plaintext message bodies. E2E would need a per-user key, a key
exchange over the existing DM lane, and a per-message envelope.

- **Scope sketch**: reuse the `users.account_pubkey` row; derive a messaging
  keypair; encrypt the DM payload before it hits the wire; surface
  "encrypted" in both frontends.
- **Blast radius**: the DM protocol codec, the `messages` table (ciphertext vs
  plaintext), and every frontend's message rendering.

### Voice/video call media (US-044)

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

### Public bootstrap nodes (US-050)

Discovery is mDNS only, so the network is LAN-scoped by construction. A
bootstrap node would need a Kademlia or rendezvous behaviour — a new libp2p
behaviour and a config surface for the node address.

### Manual peer address entry (US-053)

No way to dial a peer by multiaddr today. A "connect to address" input on the
Settings tab would need a `SwarmCommand::Dial` and a peer address field.

### Peer blocklist / mute (US-054)

A muted peer should be ignored at the swarm boundary (don't persist its
messages, don't dial it) rather than merely hidden in the UI.

---

## P2 — polish

### Routine dependency bumps that need a manifest change

`cargo outdated --depth 1` reports one actionable row whose newer version is
outside the current requirement, so the manifest has to move (the lockfile-only
rows, the pinned `libp2p` 0.57.1 bump and `tower-http` 0.7.1 were cleared on
2026-10-29 — see `dependencies.md`):

| Crate | Locked | Newer | Note |
|---|---|---|---|
| `sha2` | 0.10.9 | 0.11.1 | Major bump — the attachment `file_id` digest must keep producing the same hex. |

Two further rows are **held on purpose**, not work:

- `libsqlite3-sys` is at 0.38.2, the newest release diesel 2.3.14 accepts
  (`>=0.17.2, <0.39.0`); 0.39.1 needs another upstream diesel bump, so nothing
  in this repo can move it.
- `tokio-tungstenite` stays at 0.29 to match axum's `ws` feature. 0.30 is
  dev-only but pulls a second `tungstenite` tree; the bump was built, verified
  green, and backed out because it buys nothing for a test-only client.

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
| `cargo outdated` reports three direct rows | One is a routine bump (see the dependency lane above). The other two are held on purpose: `libsqlite3-sys` 0.38.2 → 0.39.1 is a tracker for a future diesel release, and `tokio-tungstenite` 0.29 → 0.30 is pinned to axum's copy so the tree keeps one `tungstenite`. |

---

## Closed recently

| Item | When | Where |
|---|---|---|
| `tower-http` 0.6 → 0.7 (web server static files + CORS) | 2026-10-29 | `deps: take tower-http 0.7` |
| `libp2p` 0.57.0 → 0.57.1 (deliberate pin bump) | 2026-10-29 | `deps: move the libp2p pin to 0.57.1` |
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
