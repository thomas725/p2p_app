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

None open. The former sibling-device claim authority item shipped as `ede126b`
and is recorded in
[`archive/2026-11-02_sibling_device_claims.md`](archive/2026-11-02_sibling_device_claims.md).

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

### Call media transport — **transport shipped; browser + Android + TUI adapters shipped**

The transport half of US-044 shipped: G.711 μ-law voice frames over a dedicated
`/p2p-chat/call-media/1.0.0` lane, with Rust owning the session, jitter buffer
and mute state in `src/call_media.rs`. Both frontends hand raw 16 kHz mono PCM
to Rust and receive decoded PCM back. Full record:
[`archive/2026-11-22_call_media_transport.md`](archive/2026-11-22_call_media_transport.md).

Two Flutter device adapters now exist under
`apps/flutter_app/lib/src/util/call_audio*.dart`, both wired in `CallScreen`:

- **Flutter web browser** — `getUserMedia` + Web Audio capture resampled to
  16 kHz mono i16 frames, and `AudioBufferSourceNode` playback. Raw
  `dart:js_interop`, no new dependencies. Capture needs a secure context
  (HTTPS/localhost), so an HTTP LAN deployment gets playback only plus an
  actionable capture-error line.
- **Android** — `CallAudio.kt` drives `AudioRecord`/`AudioTrack` at the wire rate
  (16 kHz mono, no resampler) over two method channels (commands Dart->native,
  `pcm` events native->Dart). `RECORD_AUDIO` is requested at call start; denial
  still leaves playback running and shows an actionable error. Capture is
  activity-scoped — backgrounding the app ends it rather than leaving the mic
  open (no microphone foreground-service type).

- **TUI (opt-in)** — `src/bin/tui/call_audio.rs` drives `cpal`
  capture/playback on a dedicated thread, resampling the device rate to the
  16 kHz mono wire format and back. It lives behind the new `tui-audio`
  feature, which is deliberately **not** in `default` or `tui`: without it the
  TUI compiles a no-op handle (the frame counter remains the transport signal),
  and the headless server and Android library compile no audio code or `cpal`
  dependency at all. `scripts/build-all.sh --tui-audio` enables it for the TUI
  only and builds the server separately; Linux needs `libasound2-dev`.

Rust still owns framing, jitter and mute in every case; the adapters only move
PCM.

Still open (device adapters, not protocol work):

- **iOS / native desktop Flutter audio** — `call_audio_platform.dart` is still a
  no-op stub off Android; these targets get transport only.
- **Background mic on Android** — would need a microphone-type foreground
  service (and its notification), deliberately not part of the activity-scoped
  adapter.
- The `calls` Cargo feature is still **empty** — signaling and transport are
  both un-gated. Do **not** gate either behind it; that happened once and was
  corrected.

### File shares / folder sync UI — **Rust core + Flutter/web list, detail, grant editor, discovery and network actions shipped; creation/binding/publishing deferred**

The Rust core landed in `7dfaefe` (domain, DB, ACLs) and the share-lane
follow-up (`/p2p-chat/shares/1.0.0` wire protocol, swarm handlers, events, GC
hook). Full design and invariants: [`files_attachments.md`](files_attachments.md).

Shipped in core:

- private shares keyed on verified `user_id` device bindings, with per-user
  read-only/read-write grants; public shares are readable by any connected peer
  and write is opt-in;
- versioned `ShareManifest`s over the existing content-addressed blob store,
  with `base_revision` rejection of stale writes;
- snapshots and manual refresh only — no background filesystem watcher in the
  first version;
- path validation (no absolute paths, `..`, symlink escapes), apply only to a
  bound real local folder, and deletion only of previously tracked files;
- blocklist-gated `request_response` lane, fail-closed ACLs, and deferred
  application once all manifest blobs arrive.

Shipped after the core:

- FRB/web facade parity for create/list/bind/delete/snapshot/grants, manifest
  load/save, offer acceptance and the four share commands, mirrored in the Dart
  `NodeApi`/`FrbNodeApi`/`HttpNodeApi` transports. Web clients cannot pass a
  server-local path; `bindShare` is rejected outright (a null root could clear a
  native binding), and only native installs can snapshot a folder. Share objects
  use an explicit camelCase DTO boundary (`src/shares_json.rs`) for the web and
  mobile event transports, keeping the domain/wire types snake_case;
- correctness hardening: received manifests are monotonic (a lower revision or
  unknown share is rejected), `accept_submitted_manifest` checks ACL and base
  revision in one transaction, untrusted offers can no longer rewrite the ACL,
  root or revision of a share we own/bound/have materialised, manifests with all
  blobs local apply immediately, blob-arrival applies are attributed to the
  delivering peer, `delete_share` is transactional and drops in-flight applies,
  and a failed file install no longer strands a `.part` staging file;
