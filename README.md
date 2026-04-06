# p2p_app

A P2P chat application built with Rust, libp2p, and Diesel (SQLite). Features a terminal UI with broadcast messaging, direct messages, peer discovery, and persistent storage.

## Features

- **P2P Chat**: Broadcast messages to all peers via gossipsub protocol
- **Direct Messages**: Private messaging via request-response protocol
- **TUI**: Interactive terminal interface with 4 tabs (Chat, Peers, Direct, Debug)
- **Peer Discovery**: mDNS for automatic local network peer discovery
- **Persistence**: SQLite database for messages, peers, peer sessions, and identity
- **Multiple Transports**: TCP and optional QUIC support
- **Logging**: Debug logging with millisecond timestamps and libp2p tracing integration
- **Network Size Optimization**: Gossipsub config adapts based on historical peer count
- **Multi-instance Support**: Ephemeral identities when DATABASE_URL is not set

## Build & Run

```bash
# Build
cargo build

# Run (TUI enabled by default)
cargo run

# Headless mode (no TUI, reads stdin, prints to stderr)
cargo run --no-default-features --features mdns,tracing

# With custom database
DATABASE_URL=my.db cargo run
```

## TUI Usage

Press `Tab` to switch between tabs:
- **Chat**: Broadcast messages to all peers
- **Peers**: List discovered peers (sorted by last seen), Up/Down to select, Enter to open DM
- **Direct**: Direct message with selected peer
- **Debug**: System logs and debug output (scrollable with Up/Down)

Press `Ctrl+Q` or `Esc` to quit.

## Architecture

- **libp2p**: P2P networking (gossipsub, request-response, mDNS, TCP/QUIC)
- **Diesel**: ORM for SQLite persistence
- **Ratatui**: Terminal UI framework
- **tokio**: Async runtime
- **tracing-subscriber**: Structured logging with denylist filter

## Project Structure

```
src/
├── lib.rs                    # Library: database, models, tracing filter, utilities
├── schema.rs                 # Auto-generated Diesel schema
├── models_insertable.rs      # Insertable models
├── models_queryable.rs       # Queryable models
└── bin/
    └── p2p_chat_example.rs   # Binary: TUI and headless mode entry points
migrations/                   # SQL migration files
tests/
└── p2p_integration.rs        # Integration tests with tracing
```

## Todo

- [ ] Build for embedded Linux devices (OpenWRT, etc.)
- [ ] Store contacts in database with usernames
- [ ] Auto-generate user nicknames
- [ ] Improve UI/UX

## Done

- [x] Embedded Diesel migrations for runtime schema management
- [x] Ratatui TUI with 4-tab interface
- [x] Direct messaging via request-response protocol
- [x] Peer discovery via mDNS
- [x] Message and peer persistence in SQLite
- [x] Identity keypair generation and storage
- [x] Peer session tracking (concurrent peer count history)
- [x] Network size optimization (Small/Medium/Large gossipsub configs)
- [x] Debug logging system with timestamps and scrolling
- [x] QUIC transport support
- [x] Tracing integration for libp2p debugging with denylist filter
- [x] Headless CLI mode (reads stdin, prints to stderr)
- [x] Multi-instance support (ephemeral identities without DATABASE_URL)
- [x] Dial known peers from database on startup
- [x] Peer selection with arrow keys in Peers tab
- [x] Peer list sorted by last_seen descending
- [x] ANSI escape code stripping in tracing output
- [x] Integration tests with same tracing filter as main app
