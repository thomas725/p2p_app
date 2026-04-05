# p2p_app

A P2P chat application built with Rust, libp2p, and Diesel (SQLite). Features a terminal UI with broadcast messaging, direct messages, peer discovery, and persistent storage.

## Features

- **P2P Chat**: Broadcast messages to all peers via gossipsub protocol
- **Direct Messages**: Private messaging via request-response protocol
- **TUI**: Interactive terminal interface with 4 tabs (Chat, Peers, Direct, Debug)
- **Peer Discovery**: mDNS for automatic local network peer discovery
- **Persistence**: SQLite database for messages, peers, and identity
- **Multiple Transports**: TCP and optional QUIC support
- **Logging**: Debug logging with millisecond timestamps

## Build & Run

```bash
# Build
cargo build

# Run (TUI enabled by default)
cargo run

# Headless mode (no TUI)
cargo run --no-default-features --features mdns,tracing

# With custom database
DATABASE_URL=my.db cargo run
```

## TUI Usage

Press `Tab` to switch between tabs:
- **Chat**: Broadcast messages to all peers
- **Peers**: List discovered peers, press Enter to open DM
- **Direct**: Direct message with selected peer
- **Debug**: System logs and debug output

Press `Ctrl+Q` or `Esc` to quit.

## Architecture

- **libp2p**: P2P networking (gossipsub, request-response, mDNS, TCP/QUIC)
- **Diesel**: ORM for SQLite persistence
- **Ratatui**: Terminal UI framework
- **tokio**: Async runtime

## Project Structure

```
src/
├── lib.rs                    # Library: database, models, DirectMessage codec
├── schema.rs                 # Auto-generated Diesel schema
├── models_insertable.rs      # Insertable models
├── models_queryable.rs       # Queryable models
└── bin/
    └── p2p_chat_example.rs   # Binary: TUI and headless mode entry points
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
- [x] Debug logging system with timestamps
- [x] QUIC transport support
- [x] Tracing integration for libp2p debugging
- [x] Headless CLI mode (no TUI, reads stdin, prints to stderr)
