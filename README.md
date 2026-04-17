# p2p_app

This is meant to become a multi-purpose p2p application. Our first feature set is centered around chat. The main p2p functionality is provided by libp2p.io. Persistence layer uses sqlite + diesel. We have 3 frontends:

* a simple minimal headless but still [interactive cli mode](src/bin/p2p_chat.rs) for the smallest footprint
* The most work so far has gone into the [ratatui based TUI](src/bin/p2p_chat_tui.rs)
* For really nice and cross plattform GUI we've created a [Dioxus stub](src/bin/p2p_chat_dioxus.rs)

## Features

* **P2P Chat**: Broadcast messages to all peers via gossipsub protocol
* **Direct Messages**: Private messaging via request-response protocol
* **Peer Discovery**: mDNS for automatic local network peer discovery
* **Multiple Transports**: TCP and optional QUIC support
* **Logging**: Debug logging with millisecond timestamps and libp2p tracing integration
* **Network Size Optimization**: Gossipsub config adapts based on historical peer count
* **Multi-instance Support**: Ephemeral identities when DATABASE_URL is not set

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
* **Chat**: Broadcast messages to all peers
* **Peers**: List discovered peers (sorted by last seen), Up/Down to select, Enter to open DM
* **Direct**: Direct message with selected peer
* **Debug**: System logs and debug output (scrollable with Up/Down)

Press `Ctrl+Q` or `Esc` to quit.

## Architecture

* **libp2p**: P2P networking (gossipsub, request-response, mDNS, TCP/QUIC)
* **Diesel**: ORM for SQLite persistence
* **Ratatui**: Terminal UI framework
* **tokio**: Async runtime
* **tracing-subscriber**: Structured logging with denylist filter

## Project Structure

```
src/
├── lib.rs                    # Library: database, models, tracing filter, utilities
├── schema.rs                 # Auto-generated Diesel schema
├── models_insertable.rs      # Insertable models
├── models_queryable.rs       # Queryable models
└── bin/
    └── p2p_chat_tui.rs       # Binary: TUI and headless mode entry points
migrations/                   # SQL migration files
tests/
└── p2p_integration.rs        # Integration tests with tracing
```

## Todo

* [ ] Build for embedded Linux devices (OpenWRT, etc.)
* [ ] Store contacts in database with usernames
* [ ] Auto-generate user nicknames
* [ ] Improve UI/UX

## Done

* [x] Embedded Diesel migrations for runtime schema management
* [x] Ratatui TUI with 4-tab interface
* [x] Direct messaging via request-response protocol
* [x] Peer discovery via mDNS
* [x] Message and peer persistence in SQLite
* [x] Identity keypair generation and storage
* [x] Peer session tracking (concurrent peer count history)
* [x] Network size optimization (Small/Medium/Large gossipsub configs)
* [x] Debug logging system with timestamps and scrolling
* [x] QUIC transport support
* [x] Tracing integration for libp2p debugging with denylist filter
* [x] Headless CLI mode (reads stdin, prints to stderr)
* [x] Multi-instance support (ephemeral identities without DATABASE_URL)
* [x] Dial known peers from database on startup
* [x] Peer selection with arrow keys in Peers tab
* [x] Peer list sorted by last_seen descending
* [x] ANSI escape code stripping in tracing output
* [x] Integration tests with same tracing filter as main app
