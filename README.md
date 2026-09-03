# p2p_app

A multi-purpose P2P application. The first feature set is **chat**, built on
[libp2p](https://libp2p.io) with SQLite (via Diesel) for persistence. The
networking and persistence core is written in Rust and shared across two
frontends:

* **TUI** (`ratatui`) — the most complete frontend, runs in a terminal.
* **Headless CLI** — minimal interactive mode with the smallest footprint
  (reads stdin, prints to stderr).
* **Flutter** — cross-platform GUI that talks to Rust through a
  `flutter_rust_bridge` (FRB) FFI layer.

## Features

* **Broadcast chat** — publish messages to all peers over gossipsub.
* **Direct messages** — private 1:1 messaging over the request-response protocol.
* **Peer discovery** — mDNS for automatic local-network discovery.
* **Transports** — TCP, with optional QUIC.
* **Nicknames** — per-peer and self nicknames, with auto-generated petnames for
  silent peers. Validation lives in Rust (`crate::nickname::validate_nickname`).
* **Network-size tuning** — gossipsub mesh parameters adapt to peer count
  (Small / Medium / Large, see `crate::network::NetworkSize`).
* **Debug logging** — structured, scrollable logs with ANSI stripping.
* **Multi-instance safety** — picks the first unused `sqlite_*.db` via lock files;
  refuses to open a DB already locked by another instance.

## User Stories

* As a user I want to broadcast a message to everyone on my local network so we
  can hold a group conversation.
* As a user I want to open a private DM tab with a specific peer from the peers
  list and exchange messages only we can read.
* As a user I want peers to appear automatically (mDNS) without manual setup.
* As a user I want to set a nickname (validated by Rust) so others can recognise
  me, and set per-peer nicknames.
* As a user I want a scrollable debug/log tab to watch connection and swarm
  events while troubleshooting.
* As a user I want my identity, peers and message history persisted across
  restarts in SQLite, and two instances never clobber the same database.
* As a mobile user I want the app to keep running in the background (Android
  foreground service holding the mDNS multicast lock) so I stay reachable.

## Build & Run

```bash
cargo build                 # TUI is the default binary
cargo run                   # run the TUI

# Headless mode (no TUI; reads stdin, prints to stderr)
cargo run --no-default-features --bin p2p_chat

# Custom database location
#
# The desktop app auto-selects an unused `sqlite_*.db` in the working directory.
# On mobile / via the Rust API, point a thread at a specific database with
# `p2p_app::db::set_db_url(path)` (per-thread, so callers stay isolated).
```

### Flutter (mobile / desktop GUI)

The Flutter app lives in `apps/flutter_app`. It depends on the compiled Rust
library via `flutter_rust_bridge`:

```bash
cd apps/flutter_app
flutter pub get
flutter run        # needs the Rust .so on the target platform
```

The Rust↔Dart bindings are **generated** (`src/frb_generated.rs`,
`lib/src/rust/*`). After changing the FRB surface in `src/api/mod.rs`,
regenerate with the project's codegen config:

```bash
flutter_rust_bridge_codegen generate --config-file <path-to-frb-config>
```

> The repo's original bindings were produced with
> `rust_input=crate::api`. New functions added to `src/api/mod.rs`
> (`validate_nickname`, `network_size_label`) are the single Rust source of
> truth; they become callable from Dart only after regeneration.

## Architecture

* **`libp2p`** — networking (gossipsub, request-response, mDNS, TCP/QUIC).
* **Diesel + SQLite** — persistent storage of messages, peers, identities, receipts.
* **`ratatui`** — terminal UI (binaries under `src/bin/tui`).
* **`tokio`** — async runtime and swarm task orchestration.
* **`flutter_rust_bridge`** — FFI surface in `src/api/mod.rs` for the Flutter app.

### Rust layout

```
src/
├── lib.rs / api/mod.rs     # crate root + FRB entry surface
├── behavior.rs             # libp2p swarm behaviour construction
├── swarm_handler.rs        # translate libp2p events into app events
├── db.rs                   # SQLite connection, identity, multi-instance lock
├── messages.rs             # message persistence + retrieval
├── peers.rs / nickname.rs  # peer tracking, nicknames
├── network.rs              # adaptive NetworkSize classification
├── fmt.rs / types.rs       # formatting + shared event/command types
├── logging.rs              # logging + TUI log buffer
├── mobile_api.rs / mobile_node.rs  # FRB facade + node lifecycle
├── tui_*.rs / tui_tabs.rs  # TUI rendering, state, input, scroll, clicks
└── generated/              # Diesel schema/models (codegen from migrations)
src/bin/
├── p2p_chat.rs             # headless CLI
├── p2p_chat_tui.rs         # TUI entry point
└── tui/                    # TUI submodules (main_loop, handlers, render_loop)
```

### Android

`apps/flutter_app/android/...` holds the Kotlin glue: a `FlutterActivity`
(`MainActivity.kt`) bridging Dart↔Android via a `MethodChannel`, and a
`P2pForegroundService.kt` that holds the Wi-Fi multicast lock and posts the
persistent notification keeping the process alive. The service calls back into
Dart, which in turn starts the Rust node via FRB. This glue is platform-only and
cannot move into Rust.

## Testing

```bash
cargo t            # alias: cargo test --all-features
cargo ct           # alias: cargo clippy --all-targets --all-features
```

Tests are split into `unit/` (co-located lib unit tests) and `tests/`
(integration + TUI/swarm simulations). DB-backed tests use an isolated temp
database via `set_cached_db_url` and are serialised to stay idempotent.

## Done

* [x] Embedded Diesel migrations + runtime `ensure_columns` for legacy DBs
* [x] Ratatui TUI with Chat / Peers / Direct / Debug tabs
* [x] Direct messaging via request-response
* [x] mDNS peer discovery
* [x] Message / peer / identity / receipt persistence
* [x] Network-size-adaptive gossipsub config
* [x] Flutter mobile app via flutter_rust_bridge
* [x] Multi-instance DB selection + lock guarding
* [x] Per-peer nicknames + auto petnames for silent peers

## Todo

* [ ] Build for embedded Linux (OpenWRT, etc.)
* [ ] Contact list with stored usernames
* [ ] Expose `validate_nickname` / `network_size_label` to Dart and dedupe the
      Dart-side reimplementations (requires FRB codegen run)
* [ ] Improve UI/UX
