# AGENTS.md - Developer Guidelines for p2p_app

This is a Rust project using Diesel (SQLite), libp2p (P2P networking), and tokio.

## Features

- **P2P Chat**: gossipsub for broadcast messages, request-response for direct messages
- **TUI**: Interactive terminal UI with 4 tabs (Chat, Peers, Direct, Log)
- **Peer Discovery**: mDNS for local peer discovery
- **Persistence**: SQLite database for messages, peers, and identity
- **Mouse**: Click tabs to switch, click peers to open DM

## User Stories

### As a user, I can

1. Broadcast messages to all connected peers via gossipsub
2. Send direct messages to specific peers
3. See a list of discovered peers with their connection status
4. Set local nicknames for peers for easier identification
5. View chat history in the Chat tab
6. View message history with specific peers in DM tabs

### As a developer, I can

1. Run the application in TUI mode or headless mode
2. Configure network mesh parameters based on expected peer count
3. Automatically recover from stale peer addresses via mDNS
4. View logs with timestamps in the Log tab

## Build & Run Commands

```bash
# Build the project
cargo build

# Build release version (uses build_release.sh)
./build_release.sh

# Run the default TUI binary
cargo run

# Run with custom database
DATABASE_URL=my.db cargo run

# Run headless version (no TUI, with mdns and tracing)
cargo run --no-default-features --features mdns,tracing

# Run CLI version (no TUI, no mdns)
cargo run --no-default-features --features tracing

# Run integration tests
cargo test --test p2p_integration
```

### Binary Selection

The project has three binaries defined in Cargo.toml:

| Binary | Description | Run Command |
|--------|-------------|-------------|
| `p2p_chat` | CLI (basic, no UI) | `cargo run --bin p2p_chat` |
| `p2p_chat_tui` | TUI frontend | `cargo run` (default) |
| `p2p_chat_dioxus` | Dioxus GUI | `cargo run --bin p2p_chat_dioxus --features dioxus-desktop` |

**Note:** The Dioxus GUI requires GTK/WebKit system libraries. See the Dioxus section below.

## Linting & Formatting

```bash
# Format code (Rustfmt)
cargo fmt

# Run Clippy lints
cargo clippy

# Run Clippy with fix suggestions (allows dirty working directory)
cargo clippy --fix --allow-dirty

# Run Clippy with strict warnings (as in CI)
cargo clippy -- -D warnings
```

### Testing revisted

```bash
# Run all tests
cargo test

# Run a single test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test p2p_integration

# Run TUI tests
cargo test --test tui_chat

# Run a specific TUI test
cargo test --test tui_chat test_name
```

## Code Style Guidelines

### Imports

- Use underscore suffixes for trait imports used for methods: `use tokio::io::AsyncBufReadExt as _`
- Group imports: std first, then external crates, then crate modules
- Use `use` statements at module level, not inline
- For libp2p, prefer specific feature imports when possible

### Formatting

- Run `cargo fmt` before committing
- Use 4 spaces for indentation (Rust default)
- Maximum line length: 100 characters (default rustfmt)
- Prefer explicit returns over implicit returns in public functions

### Types

- Use explicit return types on public functions
- Prefer `Result<T, color_eyre::Report>` for error handling
- Use the type system; avoid `unwrap()` in production code
- For internal fallible functions, use `Option<T>` or `Result<T, SpecificError>`

### Naming Conventions

- `snake_case` for variables, functions, modules, files
- `PascalCase` for types, traits, enums
- `SCREAMING_SNAKE_CASE` for constants
- Prefix private fields with underscore: `struct Foo { _private: T }`
- Suffix trait imports used for methods with underscore: `AsyncBufReadExt as _`

### Error Handling

- Use `color_eyre` for errors (already configured in project)
- Use `wrap_err_with()` or `eyre!()` for context
- Use `tracing` for logging errors: `tracing::error!("description: {e}")`
- Avoid silent failures: use `.ok()` or `.map_err()` explicitly
- In applications, handle errors at boundaries; in libraries, return them

### Database (Diesel)

- Models are in `src/models_queryable.rs` and `src/models_insertable.rs`
- Schema is auto-generated in `src/schema.rs` - do not edit manually
- Run `./diesel_generate.sh` to regenerate after schema changes
- Migrations live in `./migrations/` directory
- **Migration naming convention**: `YYYY-MM-DD-HHMMSS_description` (the 6 digits after the date are hours, minutes, seconds, e.g. `2026-04-06-120000_add_ports`)
- Use Diesel's query builder rather than raw SQL when possible
- Handle database errors with `color_eyre::Report`

### Async Code (tokio)

- Use `#[tokio::main]` for async main functions
- Prefer `tokio::select!` for concurrent operations
- Use `StreamExt` for stream processing
- For CPU-intensive tasks, use `tokio::task::spawn_blocking`
- Prefer `tokio::sync` primitives over `std::sync` for async contexts
- Avoid blocking the tokio runtime with long synchronous operations

### libp2p

- Custom behaviours must derive `NetworkBehaviour`
- Use `SwarmEvent` for handling swarm events
- Topic subscription: use `gossipsub::IdentTopic`
- Direct messages use `request_response::Behaviour` with JSON codec
- Handle connection events properly (listener, dialer)
- Use appropriate timeouts for request-response protocols
- When handling events, match on specific event types rather than using wildcards

### Project Structure

