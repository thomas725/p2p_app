# p2p_app

A multi-purpose P2P application. The first feature set is **chat**, built on
[libp2p](https://libp2p.io) with SQLite (via Diesel) for persistence. The
networking and persistence core is written in Rust and shared across four
frontends:

* **TUI** (`ratatui`) — the most complete frontend, runs in a terminal.
* **Flutter mobile** (Android) — GUI over a `flutter_rust_bridge` (FRB) FFI
  layer; a foreground service keeps the node reachable in the background.
* **Flutter desktop** (Linux) — the same Flutter app, built for the desktop.
* **Flutter Web** — a *display client* for a Rust node running in-process in
  the `p2p_chat_server` binary. Nothing is compiled to WASM.

There is also a **headless CLI** (`p2p_chat`) — a minimal interactive mode with
the smallest footprint (reads stdin, prints to stderr).

## Features

* **Broadcast chat** — publish messages to all peers over gossipsub.
* **Direct messages** — private 1:1 messaging over the request-response protocol.
* **Group chats** — gossipsub topics with membership, history, and a
  store-and-forward backfill lane.
* **File attachments** — content-addressed (SHA-256) transfer over a separate
  lane, so gossipsub's 256 KiB message cap is never hit.
* **Call signaling** — ring / accept / decline / hang up. Media transport is
  not implemented yet.
* **Identity and device binding** — an account key per user, signed claims
  binding devices, and a Contacts roster that collapses one user's devices
  into a single row.
* **Peer discovery** — mDNS on the LAN plus Kademlia DHT bootstrap for
  internet-wide discovery.
* **NAT traversal** — circuit-relay v2 reservations, AutoNAT reachability
  detection, and DCUtR hole punching.
* **Network diagnostics** — live OS interfaces, listen/external addresses,
  reachability, relay reservations, and DHT state on Settings in both
  frontends.
* **Transports** — TCP, with optional QUIC.
* **Nicknames** — per-peer and self nicknames, with auto-generated petnames for
  silent peers. Validation lives in Rust (`crate::nickname::validate_nickname`).
* **Network-size tuning** — gossipsub mesh parameters adapt to peer count
  (Small / Medium / Large, see `crate::network::NetworkSize`).
* **Debug logging** — structured, scrollable logs with ANSI stripping.
* **Multi-instance safety** — picks the first unused `sqlite_*.db` via lock files;
  refuses to open a DB already locked by another instance.

## Example user stories

* As a user I want to broadcast a message to everyone on my local network so we
  can hold a group conversation.
* As a user I want to open a private DM tab with a specific peer from the
  contacts list and exchange messages only we can read.
* As a user I want peers to appear automatically (mDNS) without manual setup.
* As a user I want to set a nickname (validated by Rust) so others can recognise
  me, and set per-peer nicknames.
* As a user I want a scrollable debug/log tab to watch connection and swarm
  events while troubleshooting.
* As a user I want my identity, peers and message history persisted across
  restarts in SQLite, and two instances never clobber the same database.
* As a user I want to send a file to a peer and see it in the conversation.
* As a mobile user I want the app to keep running in the background (Android
  foreground service holding the mDNS multicast lock) so I stay reachable.

This is a sample, not the catalogue: the open stories and their acceptance
criteria live in [`docs/backlog.md`](docs/backlog.md), and the full original set
(including the shipped ones) is in
[`docs/archive/2026-04-17_user_stories.md`](docs/archive/2026-04-17_user_stories.md).

## Build & run

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

`flake.nix` provides Flutter (plus the Android SDK and JDK) via `nix develop`.
Where nix is not an option, the official SDK tarball needs no root and is enough
for `flutter analyze`, `flutter test` and `flutter build web` (Linux *desktop*
builds additionally need `clang`, `cmake`, `ninja-build`, `gtk3-devel` and
`pkgconf-pkg-config` from the distro):

```bash
curl -fLO https://storage.googleapis.com/flutter_infra_release/releases/stable/linux/flutter_linux_3.47.9-stable.tar.xz
tar xf flutter_linux_3.47.9-stable.tar.xz -C ~ && rm flutter_linux_3.47.9-stable.tar.xz
export PATH="$HOME/flutter/bin:$PATH"     # add to ~/.bashrc to persist
flutter config --no-analytics && flutter precache --web
```

### Web frontend (Flutter Web, Rust node server)

The browser app is a **display client**: the node stays native inside the Rust
web server (`p2p_chat_server`) and the browser talks JSON-over-WebSocket to it.
Nothing is compiled to WASM. Build both desktop frontends in one go:

```bash
scripts/build-all.sh              # TUI + web frontend (add --debug for a fast build)
./target/release/p2p_chat_tui     # terminal frontend
./target/release/p2p_chat_server --assets apps/flutter_app/build/web
```

The server binds `127.0.0.1` by default (no credentials — the TUI trust model).
For LAN exposure pass `--bind 0.0.0.0` together with HTTP Basic credentials
(`P2P_WEB_PASSWORD`, or `--password`), which the browser prompts for and then
remembers; the gate covers the API, the WebSocket and the static SPA. See
[`docs/web_frontend.md`](docs/web_frontend.md).

The Rust↔Dart bindings are **generated** (`src/frb_generated.rs`,
`lib/src/rust/*`) from the checked-in `flutter_rust_bridge.yaml` (input:
`crate::api`, `crate::mobile_api`, `crate::mobile_node`). After changing an
FRB-exposed function, regenerate:

```bash
flutter_rust_bridge_codegen generate   # reads flutter_rust_bridge.yaml
```

Then re-append the clippy allow block to `src/frb_generated.rs`, run
`cargo fmt --all`, and delete the untracked `src/frb_generated.h`. The full
checklist is in [`AGENTS.md`](AGENTS.md) ("FRB regeneration protocol").

## Architecture

* **`libp2p`** — networking (gossipsub, four request-response protocols, mDNS,
  Kademlia, relay/AutoNAT/DCUtR, TCP/QUIC, ping).
* **Diesel + SQLite** — persistent storage of messages, peers, identities,
  receipts, groups, and contacts.
* **`ratatui`** — terminal UI (binaries under `src/bin/tui`).
* **`tokio`** — async runtime and swarm task orchestration.
* **`flutter_rust_bridge`** — FFI surface in `src/api/mod.rs` for the Flutter app.
* **`axum`** — the web frontend's HTTP + WebSocket server (`src/web/`).

### Rust layout

```
src/
├── lib.rs                     # crate root; module wiring
├── behavior.rs                # libp2p behaviours: gossipsub, 4 request_response lanes, mdns, ping
├── swarm_handler.rs           # libp2p events -> app events; inbound persistence
├── db.rs                      # SQLite connection, multi-instance lock, ensure_columns
├── messages.rs                # message + receipt persistence, per-peer stats
├── peers.rs / nickname.rs     # peer rows, sessions, display-name resolution
├── groups.rs                  # groups, members, group messages, backfill watermark
├── identity.rs / contacts.rs  # users, account keys, device bindings, contact roster
├── calls.rs                   # call session model (signaling only)
├── files.rs                   # content-addressed attachment store
├── network.rs                 # adaptive NetworkSize classification
├── net_diag.rs                # live diagnostics (interfaces, NAT, relay, DHT)
├── fmt.rs / types.rs          # formatting + shared event/command types
├── logging.rs                 # logging + TUI log buffer
├── api/mod.rs                 # FRB entry surface (mobile + web)
├── mobile_api.rs              # FRB-exposed helpers (sorting, formatting, stats)
├── mobile_node.rs             # node lifecycle for the mobile/web server path
├── tui_tabs.rs / tui_helpers.rs / tui_render_state.rs
├── tui_render/                # ratatui renderers (peers, chat, groups, log, ...)
├── web/                       # axum HTTP+WS server (feature `web`)
├── generated/                 # Diesel schema/models (codegen from migrations)
└── bin/
    ├── p2p_chat.rs             # headless CLI
    ├── p2p_chat_tui.rs         # TUI entry point
    ├── p2p_chat_server.rs      # node + web server
    └── tui/                    # TUI submodules (main_loop, handlers, render_loop)
```

Full details: [`docs/architecture.md`](docs/architecture.md).

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

cd apps/flutter_app && flutter test
```

`tests/unit/` holds the unit-level suites (library modules plus the `src/bin`
TUI submodules); the top-level `tests/*.rs` files hold the integration and
swarm/TUI simulations. DB-backed tests point each test at an isolated temp
database via `p2p_app::db::set_db_url` and take `db::shared_db_test_lock()` so
they stay serialised and idempotent.

## Documentation

[`docs/README.md`](docs/README.md) is the index. The live documents are:

| Doc | What it is |
|---|---|
| [`docs/architecture.md`](docs/architecture.md) | How the system fits together |
| [`docs/backlog.md`](docs/backlog.md) | **Live, open work** — what is next |
| [`docs/dependencies.md`](docs/dependencies.md) | Direct dependencies and why each is here |
| [`docs/ci_pipeline.md`](docs/ci_pipeline.md) | The GitHub Actions workflows |
| [`docs/identity.md`](docs/identity.md) | Users, devices, signed claims, the Contacts roster |
| [`docs/files_attachments.md`](docs/files_attachments.md) | The attachment lane |
| [`docs/web_frontend.md`](docs/web_frontend.md) | The Flutter Web display client |
| [`docs/android_sqlite.md`](docs/android_sqlite.md) | Why Android embeds its own SQLite |
| [`KEYBOARD_SHORTCUTS.md`](KEYBOARD_SHORTCUTS.md) | Every TUI key and mouse binding |
| [`AGENTS.md`](AGENTS.md) | Operating rules for agents working in this repo |

Completed work and superseded designs live in [`docs/archive/`](docs/archive/).

## Roadmap

Open work is tracked in [`docs/backlog.md`](docs/backlog.md) — priorities,
blockers, and the known limitations that are deliberate rather than unfinished.
