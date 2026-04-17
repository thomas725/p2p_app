# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**p2p_app** is a decentralized P2P chat application built with Rust. It provides both a TUI (terminal UI) and headless CLI for communication across networks using libp2p's gossipsub (broadcast) and request-response (direct messaging) protocols. All messages and peer metadata are persisted in SQLite using Diesel ORM.

**Key Technologies:**
- **libp2p**: P2P networking (gossipsub, request-response, mDNS, TCP/QUIC transports)
- **Diesel**: SQLite ORM with embedded migrations
- **Ratatui**: Terminal UI framework with multiple tabs
- **tokio**: Async runtime for concurrent operations
- **tracing**: Structured logging with feature-gated denylist filter

## Architecture Overview

### High-Level Design

The application follows a modular architecture with clear separation of concerns:

```
┌─────────────────────────────────────────┐
│         Binary Entrypoints              │
│  ┌──────────────────┬──────────────────┐│
│  │ p2p_chat (CLI)   │ p2p_chat_tui     ││
│  │ (headless)       │ (interactive)    ││
│  └──────────────────┴──────────────────┘│
└──────────────┬──────────────────────────┘
               │
        ┌──────▼───────┐
        │ LibP2P Swarm │ (networking core)
        │  AppBehaviour│
        └──────┬───────┘
          ┌────┴──────┬──────────┐
          │            │          │
    ┌─────▼────┐ ┌────▼────┐ ┌──▼─────┐
    │ Gossipsub │ │ Request │ │ mDNS   │
    │ (Publish) │ │Response │ │(Discovery)
    │           │ │(DM)     │ │        │
    └───────────┘ └─────────┘ └────────┘
          │
    ┌─────▼────────────────────┐
    │   Diesel Database        │
    │  (SQLite persistence)    │
    └──────────────────────────┘
```

### Module Organization

- **`lib.rs`**: Library root containing:
  - `AppBehaviour` - Custom NetworkBehaviour for libp2p (combines gossipsub, request-response, mDNS)
  - `DirectMessage` - Codec for direct message protocol
  - Tracing filter configuration and logging utilities
  - Timestamp formatting and ANSI code stripping
  - Database initialization functions

- **`models_queryable.rs` & `models_insertable.rs`**: Auto-generated Diesel model structs (do not edit manually). Queryable for reading, Insertable for writing to the database.

- **`schema.rs`**: Auto-generated Diesel schema from migrations. Regenerate with `./diesel_generate.sh` after creating new migrations.

- **`bin/p2p_chat_tui.rs`**: TUI binary - implements interactive terminal interface with tabs, mouse support, and real-time message display.

- **`bin/p2p_chat.rs`**: CLI/headless binary - simpler entry point for non-interactive use.

### Feature Flags & Runtime Behavior

Feature flags control which components are compiled in. Defaults: `mdns`, `tracing`, `quic`, `tui`, `gabble`.

**When to use which configuration:**
- **TUI mode** (default): `cargo run` - Full interactive experience
- **Headless with mDNS**: `cargo run --no-default-features --features mdns,tracing` - Peer discovery without UI
- **CLI only**: `cargo run --no-default-features --features tracing` - No discovery or TUI
- **Minimal/embedded**: `cargo run --no-default-features --features basic` - Bare libp2p

## Common Development Commands

### Build & Run

```bash
# Build debug binary
cargo build

# Build release (size-optimized)
./build_release.sh

# Run with TUI (default)
cargo run

# Run with custom database
DATABASE_URL=my.db cargo run

# Run headless (no TUI, with peer discovery)
cargo run --no-default-features --features mdns,tracing

# Run CLI only (no mDNS)
cargo run --no-default-features --features tracing

# Run specific binary
cargo run --bin p2p_chat
cargo run --bin p2p_chat_tui
```

### Linting & Formatting

```bash
# Format code
cargo fmt

# Run Clippy checks
cargo clippy

# Apply Clippy fixes
cargo clippy --fix

# Strict Clippy (as in CI)
cargo clippy -- -D warnings
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run single test by name
cargo test test_name

# Run integration tests only
cargo test --test p2p_integration

# Run TUI tests only
cargo test --test tui_chat

# Run a specific TUI test
cargo test --test tui_chat test_name
```

## Database & Migrations

### Workflow

1. **Create migration**: `diesel migration generate -n description`
2. **Write SQL** in `migrations/<TIMESTAMP>_description/up.sql` and `down.sql`
3. **Apply and generate models**: `./diesel_generate.sh`
4. **Update model structs** in `models_queryable.rs` and `models_insertable.rs` if needed

### Migration Naming Convention

`YYYY-MM-DD-HHMMSS_description` where the 6 digits are hours, minutes, seconds (e.g., `2026-04-06-120000_add_ports`).

### Important Notes

- **Schema regeneration**: Always run `./diesel_generate.sh` after modifying migrations. This updates `schema.rs` and models.
- **Schema.rs**: Never edit manually; always regenerate.
- **Embedded migrations**: Migrations are embedded in the binary and run automatically at startup via `diesel_migrations::run_pending_migrations()`.
- **Database location**: Defaults to `sqlite.db` or set via `DATABASE_URL` env var.