```
src/
├── lib.rs           # Library root, database logic, DirectMessage codec
├── schema.rs        # Auto-generated Diesel schema
├── models_*.rs      # Auto-generated model structs
├── bin/
│   ├── p2p_chat.rs          # CLI binary entry point
│   └── p2p_chat_tui.rs      # TUI binary entry point
migrations/          # SQL migration files
sqlite.db            # SQLite database (created at runtime)
target/              # Compiled artifacts (gitignored)
```

## TUI Usage

The TUI has tabs accessible with `Tab` key:

- **Chat**: Broadcast messages to all peers (via gossipsub)
- **Peers**: List discovered/connected peers, press Enter to open DM
- **Direct** (dynamic): Direct message tabs open with selected peers
- **Log**: Log output and system messages

Input is only enabled in Chat and Direct tabs. Press `Ctrl+Q` to quit.

### TUI Commands

- `/nick <name>` - Set your display name (sent to peers)
- `/nick` - Show current nickname
- `/setpeer <peer-id> <name>` - Set local nickname for a peer (not transmitted)
- `Ctrl+W` - Close current DM tab
- Mouse: Click tabs to switch, click peers in Peers tab to open DM

## Development Environment

The project uses Nix for reproducible dev environments:

```bash
# Enter development shell (if flake is enabled)
nix develop

# Or use direnv (auto-loaded via .envrc)
```

Required system packages (see flake.nix): cargo, rustc, rustfmt, clippy, pkg-config, openssl, sqlite, clang, lld, upx, binutils, gcc.

### Shell Scripts

- `./diesel_generate.sh` - Regenerates Diesel models and runs migrations
- `./build_release.sh` - Builds optimized release binary

## Notes

- SQLite database file: defaults to `sqlite.db` or set via `DATABASE_URL`
- Embedded migrations run automatically on startup
- Identity keypair is generated and stored in database on first run
- Messages are persisted with `is_direct` flag for broadcast vs direct
- Direct messages use libp2p's request-response protocol (encrypted via Noise)
- CLI version reads from stdin and writes to stdout/stderr
- TUI version provides interactive interface with multiple tabs

### Nickname System

- Generated petname (e.g., "brave-wolf") used as default on first run
- Self nickname: stored locally, sent to remote peers with messages
- Peer local nickname: stored locally only, displayed instead of peer ID
- Peer received nickname: nickname received from peer, stored locally
- Priority: local nickname > received nickname > peer ID suffix

## Testing

### TUI Tests

TUI tests are in `tests/tui_chat.rs` and test the mouse click to peer mapping logic:

```bash
# Run all TUI tests
cargo test --test tui_chat

# Run a specific test
cargo test --test tui_chat test_name
```

### Writing TUI Tests

Use the `TuiTestState` struct to simulate UI state:

```rust
use tui_chat::{TuiTestState, TEST_MESSAGES};

// Create state with test messages
let state = TuiTestState::new();

// Simulate clicking a row
let peer = state.handle_mouse_click(4); // row 4

// Verify the correct peer is returned (not hardcoded - uses actual state)
let content_start = state.calculate_content_start_row();
let expected_idx = state.chat_list_state_offset + (4 - content_start) as usize;
if let Some(expected_peer) = state.chat_message_peers.get(expected_idx) {
    assert_eq!(peer, *expected_peer);
}
```

### Test State Components

- `messages`: `VecDeque<String>` - chat messages in display format
- `chat_message_peers`: `Vec<String>` - extracted peer IDs from messages
- `active_tab`: `usize` - current tab (0=Chat, 1=Peers, 2=Direct, 3=Log)
- `chat_list_state_offset`: `usize` - scroll offset
- `unread_broadcasts` / `unread_dms`: notification state

### Key Functions

- `handle_mouse_click(row)` - returns peer ID for clicked row
- `calculate_content_start_row()` - returns row where messages start (accounts for tabs + notifications)

### Integration Tests

Integration tests are in `tests/p2p_integration.rs` and test actual networking functionality:

```bash
# Run integration tests
cargo test --test p2p_integration
```

These tests spawn actual nodes and test message passing between them.

## Dioxus Frontend

The project supports a Dioxus GUI frontend alongside the TUI.

### Building Dioxus

```bash
# Install Dioxus CLI
cargo install dioxus-cli

# Build and run
cargo run --bin p2p_chat_dioxus --features dioxus-desktop
```

### System Dependencies (Linux)

```bash
# Debian/Ubuntu
sudo apt install libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev libglib2.0-dev libpango1.0-dev libcairo2-dev libgdk-pixbuf2.0-dev
```

See `flake.nix` for NixOS dependencies.

### Dioxus File Structure

```
src/bin/p2p_chat_dioxus.rs  # Main Dioxus frontend
```

### Key Dioxus Patterns

State management with Signals:

```rust
let mut count = use_signal(0);
count.set(count() + 1);
```

Event handlers:

```rust
onclick: move |_| { /* handler */ }
onkeydown: move |evt| { if evt.key() == Key::Enter { ... } }
```

RSX! macro for UI:

```rust
rsx! {
    div {
        button { onclick: move |_| count -= 1, "-" }
        p { "{count}" }
        button { onclick: move |_| count += 1, "+" }
    }
}
```

### Connecting to P2P Networking

The Dioxus frontend should:

1. Subscribe to shared broadcast channels for messages and peer events
2. Update Signals when events are received
3. Send messages through the libp2p swarm when the user submits

See `docs/dual_frontend_architecture.md` for architecture details.
