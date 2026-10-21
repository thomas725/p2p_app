# Architecture overview

How the system fits together. For *what to build next* see
[backlog.md](backlog.md); for *rules* see [../AGENTS.md](../AGENTS.md).

## What it is

A LAN-first, serverless P2P chat application. Every participant is both a
client and a node: there is no central server, and the only way peers find
each other is mDNS on the local network. Persistence is a local SQLite file
(Diesel). The app is a thin shell over a Rust core that all frontends share.

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
├── types.rs            shared constants and enums (SwarmEvent, SwarmCommand, InputMode)
├── db.rs               connection handling, multi-instance DB resolution, ensure_columns
├── messages.rs         message + receipt persistence, per-peer stats
├── peers.rs            peer rows, sessions, broadcast recipients
├── nickname.rs         display-name resolution + memoization
├── fmt.rs              canonical formatting (parse_last_seen_ms, peer_id_suffix, ...)
├── groups.rs           groups, members, group messages, backfill watermark
├── identity.rs         users, account keys, device bindings, signed claims
├── contacts.rs         the contact roster
├── calls.rs            call session model (signaling only; media is future work)
├── files.rs            content-addressed attachment store
├── connected.rs        ConnectedTracker
├── logging.rs          log helpers
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

Six libp2p behaviours (`src/behavior.rs`):

| Behaviour | Protocol | Carries |
|---|---|---|
| `gossipsub` | the chat topic | broadcast messages, group messages, group backfill requests |
| `request_response` | `/p2p-chat/dm/1.0.0` | direct messages |
| `device_binding` | `/p2p-chat/device-binding/1.0.0` | intro + signed claim exchange |
| `call_signaling` | `/p2p-chat/call/1.0.0` | call setup/accept/decline/end |
| `file_transfer` | `/p2p-chat/files/1.0.0` | attachment meta / chunk / error (cbor) |
| `mdns` | — | peer discovery on the LAN |
| `ping` | — | liveness |

Transport is TCP+QUIC with yamux multiplexing; libp2p `noise` provides
transport encryption. **There is no end-to-end encryption layer** — the
receiving node sees plaintext message bodies (see [backlog.md](backlog.md),
US-042).

Gossipsub caps a single message at 256 KiB, so group and broadcast
attachments travel **by reference**: the message carries a descriptor, and the
bytes move over the separate `file_transfer` lane.

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

SQLite via Diesel. Fourteen tables:

| Table | Holds |
|---|---|
| `identities` | the per-install libp2p keypair, self nickname, account key |
| `peers` | one row per known peer (addresses, first/last seen) |
| `peer_name_history` | the nicknames a peer has used, for the name origin label |
| `peer_sessions` | one row per connect/disconnect (real network-size stats) |
| `messages` | broadcast, direct, and group message bodies |
| `message_receipts` | delivery acknowledgements |
| `broadcast_recipients` | per-peer record of which broadcasts a peer received |
| `groups` / `group_members` / `group_messages` | the group layer |
| `users` / `device_bindings` / `contacts` | the identity + contact layer |
| `files` | attachment descriptors (the bytes live on disk, never in SQLite) |

Sixteen migrations in `migrations/`. `db::ensure_columns()` also runs at
startup as a belt-and-braces check for older databases.

Multiple instances on one machine never clobber each other: the desktop app
auto-selects an unused `sqlite_*.db` in the working directory, and the Rust
API sets the path per thread with `db::set_db_url`.

## Feature flags

| Feature | Effect |
|---|---|
| `default` | `mdns`, `tracing`, `quic`, `tui`, `attachment-gc`, `web` |
| `web` | the axum server; **implies `mobile`** |
| `mobile` | FRB surface, bundled sqlite; implies `basic`, `sqlite_bundled`, `quic`, `attachment-gc` |
| `tui` | ratatui + crossterm + ratatui-textarea |
| `basic` | libp2p only |
| `sqlite_bundled` | compile sqlite in (required for Android) |
| `attachment-gc` | sweep orphaned attachment blobs at startup; disabling it means the store never deletes anything |
| `mdns`, `tracing`, `quic` | discovery, logging, QUIC transport |
| `calls` | **empty placeholder** reserved for future call-media dependencies. Call *signaling* is un-gated and ships in the default build. |
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
```

The Flutter app's own toolchain lives in `apps/flutter_app`; see
[../README.md](../README.md) for the mobile and desktop paths, and
[web_frontend.md](web_frontend.md) for the server's flags and security model.