- wire-level coverage for offer → list → manifest → stale rejection → retry
  with the refreshed revision, plus unit coverage for grant revocation,
  superseded/deferred applies, disconnect cleanup, spoofed offers, manifest
  rollbacks, immediate applies and staging-file cleanup;
- a read-only Flutter/web share list and detail route (Settings → File shares),
  refreshed from `share_*` events over the existing `NodeApi` facade; the detail
  page shows overview, private-share grants and manifest files. Widget tests
  cover both routes without FFI. Record:
  [`archive/2026-11-24_flutter_readonly_shares_ui.md`](archive/2026-11-24_flutter_readonly_shares_ui.md).
- a private-share permission editor on that detail route (add/revoke grants with
  the canonical `readonly`/`readwrite` modes, inline failures), wired through
  the existing `NodeApi` grant methods on both transports. Its callbacks are
  optional, so the route stays read-only in FFI-free widget tests; widget
  coverage includes add, revoke, empty-ID and failure paths. Record:
  [`archive/2026-11-24_flutter_share_grant_editor.md`](archive/2026-11-24_flutter_share_grant_editor.md).
- detail-level network actions on the same route: re-request the manifest and
  re-advertise a canonical offer from a selected connected peer. The offer is
  rebuilt from the share row (no local owner sentinel), stale peer selections
  reset when the roster changes, and failures appear inline. Record:
  [`archive/2026-11-26_flutter_share_network_actions.md`](archive/2026-11-26_flutter_share_network_actions.md).
- peer-share discovery from the share list: ask a connected peer for its
  offers, keep them keyed by peer/share id in a notifier, and explicitly accept
  one into the local list. The advertised owner is shown as unverified;
  malformed event payloads are dropped. Record:
  [`archive/2026-11-26_flutter_share_discovery.md`](archive/2026-11-26_flutter_share_discovery.md).

Still open (UI/integration, not protocol work):

- TUI Shares tab: create/bind/snapshot, grant editor, public/write toggles,
  offer/manifest/publish actions, and visible update/rejection events;
- Flutter/web share publishing. Discovery, manifest refresh and re-advertise
  already ship on both transports; the browser remains a display client — only
  native installs can bind a real local folder, and a browser has no new
  filesystem snapshot to publish.

The two-node blob-pull → apply test shipped with separate per-thread databases
and blob stores; see
[`archive/2026-11-24_two_node_share_blob_pull.md`](archive/2026-11-24_two_node_share_blob_pull.md).

### Public bootstrap nodes (US-050) — **shipped**

All stages (Kademlia + Identify, NAT traversal, network details view, public
IPFS DHT) and all three follow-ups (config surface, relay reservation
triggering, AutoNAT-driven addressing) have shipped. The full record is in
[`archive/2026-11-12_public_bootstrap_discovery.md`](archive/2026-11-12_public_bootstrap_discovery.md);
the manual one-off dial (US-053) and the network-peer config surface have their
own archive entries.

---

## P2 — polish

### Dependency bumps — nothing urgent right now

The 2026-11-17 round cleared the last security advisories: diesel 2.4.0 widened
the SQLite range so `libsqlite3-sys 0.39.1` could land, alongside
`libp2p-gossipsub 0.50.1` and `quinn-proto 0.11.19`. `cargo audit` exits 0 with
three unfixable allowed warnings (`chrono`, `paste`, `yoke-derive`).

`cargo outdated --depth 1` now prints a table again (diesel no longer blocks
the probe) with five direct rows after the 2026-11-22 round took the last
compatible patches (`clap`, `ratatui-textarea`, `unicode-width`). None is a
security advisory: two are deliberate pins (`libp2p-identity` 0.3.0, dev-only
`tokio-tungstenite` 0.29), `base64` is held on libp2p's line, and two are
semver-incompatible majors. Re-run `cargo outdated --depth 1` before assuming
this lane is empty.

Details: [`dependencies.md`](dependencies.md).

### Notification sounds (US-027) and desktop notifications (US-028) — **shipped**

Rust owns the notification payload, policy and display text; the TUI, Android
and browser are thin delivery adapters with one shared focus-suppression rule.
Full record: [`archive/2026-11-21_notifications.md`](archive/2026-11-21_notifications.md).

---

## Known limitations (by design — not backlog items)

These are deliberate properties of the current architecture. They are recorded
here so they are not mistaken for unfinished work.

