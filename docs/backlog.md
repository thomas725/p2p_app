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

### `libsqlite3-sys` 0.36 → 0.38 (dependency lane) — **blocked**

`cargo outdated --depth 1` currently reports **nine** direct rows (the full
breakdown is in [`dependencies.md`](dependencies.md)): four are lockfile-only,
four need a routine manifest bump, and this one is the only row **no change
in this repo can move**. Diesel's latest 2.x (2.3.13) still pins
`libsqlite3-sys` 0.36, and 0.36 is the version that blocks the 0.38 line.

- **Blocker**: upstream diesel. Nothing in this repo can move it.
- **Do not** restructure to diesel's `sqlite-bundled` — that was evaluated and
  **rejected** (see `archive/2026-09-02_backlog.md` §7, "libsqlite3-sys
  bundled restructure"). The current `sqlite_bundled` feature already scopes
  bundling correctly: Android needs a bundled sqlite, and the desktop TUI
  links the system one.
- **Watch for**: a diesel 2.4 that allows `libsqlite3-sys` 0.38.

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

`cargo outdated --depth 1` reports four rows whose newer version is outside
the current requirement, so the manifest has to move (the other four direct
rows are lockfile-only and clear with a plain `cargo update`):

| Crate | Locked | Newer | Note |
|---|---|---|---|
| `libp2p` | 0.57.0 | 0.57.1 | The `=0.57.0` pin is deliberate; bump it deliberately and re-read the protocol docs. |
| `sha2` | 0.10.9 | 0.11.1 | Major bump — the attachment `file_id` digest must keep producing the same hex. |
| `tower-http` | 0.6.11 | 0.7.1 | Major bump — the web server's static-file and CORS wiring. |
| `tokio-tungstenite` | 0.29.0 | 0.30.0 | Dev-only; the web server's WebSocket test client. |

Details: [`dependencies.md`](dependencies.md).

### Human-readable attachment sizes (files lane)

Both frontends render the chip with **raw bytes** (`[file] report.pdf (1048576)`).
A `1.0 MiB` suffix is a small, self-contained change in `files::file_chip` plus
the Flutter `_AttachmentChip`.

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

### Keyboard shortcut help overlay (US-029) — **shipped, doc pending**

The TUI renders a tab-aware shortcut bar (`src/tui_render/mod.rs`), and
[`KEYBOARD_SHORTCUTS.md`](../KEYBOARD_SHORTCUTS.md) is the full reference. A
modal overlay (a `?` key that opens a popup with the whole table) is still
nice-to-have; the story is *mostly* satisfied by the status bar.

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
| `cargo outdated` reports nine direct rows | Eight are routine bumps (see the dependency lane above); the `libsqlite3-sys` row is a tracker for a future diesel release, not a defect. |

---

## Closed recently

| Item | When | Where |
|---|---|---|
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
