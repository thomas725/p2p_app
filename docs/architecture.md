# Architecture overview

How the system fits together. For *what to build next* see
[backlog.md](backlog.md); for *rules* see [../AGENTS.md](../AGENTS.md).

## What it is

A serverless P2P chat application. Every participant is both a client and a
node: there is no central server. Peers find each other over mDNS on the LAN by
default; Kademlia DHT bootstrap and relay-based NAT traversal are opt-in (see
[network entry points](#network-entry-points-bootstrap--relays)). Persistence is
a local SQLite file (Diesel). The app is a thin shell over a Rust core that all
frontends share.

## One core, four frontends

The Rust core (`src/`) owns the protocol, the database, and every decision
that must not be duplicated. The frontends only render and forward input.

| Frontend | Transport | Notes |
|---|---|---|
| **TUI** (ratatui + crossterm) | in-process | Terminal frontend. `src/bin/tui/`. |
| **Flutter mobile** (Android) | in-process FFI | `flutter_rust_bridge` 2.13. Android foreground service holds the mDNS multicast lock. |
| **Flutter desktop** (Linux) | in-process FFI | Same Flutter app as mobile, built for Linux. |
| **Flutter Web** | JSON over WebSocket | A **display client** — the node stays native in the Rust web server. See [web_frontend.md](web_frontend.md). |

The rule that follows from this: **one Rust function is the single source of
truth**, and every frontend calls it. A value that is computed in Dart or
pushed into the TUI is a bug waiting to happen. The only sanctioned exception
is filtering a list the frontend already holds (there is no binding that
filters a Dart list), which is why the Contacts search rule is spelled out in
`AGENTS.md`.

## Binaries

| Binary | Path | Features | What it is |
|---|---|---|---|
| `p2p_chat` | `src/bin/p2p_chat.rs` | any | Headless CLI. Reads stdin, writes stderr. |
| `p2p_chat_tui` | `src/bin/p2p_chat_tui.rs` | `tui` | The terminal frontend. |
| `p2p_chat_server` | `src/bin/p2p_chat_server.rs` | `web` | Node + axum server hosting the web frontend. |

`p2p_chat_tui` and `p2p_chat_server` declare `required-features`, so cargo
skips them when the feature is off. `default-run` is `p2p_chat_tui`.

## Module layout

```
src/
├── lib.rs              crate root; module wiring + re-exports
├── behavior.rs         libp2p behaviours: gossipsub, request_response, mdns, ping
├── swarm_handler.rs    libp2p events -> app events; inbound persistence
├── network.rs          network-size classification (Small/Medium/Large)
├── net_diag.rs         live network diagnostics snapshot (interfaces, NAT, relay, DHT,
│                       per-peer ping quality)
├── relay.rs            circuit-relay config + relayed listen address (P2P_RELAY)
├── types.rs            shared constants and enums (SwarmEvent, SwarmCommand, InputMode)
├── db.rs               connection handling, multi-instance DB resolution, ensure_columns
├── dm_outbox.rs        sealed-DM store-and-forward outbox (queue/retry/expiry)
├── messages.rs         message + receipt persistence, per-peer stats
├── peers.rs            peer rows, sessions, broadcast recipients
├── blocklist.rs        peer blocklist / mute (swarm-boundary filtering)
├── nickname.rs         display-name resolution + memoization
├── fmt.rs              canonical formatting (parse_last_seen_ms, peer_id_suffix, ...)
├── groups.rs           groups, members, group messages, backfill watermark
├── identity.rs         users, account keys, device bindings, signed claims
├── contacts.rs         the contact roster
├── calls.rs            call session model (caller-minted call_id, state machine)
├── call_media.rs       voice media transport: μ-law frames, jitter buffer, mute (US-044)
├── files.rs            content-addressed attachment store
├── shares.rs           versioned file shares / folder sync, grants, ACLs
├── connected.rs        ConnectedTracker
├── logging.rs          log helpers
├── network_config.rs   persisted bootstrap / relay multiaddrs (Settings, node start)
├── e2e.rs              DM envelope seal/open (X25519 → ChaCha20-Poly1305)
├── reactions.rs        per-message reaction persistence + summaries
├── reply.rs            canonical bounded reply preview + quoted header
├── notifications.rs    Rust-owned notification payload, policy, terminal sequence
├── settings.rs         typed app_settings readers/writers
├── theme.rs            shared theme tokens
│
├── api/mod.rs          FRB surface (mobile) — the canonical binding layer
├── mobile_api.rs       FRB-exposed helpers (sorting, formatting, stats)
├── mobile_node.rs      node lifecycle for the mobile/web server path
│
├── tui_tabs.rs         tab model (fixed + dynamic tabs)
├── tui_helpers.rs      pure table/sort/window math
├── tui_render_state.rs render-side state
├── tui_render/         ratatui renderers (mod, chat, groups, log, peer_info, peers, settings)
│
├── web/                axum HTTP+WS server (web feature)
├── generated/          Diesel schema (build.rs regenerates columns.rs)
├── frb_generated.rs    flutter_rust_bridge codegen output
└── bin/
    ├── p2p_chat.rs / p2p_chat_tui.rs / p2p_chat_server.rs
    └── tui/            the TUI binary: state, input, click, scroll, render loop
```

## Protocols and transport

The libp2p behaviour set (`src/behavior.rs`):

| Behaviour | Protocol | Carries |
|---|---|---|
| `gossipsub` | the chat topic | broadcast messages, group messages, group backfill requests |
| `request_response` | `/p2p-chat/dm/1.0.0` | direct messages |
| `device_binding` | `/p2p-chat/device-binding/1.0.0` | intro + signed claim exchange |
| `call_signaling` | `/p2p-chat/call/1.0.0` | call setup/accept/decline/end |
| `call_media` | `/p2p-chat/call-media/1.0.0` | μ-law voice frames + acks (cbor, US-044) |
| `typing` | `/p2p-chat/typing/1.0.0` | fire-and-forget DM typing start/stop (US-009) |
| `reactions` | `/p2p-chat/reactions/1.0.0` | per-message reaction add/remove sync (US-008) |
| `file_transfer` | `/p2p-chat/files/1.0.0` | attachment meta / chunk / error (cbor) |
| `shares` | `/p2p-chat/shares/1.0.0` | share offers, versioned manifests, revision-checked writes (JSON) |
| `mdns` | — | peer discovery on the LAN |
| `identify` | — | peer info + listen addresses (feeds Kademlia) |
| `kad` | — | internet-wide peer discovery over the DHT |
| `ping` | — | liveness |

Transport is TCP+QUIC with yamux multiplexing; libp2p `noise` provides
transport encryption. Direct messages additionally carry a payload-level
end-to-end envelope (US-042): `src/e2e.rs` seals the body for the recipient's
device in `send_dm`, and `handle_request_response` opens it before persistence.
Broadcasts and group messages stay plaintext (multi-recipient); a future relay
or store-and-forward holder sees only the sealed envelope for a DM. The
sender-side outbox (`dm_outbox.rs`) already parks sealed envelopes for offline
recipients and retries them on reconnect. See
[`e2e_encryption.md`](e2e_encryption.md) for the threat model, the outbox
lifecycle and the limits.

Gossipsub caps a single message at 256 KiB, so group and broadcast
attachments travel **by reference**: the message carries a descriptor, and the
bytes move over the separate `file_transfer` lane.

Versioned file shares reuse that blob store and file lane for their bytes, but
advertise, request and submit snapshots over the dedicated `shares` lane. ACLs
are checked at the swarm boundary against verified `user_id`s (never a claimed
payload id), public write is opt-in, and a submission carrying a stale
`base_revision` is rejected so the writer must refresh and retry. See
[`files_attachments.md`](files_attachments.md) for the share invariant set.

Typing indicators (US-009) also get their own `request_response` lane so a
keystroke burst never occupies a DM request slot. `TypingSignal` is
fire-and-forget, unpersisted and blocklist-gated; each frontend applies a short
TTL. See [`archive/2026-11-21_typing_indicators.md`](archive/2026-11-21_typing_indicators.md)
for the full slice.

Reactions (US-008) are persisted per `(msg_id, peer_id, emoji)` in the
`message_reactions` table — keying on the wire `msg_id` lets one table serve
broadcast, DM and group messages. A `request_response` lane syncs each change to
the peers holding that message (DM partner, all known peers, or group members);
the reactor applies its own change locally first, and inbound signals ignore the
claimed actor in favour of the connection peer id. `SwarmEvent::Reaction` carries
only the `msg_id`; each frontend fetches typed `ReactionSummary` values from Rust
(`get_reactions`), so neither parses reaction JSON itself. Token validation is
Rust-owned too: `reactions::validate_emoji` (non-empty, ≤32 chars after trim) is
re-applied on write and exposed to both frontends — the TUI reaction editor and
Flutter's "Custom…" token dialog (`validateEmoji` binding) call the same rule,
so neither frontend duplicates it. See
[`archive/2026-11-21_message_reactions.md`](archive/2026-11-21_message_reactions.md)
for the full slice.

Replies (US-006) carry a canonical `ReplyRef { msg_id, sender, preview }` in the
message payload rather than a foreign key. `src/reply.rs` owns the bounded
preview and the `↳ {sender}: {preview}` header used by all three frontends; the
snapshot survives local tombstones and is persisted in `reply_to_json` so
reloaded rows still render their quote. Incoming wire ids are stored in
`msg_id` so those rows can be replied to later.

### Notifications (US-027 / US-028)

`src/notifications.rs` owns the whole policy: it decides which incoming events
notify, builds the display text, and attaches a `Notification` payload
(`title`, bounded single-line `body`, a stable `conversationKey`, and `sound` /
`desktop` flags) to every broadcast, DM and group swarm event. Frontends are
thin delivery adapters: the TUI emits a bell / OSC 777 sequence after drawing,
Flutter and web ask `lib/src/util/notifications.dart` to deliver, and Android
posts through a MethodChannel. The one frontend rule — suppress when the app is
foregrounded **and** that conversation is on screen — lives with the adapter;
permission is requested only when the desktop channel is enabled. The two
channels persist in `app_settings` (`getNotificationSettings` /
`setNotificationSettings`, mirrored in web JSON). See
[`archive/2026-11-21_notifications.md`](archive/2026-11-21_notifications.md)
for the full slice.

### Network diagnostics

`src/net_diag.rs` is the single live-state store behind the Settings-tab
network details in **both** frontends. The swarm handler records every
transport/reachability event there (listen + external addresses, AutoNAT
verdict and probe outcomes, DCUtR hole-punch results, relay reservations and
circuits, Kademlia mode/routing size/bootstrap results, DHT announcements), and
both frontends render the same polled snapshot — the TUI reads it per frame,
Flutter through the `getNetworkDetails` FRB/JSON method (Settings polls it every
5 s while visible). OS interfaces (name, MAC, IPv4/IPv6) come from
`network-interface` and are cached for 5 s.

DHT presence is real, not inferred: the node publishes a
`/p2p-chat/peer/<peer_id>` record (value `{peerId, addresses, public}`) after
it has routing peers and reachable addresses, rate-limited to one attempt per
60 s; `PutRecord`'s result drives the "announced" status.

NAT'd nodes get relayed reachability. `P2P_RELAY` (comma-separated relay
multiaddrs, each carrying `/p2p/<relay-peer-id>`) makes the swarm handler
*listen* on each relayed address (`/…/p2p/<relay>/p2p-circuit`) — that is how
libp2p-relay 0.22 requests a reservation, and the client behaviour renews it
while the listener stays open. A closed listener frees its relay so a later
connection retries. Once AutoNAT reports `Private` and a reservation is held,
the DHT announcement (and the circuit listener address Identify advertises)
narrow to the dialable `/…/p2p/<relay>/p2p-circuit/p2p/<self>` form instead of
the unreachable LAN addresses; otherwise external/listen addresses stay first
with any completed circuit address appended.

### Network entry points (bootstrap + relays)

The two multiaddr lists that decide how the node reaches the internet are
persisted in the `network_peers` table (`kind` = `bootstrap` | `relay`, unique
on `(kind, multiaddr)`) and editable from Settings in all three frontends —
`src/network_config.rs` owns validation, listing, add and remove, and the FRB
(`listNetworkPeers` / `addNetworkPeer` / `removeNetworkPeer`) and web JSON
methods are 1:1 wrappers over it. Every frontend submits raw text; Rust parses
it back into a `Multiaddr` and returns the canonical string, so no unparseable
value is ever stored. Relay entries additionally require `/p2p/<peer-id>`,
because a reservation is requested from a specific relay.

Resolution precedence is deliberate and asymmetric:

- **Bootstrap**: `P2P_BOOTSTRAP` wins outright when *set* (even to an empty
  string, which is the documented way to disable bootstrapping and ignore the
  persisted list) → persisted `bootstrap` rows → the public IPFS bootstrap
  nodes, but only with `P2P_PUBLIC_DHT=1`. A default install stays LAN-only.
- **Relay**: `P2P_RELAY` and the persisted `relay` rows are **unioned**
  (`relay::effective_relay_addrs`), deduplicated. A relay is an explicit
  opt-in, so adding one in Settings must not silently drop an operator's
  env-configured one.

Both lists are read **only at node start** — the Settings surface (and every
saved-add toast) says "applies on next node start", and there is no live
reload. The TUI edits the list on the Settings tab (`b` add bootstrap, `r` add
relay through a shared prompt under `InputMode::AddingNetworkPeer`, `x` remove
the highlighted row, Up/Down/Home/End to move the highlight); Flutter uses a
"Network peers" card with an add dialog and a per-row delete.
`stored_*_addrs()` is best-effort: a database error yields an empty list rather
than failing startup.

Per-peer connection quality comes from the libp2p ping behaviour: every
`ping::Event` with an outbound result feeds `net_diag::record_ping_rtt` /
`record_ping_failure`, and `PeerQuality` (latest RTT, success/failure counts,
loss fraction) is read back with `peer_quality(peer_id)`. The sample is
**live-only**: `handle_connection_closed` clears it when the peer's last
connection goes away, so a stale RTT is never shown as current, and nothing is
persisted. Rust owns the display strings (`fmt::format_rtt` → `12 ms`,
`fmt::format_ping_loss` → `3% (1/32)`, `n/a` before the first ping) so the TUI
Peer Info tab and Flutter's `getPeerQuality` card cannot format numbers
differently.

### Peer blocklist / mute (US-054)

A blocked peer is dropped at the **swarm boundary**, not merely hidden in a
list. `blocked_peers` (primary key `peer_id`, canonical base58 text) is owned by
`src/blocklist.rs`; `swarm_handler` consults `is_blocked` on every inbound and
outbound path:

- gossipsub messages (broadcast, group chat **and** group backfill topics) are
  discarded before parsing, so nothing a muted peer sends is persisted;
- DM, device-binding, call-signaling, call-media, typing, reaction and
  file-transfer `request_response` messages are discarded; Identify results are
  not applied (no DHT/roster entry), mDNS discoveries are not dialed, and a
  connection from a blocked peer is disconnected immediately;
- outbound `SendDm` / `SendDeviceBinding` / `SendCallSignal` / `SendTyping` /
  reaction fan-out and manual dials whose multiaddr carries `/p2p/<blocked-id>`
  are refused.

Blocking also queues `SwarmCommand::DisconnectPeer`, so the connection that
existed *before* the block is closed instead of waiting for its next message.
The check is **fail-open**: an unreadable database returns "not blocked", so a
broken DB can never act as a global mute. Blocking is per peer id on purpose —
the account layer knows sibling devices, the blocklist deliberately does not, so
blocking one device does not mute the rest of a contact's devices.

The TUI toggles it on the Peer Info tab (`b`, with a `Blocked: yes/no` row);
Flutter exposes the same action on `PeerInfoScreen`'s app bar and renders the
same row. Both read/write through the FRB surface
(`isPeerBlocked` / `blockPeer` / `unblockPeer` / `listBlockedPeers`), which
`src/web/methods.rs` mirrors as JSON methods 1:1.

Muted peers stay in the roster — blocking hides nothing, it only drops traffic.
To make the state visible, the TUI Contacts table prefixes a fully muted contact
with `⊘` (or marks just the blocked device sub-rows when only some devices are
muted), and Flutter's `ContactList` renders the same marker from the
`blockedPeerIds` set. The marker is presentational only; neither frontend
filters the list by it.

## The identity model

Two layers, described in full in [identity.md](identity.md):

- **Account** — a stable user identity, keyed by the hash of an account
  public key. The account key is the only signer that can bind devices.
- **Device** — each install keeps its own libp2p keypair and `peer_id`. A
  device is trusted only when a signed claim binds it to a user's device set,
  and any peer verifies that claim before trusting it.

This is why the same user on two devices collapses into one **Contacts** row
with expandable device sub-rows, while a peer with no binding is a plain
single-device contact.

## Persistence

SQLite via Diesel. Nineteen tables:

| Table | Holds |
|---|---|
| `identities` | the per-install libp2p keypair, self nickname, account key |
| `peers` | one row per known peer (addresses, first/last seen) |
| `peer_name_history` | the nicknames a peer has used, for the name origin label |
| `peer_sessions` | one row per connect/disconnect (real network-size stats) |
| `messages` | broadcast and direct message bodies; `deleted_at` is a local-only tombstone |
| `dm_outbox` | sealed DM envelopes awaiting an offline recipient's ack (store-and-forward) |
| `message_receipts` | delivery acknowledgements |
| `message_reactions` | per-message reactions keyed on `(msg_id, peer_id, emoji)` |
| `broadcast_recipients` | per-peer record of which broadcasts a peer received |
| `groups` / `group_members` / `group_messages` | the group layer (`group_messages.deleted_at` is a local-only tombstone) |
| `users` / `device_bindings` / `contacts` | the identity + contact layer |
| `files` | attachment descriptors (the bytes live on disk, never in SQLite) |
| `shares` | owned/accepted shares, visibility, bound local root, current revision |
| `share_grants` | per-user read-only / read-write ACL on private shares |
| `share_entries` | current manifest entries; also the GC reference set for share blobs |
| `network_peers` | persisted bootstrap / relay multiaddrs (Settings, read at node start) |
| `blocked_peers` | muted peer ids; the swarm boundary drops their traffic |
| `app_settings` | key/value store behind notification channels and LAN-only mode |

Twenty-six migrations in `migrations/`. `db::ensure_columns()` also runs at
startup as a belt-and-braces check for older databases.

Multiple instances on one machine never clobber each other: the desktop app
auto-selects an unused `sqlite_*.db` in the working directory, and the Rust
API sets the path per thread with `db::set_db_url`.

## Feature flags

| Feature | Effect |
|---|---|
| `default` | `mdns`, `kad`, `relay`, `nat`, `tracing`, `quic`, `tui`, `attachment-gc`, `web` |
| `web` | the axum server; **implies `mobile`** |
| `mobile` | FRB surface, bundled sqlite; implies `basic`, `sqlite_bundled`, `quic`, `attachment-gc` |
| `tui` | ratatui + crossterm + ratatui-textarea |
| `basic` | libp2p only |
| `sqlite_bundled` | compile sqlite in (required for Android) |
| `attachment-gc` | sweep orphaned attachment blobs at startup; disabling it means the store never deletes anything |
| `mdns`, `tracing`, `quic` | LAN discovery, logging, QUIC transport |
| `kad` | Kademlia DHT + Identify (internet-wide discovery, DHT announcements) |
| `relay` | circuit-relay v2 server + client |
| `nat` | AutoNAT reachability detection + DCUtR hole punching |
| `calls` | **empty placeholder** reserved for future media dependencies (codecs/device adapters). Call signaling *and* μ-law media transport are un-gated and ship in the default build. |
| `test-utils` | test-only helpers |

Note that `default` includes `web`, which includes `mobile` — so the default
build compiles the FRB bridge and the whole axum server. That is deliberate:
one feature resolution for every desktop artifact, so the TUI and the web
frontend cannot drift apart.

## Build and run

```bash
scripts/build-all.sh            # TUI + web frontend (one command)
./target/release/p2p_chat_tui   # the terminal frontend
./target/release/p2p_chat_server --assets apps/flutter_app/build/web
# Both frontends on one node (the replacement for p2p_chat_combined):
./target/release/p2p_chat_tui --web --web-assets apps/flutter_app/build/web
```

`p2p_chat_tui --web` (optionally `--web-assets <dir>` / `P2P_WEB_ASSETS`) runs
the TUI and the web frontend against the same swarm. See
[web_frontend.md](web_frontend.md#building-and-running) for the three modes
and their feature sets.

The Flutter app's own toolchain lives in `apps/flutter_app`; see
[../README.md](../README.md) for the mobile and desktop paths, and
[web_frontend.md](web_frontend.md) for the server's flags and security model.