| Limitation | Why |
|---|---|
| Group backfill only reaches members that were online | There is no external store-and-forward; a backfill answer is served by a history-holding member that is currently connected. See `archive/2026-09-02_backlog.md` §11.5. |
| No message sync between a user's own devices | A user's devices are separate peers; syncing them needs one of the user's own devices online within the gap window. Out of scope by decision (`archive/2026-09-02_backlog.md` §11). |
| Calls have no device audio on iOS / native desktop | The media transport ships (μ-law frames over the calls lane); Flutter web and Android have real mic/speaker adapters, and the TUI has an opt-in `cpal` adapter (`tui-audio`). Non-Android native Flutter targets still have transport only (see US-044 above). |
| The browser is a display client, never a node | The Rust core is not compiled to WASM. By decision — see [`web_frontend.md`](web_frontend.md). |
| Native Flutter desktop builds deliver no notification sound/system alert | The primary desktop path is the browser frontend, which delivers both channels through the WebAudio/`Notification` adapters. Android uses the MethodChannel. The `flutter build linux` artifact is currently a display shell without a native notification plugin; TUI users get bell/OSC 777. |
| DMs are readable by a store-and-forward holder | US-042 seals DM bodies per recipient; a holder sees only the envelope (see `e2e_encryption.md`). |
| The web frontend's HTTP Basic credentials are not encrypted on the wire | The server has no TLS of its own (it is a LAN display client). Put a TLS-terminating reverse proxy in front for any exposure beyond a trusted LAN; the browser-native credential dialog and the session cookie already keep the secret out of URLs, history and logs. |
| `cargo outdated` reports five direct rows | Informational (exit 0 by default); none is a security advisory. Two deliberate pins, `base64` held on libp2p's line, and two semver-incompatible majors — see the dependency lane above. |

---

## Closed recently

| Item | When | Where |
|---|---|---|
| Flutter/web peer-share discovery + explicit acceptance | 2026-11-26 | [`archive/2026-11-26_flutter_share_discovery.md`](archive/2026-11-26_flutter_share_discovery.md) |
| Flutter/web share manifest refresh + re-advertise | 2026-11-26 | [`archive/2026-11-26_flutter_share_network_actions.md`](archive/2026-11-26_flutter_share_network_actions.md) |
| Flutter/web private-share grant editor | 2026-11-24 | [`archive/2026-11-24_flutter_share_grant_editor.md`](archive/2026-11-24_flutter_share_grant_editor.md) |
| Read-only Flutter/web share list + detail | 2026-11-24 | [`archive/2026-11-24_flutter_readonly_shares_ui.md`](archive/2026-11-24_flutter_readonly_shares_ui.md) |
| Two-node share blob pull → folder apply test | 2026-11-24 | [`archive/2026-11-24_two_node_share_blob_pull.md`](archive/2026-11-24_two_node_share_blob_pull.md) |
| Share offers stop leaking the local unknown-owner marker | 2026-11-24 | [`archive/2026-11-24_share_offer_owner_marker.md`](archive/2026-11-24_share_offer_owner_marker.md) |
| Shared DB-backed broadcast unread watermark | 2026-11-23 | [`archive/2026-11-23_broadcast_unread_watermark.md`](archive/2026-11-23_broadcast_unread_watermark.md) |
| TUI DM/group unread banners + scroll fidelity | 2026-11-23 | [`archive/2026-11-23_tui_dm_group_unread.md`](archive/2026-11-23_tui_dm_group_unread.md) |
| TUI Log "Copy All" parity (`Ctrl+C`, OSC 52) | 2026-11-23 | [`archive/2026-11-23_tui_log_copy_all.md`](archive/2026-11-23_tui_log_copy_all.md) |
| TUI clipboard parity (`Ctrl+C`, OSC 52) | 2026-11-23 | [`archive/2026-11-23_tui_clipboard.md`](archive/2026-11-23_tui_clipboard.md) |
| Notification sounds + desktop alerts (US-027/US-028) | 2026-11-21 | [`archive/2026-11-21_notifications.md`](archive/2026-11-21_notifications.md) |
| LAN-only mode toggle (US-051) | 2026-11-21 | [`archive/2026-11-21_lan_only_mode.md`](archive/2026-11-21_lan_only_mode.md) |
| Public bootstrap nodes / internet discovery (US-050) | 2026-11-12 | [`archive/2026-11-12_public_bootstrap_discovery.md`](archive/2026-11-12_public_bootstrap_discovery.md) |
| Sibling-device claim authority (P0 identity) | 2026-11-02 | [`archive/2026-11-02_sibling_device_claims.md`](archive/2026-11-02_sibling_device_claims.md) |
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