## Key Architectural Patterns

### LibP2P Behaviors

The `AppBehaviour` struct implements `NetworkBehaviour` and orchestrates three sub-behaviors:
- **Gossipsub**: Broadcast messaging to subscribed topic (`"test-net"`)
- **Request-Response**: Direct messaging between peers
- **mDNS**: Automatic peer discovery on local network

Events flow through `SwarmEvent` -> matched on specific event types -> application logic.

### Direct Messaging Protocol

- **Protocol name**: `/p2p-chat/dm/1.0.0`
- **Codec**: JSON over libp2p's request-response
- **Structure**: `DirectMessage { content: String, timestamp: String }`
- **Encryption**: Automatic via Noise protocol (libp2p default)

### Logging & Tracing

The `tracing_filter()` function implements a denylist approach:
- **OFF**: `multistream_select`, `yamux::connection`, `libp2p_core::transport::choice`, `libp2p_mdns::behaviour::iface` (noise)
- **DEBUG**: `libp2p_swarm`, `libp2p_tcp`, `libp2p_quic::transport`, `libp2p_mdns::behaviour`
- **INFO**: `libp2p_gossipsub::behaviour`
- **WARN**: Everything else (default)

Use `push_log()` to add timestamped messages to the TUI debug tab and stderr.

### Async & Concurrency

- **Runtime**: Single tokio multi-threaded runtime (`tokio::main`)
- **Selection**: Use `tokio::select!` for concurrent operations (network events vs stdin)
- **Blocking tasks**: Use `tokio::task::spawn_blocking` for CPU-intensive work
- **Sync primitives**: Prefer `tokio::sync` (Mutex, RwLock) over `std::sync` in async contexts

### Database Patterns

- **Connection pool**: Diesel manages SQLite connections internally
- **Transactions**: Use `connection.transaction()` for multi-statement consistency
- **Error handling**: Return `Result<T, diesel::result::Error>` and wrap in `color_eyre::Report`
- **Insertable pattern**: Create `NewModel` struct, insert, then query back if needed
- **Persistence**: Messages include `is_direct` flag; peers track `last_seen` timestamp

## Code Style

See AGENTS.md for detailed style guidelines. Key points:
- Trait imports used for methods: `use crate::AsyncBufReadExt as _`
- Explicit return types on public functions
- `Result<T, color_eyre::Report>` for error handling
- Avoid `unwrap()` in production code
- `snake_case` for functions/variables, `PascalCase` for types

## TUI Architecture

### Tab System

The TUI maintains multiple tabs accessible via Tab key:
1. **Chat**: Broadcast messages (write-enabled)
2. **Peers**: Discovered peers list (read-only, click to open DM)
3. **Direct**: Dynamic tabs for each active peer conversation (write-enabled)
4. **Debug**: System logs and events (read-only, scrollable)

### State Management

`App` struct holds:
- `active_tab`: Current tab index
- `messages`: VecDeque of formatted chat messages
- `direct_message_tabs`: HashMap of peer_id -> conversation state
- `peers`: List of known peers with metadata
- `chat_list_state`: Ratatui list widget state (scroll position)

Mouse click detection maps row to peer ID by calculating content offset and list index.

### Mouse Support

- Click tabs to switch
- Click peer name in Peers tab to open DM
- Arrow keys to navigate in Peers list
- Enter to open DM from selected peer

## Testing

### Test Coverage Summary

**Total: 28 tests (4 unit + 4 integration + 20 TUI), 100% passing**

### Unit Tests (4 tests in `src/lib.rs`)

Core utilities and formatting:
1. `test_format_peer_datetime()` - Timezone-aware peer timestamp formatting
2. `test_strip_ansi_codes()` - ANSI escape sequence removal for log display
3. `test_now_timestamp_format()` - Current timestamp generation in UTC
4. `test_network_size_from_peer_count()` - Adaptive network sizing (Small/Medium/Large)

### Integration Tests (7 tests in `tests/p2p_integration.rs`, 4 active + 3 ignored)

Network behavior and message passing:
1. **`test_auto_discovery_via_mdns()`** - Peers automatically discover each other via mDNS within ~2 sec
2. **`test_p2p_message_transfer()`** - Messages published to gossipsub topic propagate to subscribed peers
3. **`test_bidirectional_messages()`** - Messages flow correctly in both directions between nodes
4. **`test_connection_with_stale_db_address_and_mdns_recovery()`** - Nodes recover and reconnect when DB has stale peer addresses

Ignored (marked for manual testing):
- `test_direct_message_protocol()` - Direct message (request-response) protocol validation
- `test_peer_persistence()` - Peer list survives binary restart (requires DB setup)
- `test_message_deduplication()` - Gossipsub message deduplication via DefaultHasher

### TUI Tests (20 tests in `tests/tui_chat.rs`)

**Mouse & Input Handling (3 tests):**
- `test_handle_mouse_click()` - Valid row coordinates map to correct peer
- `test_handle_mouse_click_outside_bounds()` - Out-of-bounds clicks return None
- `test_calculate_content_start_row()` - Content layout accounts for notification area

