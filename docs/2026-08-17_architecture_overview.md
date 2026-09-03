# Architecture Overview

## Project Summary

`p2p_app` is a peer-to-peer chat application built primarily in Rust (edition 2024). It uses libp2p for networking, SQLite (via Diesel ORM) for persistence, and currently has three frontend targets: CLI, TUI, and a Flutter mobile/desktop app (via flutter_rust_bridge). The earlier Dioxus desktop frontend was removed in favor of Flutter.

**Status:** Functional TUI and CLI apps. Flutter mobile app talks to Rust via FRB. Android is a major target. No Dioxus code remains.

---

## Crate Structure

Single crate (not a workspace). Two binary targets (plus the Flutter app in `apps/flutter_app`):

| Binary | Entry | Feature Gate | Description |
|--------|-------|-------------|-------------|
| `p2p_chat` | `src/bin/p2p_chat.rs` | none | Headless CLI, stdin/stderr |
| `p2p_chat_tui` (default) | `src/bin/p2p_chat_tui.rs` | `tui` | Full ratatui UI |

Library root is `src/lib.rs` which exports all shared modules.

---

## Core Library Modules

```
src/
├── lib.rs                 # Module declarations, re-exports, MIGRATIONS const
├── behavior.rs            # NetworkBehaviour: gossipsub + request-response + mDNS
├── swarm_handler.rs       # Translates libp2p events → SwarmEvent via MPSC
├── db.rs                  # SQLite connection, identity, lock files, migrations, ensure_columns()
├── messages.rs            # Message CRUD (save, load, mark sent, receipts)
├── peers.rs               # Peer CRUD, session tracking, port persistence
├── nickname.rs            # Nickname management (self, peer local, received, per-peer)
├── network.rs             # NetworkSize enum for adaptive gossipsub config
├── types.rs               # SwarmEvent, SwarmCommand, MessageEvent, DisplayMessage, PeerRecord
├── fmt.rs                 # Formatting utilities (timestamps, latency, peer IDs)
├── logging.rs             # Tracing-based logging with TUI callback, in-memory log buffer
├── tui_tabs.rs            # Tab management (DynamicTabs, DmTab, TabContent)
├── tui_render.rs          # ratatui rendering functions
├── tui_render_state.rs    # TuiRenderState abstraction (testable rendering)
├── tui_helpers.rs         # Pure helper functions (scroll, nicknames, validation)
├── connected.rs           # ConnectedPeerTracker shared with the Flutter backend
├── mobile_api.rs          # FRB facade (mobile feature)
└── generated/             # Auto-generated Diesel code (schema, models, columns)
```

---

## P2P Networking Layer

### Transport Stack

```
TCP + QUIC (optional)
  └── Noise (encryption)
       └── Yamux (multiplexing)
```

### Protocols

| Protocol | Purpose |
|----------|---------|
| Gossipsub | Broadcast messages to topic "test-net" |
| Request-Response (JSON) | Direct messages + receipt confirmations |
| mDNS | Automatic LAN peer discovery |

### Message Types

- **BroadcastMessage**: `{ content, sent_at, nickname, msg_id }` - gossipsub
- **DirectMessage**: `{ content, timestamp, sent_at, nickname, msg_id, ack_for, received_at }` - request-response

### Adaptive Network Size

`NetworkSize` enum adapts gossipsub parameters based on historical peer count:

| Peers | Heartbeat | History Gossip | History Length |
|-------|-----------|----------------|----------------|
| 0-3 (Small) | 1s | 3 | 20 |
| 4-15 (Medium) | 2s | 6 | 30 |
| 16+ (Large) | 5s | 12 | 50 |

### Receipt System

- Broadcasts trigger a receipt DM back to propagation source
- DMs trigger an ACK DM
- Stored in `message_receipts` table (kind 0=broadcast, 1=DM)

---

## TUI Architecture (4-Task Model)

```
SwarmHandler ──SwarmEvent──> CommandProcessor <──InputEvent── EventSource
                                  │   ↑                             ↑
                                  │   │                        Terminal Input
                                  ↓   │
                              RenderLoop
                                  │
                                  ↓
                             Terminal Output
```

- **SwarmHandler**: Translates libp2p events to `SwarmEvent` via MPSC
- **EventSource**: Polls crossterm at 60 FPS for keyboard/mouse
- **CommandProcessor**: Single writer to `AppState` behind `Arc<Mutex<>>`
- **RenderLoop**: Event-driven rendering, only redraws on `RenderEvent`

State lives in `AppState` (messages, peers, DMs, receipts, scroll states, nicknames).

---

## Database Schema (SQLite via Diesel)

| Table | Purpose |
|-------|---------|
| `identities` | libp2p keypair storage (Ed25519), ports, self_nickname |
| `messages` | All messages (broadcast + DM), content, peer_id, topic, msg_id |
| `peers` | Known peers, addresses, first/last seen, nicknames |
| `peer_sessions` | Concurrent peer count history |
| `message_receipts` | Delivery/read confirmations |

### Multi-Instance Support

The database path is resolved per-thread (no process-global or env-var state),
so each test/worker opens its own database:

 1. If a thread set a path via `db::set_db_url`, that path is used.
 2. Otherwise, scans CWD for `sqlite_*.db` files
 3. Checks `.db.lock` files (PID-based, Linux /proc check)
 4. Uses first unlocked database
 5. Creates new sequential database if all locked

### Migration Strategy

- Fresh installs: columns in `CREATE TABLE` (via Diesel migrations)
- Existing DBs: `ensure_columns()` in `db.rs` uses auto-generated `SCHEMA_ENTRIES` (from `build.rs` parsing `schema.rs`) to add missing columns
- No `ALTER TABLE` migrations needed

---

## Build System

### Key Files

| File | Purpose |
|------|---------|
| `build.rs` | Generates `SCHEMA_ENTRIES` constant from `schema.rs` for `ensure_columns()` |
| `build_release.sh` | Cross-compilation + UPX compression (achieves ~34% size reduction) |
| `diesel_generate.sh` | Diesel code generation pipeline (8 steps) |
| `.cargo/config.toml` | Aliases: `cargo t` (test + test-utils), `cargo c` (clippy) |

### Cross-Compilation Targets

Already supports: x86_64, aarch64, armv7, mipsel (with `sqlite_bundled` for embedded).

### Release Optimization

`Cargo.toml` release profile: `strip=true`, `opt-level='z'`, `lto='fat'`, `codegen-units=1`, `panic='abort'`. Binary sizes around 1.6-4.7 MB depending on features.

---

## Feature Flags

| Feature | Purpose |
|---------|---------|
| `mdns` | mDNS local discovery (default) |
| `tracing` | Structured logging (default) |
| `quic` | QUIC transport (default) |
| `tui` | ratatui terminal UI (default) |
| `mobile` | Flutter/Rust bridge (FRB), not in default |
| `basic` | Minimal P2P, no optional features |
| `sqlite_bundled` | Static SQLite linking |
| `test-utils` | Test helpers (gated) |

---

## Testing

- Unit tests in `tests/unit/` (included via `#[path]` in module `#[cfg(test)]` blocks)
- Integration tests in `tests/` (standard cargo test files)
- 26 unit test files, integration tests for P2P, DB, TUI rendering
- Test pattern: `with_test_db()` for isolated temp databases with serialized access

## Codebase Size

37 Rust source files, ~6,800 LOC total. Average 184 lines/file.