**Tab Management (4 tests):**
- `test_tab_id_index()` - TabId enum maps to correct numeric index (0-3)
- `test_tab_id_from_index()` - TabId created from numeric index
- `test_tab_id_default()` - Default TabId is Chat (0)
- `test_tab_id_partial_eq()` - TabId equality works correctly

**Tab Switching (4 tests):**
- `test_tab_click_to_chat()` - Tab row 0 switches to Chat
- `test_tab_click_to_peers()` - Tab row 1 switches to Peers
- `test_tab_click_to_log()` - Tab row 3 switches to Debug log
- Custom message handling tests (comprehensive message flow)

**Notification & Unread Tracking (4 tests):**
- `test_unread_notification_increments()` - Unread count increments on new broadcast
- `test_notification_clears_on_chat_tab_focus()` - Unread clears when Chat tab active
- `test_notification_area_calculation()` - Notification area expands/contracts based on count

**Data Structure Tests (3 tests):**
- `test_dm_tab_new()` - Create empty DM tab for peer
- `test_dm_tab_with_messages()` - DM tab initialized with message history
- `test_dm_tab_message_persistence()` - Messages survive DM tab cloning
- `test_dm_tab_cloning()` - DmTab Clone implementation works

**TUI Test Pattern**

Use `TuiTestState` to simulate UI state without running full application:

```rust
use p2p_app::tui::{DmTab, TuiTestState};

let mut state = TuiTestState::new();
state.active_tab = 0; // Switch to Chat
let peer = state.handle_mouse_click(5);
let start_row = state.calculate_content_start_row();
```

Key state components: `messages`, `chat_message_peers`, `active_tab`, `unread_broadcasts`.

## Code Quality & Robustness

### Quality Metrics

- **Test Coverage**: 28 automated tests across unit, integration, and TUI categories
- **Compilation**: Zero warnings with `cargo clippy -- -D warnings`
- **Error Handling**: Comprehensive `Result<T, color_eyre::Report>` throughout; zero panics in library code
- **Code Safety**: All `unwrap()` calls replaced with explicit `.expect()` messages documenting assumptions
- **Type Safety**: Explicit `.expect()` on all Mutex locks, durations, and system time operations
- **Documentation**: Public APIs documented with examples and edge cases

### Error Handling Philosophy

- **Library code** (`src/lib.rs`): No `unwrap()` - all errors propagated via `Result`
- **TUI code** (`src/bin/p2p_chat_tui.rs`): 6 `expect()` calls all with defensive guards checking preconditions
- **Tests**: Comprehensive timeout handling prevents hung tests (15-60 second limits)

## Performance & Optimization

- **Release build**: Enabled with `./build_release.sh`, applies LTO, size optimization, symbol stripping
- **Network sizing**: Gossipsub config adapts to historical peer count (Small/Medium/Large)
  - Small (1-3 peers): Aggressive heartbeat, max 10 mesh peers
  - Medium (4-15 peers): Balanced parameters
  - Large (16+ peers): Conservative settings to reduce CPU/bandwidth
- **Logging overhead**: Denylist filter prevents spam from internal libp2p modules (max 2000 log entries)
- **Database indices**: Migration strategy includes indexed columns for O(log n) peer lookups

## Debugging Tips

1. **Network events**: Increase tracing level for `libp2p_swarm` or `libp2p_gossipsub::behaviour`
2. **Message flow**: Enable tracing for `request_response` and check direct message protocol
3. **Database issues**: Test migrations with `diesel migration run` and `diesel migration revert`
4. **TUI rendering**: Check terminal backend in `bin/p2p_chat_tui.rs` CrosstermBackend setup
5. **Logs**: Messages appear in TUI Debug tab and stderr; check for truncation at 1000 messages

## File Structure Summary

```
src/
├── lib.rs                    # Core: AppBehaviour, logging, utilities
├── schema.rs                 # Auto-generated (don't edit)
├── models_queryable.rs       # Auto-generated (don't edit)
├── models_insertable.rs      # Auto-generated (don't edit)
└── bin/
    ├── p2p_chat.rs           # CLI/headless entry point
    └── p2p_chat_tui.rs       # TUI entry point (conditional compile)
tests/
├── p2p_integration.rs        # Network behavior tests
└── tui_chat.rs               # TUI component tests
migrations/                   # SQL schema migrations
├── 2025-08-29-221807_identities/
├── 2026-04-04-225730_messages/
├── 2026-04-05-000000_peers/
├── 2026-04-05-000001_peers_timestamps/
├── 2026-04-05-000002_peer_sessions/
├── 2026-04-05-040410_direct_messages/
├── 2026-04-06-120000_identity_ports/
└── 2026-04-14-154344-0000_add_peer_nicknames/
```

## Links to Key Files

- Database queries & helpers: `src/lib.rs`
- TUI implementation: `src/bin/p2p_chat_tui.rs`
- Integration tests: `tests/p2p_integration.rs`
- Project config: `Cargo.toml` (features, dependencies)
- Dev environment: `flake.nix`, `.envrc`
