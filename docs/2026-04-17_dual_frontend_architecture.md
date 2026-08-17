# Dual Frontend Architecture

## Goal

Support both TUI and GUI frontends from the same codebase:
- **TUI** (Terminal User Interface) - Small binaries, SSH-friendly, low resource
- **Dioxus GUI** - Rich UI, cross-platform native apps, web deployment

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      p2p_app                                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    lib.rs                            │   │
│  │  - Network behavior (libp2p)                        │   │
│  │  - Database operations (Diesel/SQLite)              │   │
│  │  - Message types (Broadcast, Direct)                │   │
│  │  - Shared state management                          │   │
│  │  - P2P networking logic                            │   │
│  └─────────────────────────────────────────────────────┘   │
│                           │                                 │
│            ┌──────────────┴──────────────┐                 │
│            ▼                             ▼                 │
│  ┌─────────────────────┐     ┌─────────────────────┐       │
│  │    TUI Frontend     │     │   Dioxus Frontend   │       │
│  │  (ratatui + TUI)   │     │  (Desktop/Web/Mobile) │      │
│  │                     │     │                     │       │
│  │ - p2p_chat_tui.rs  │     │ - p2p_chat_dioxus/ │       │
│  │ - Terminal styling  │     │ - WebView rendering │       │
│  │ - Keyboard nav      │     │ - CSS styling       │       │
│  └─────────────────────┘     └─────────────────────┘       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Shared Components

### Message Types
Both frontends work with the same message types:

```rust
pub struct BroadcastMessage {
    pub content: String,
    pub sent_at: Option<f64>,
}

pub struct DirectMessage {
    pub content: String,
    pub timestamp: i64,
    pub sent_at: Option<f64>,
}
```

### Channel-Based Communication
Frontends receive events via channels:

```rust
// In lib.rs
pub struct AppState {
    pub messages: broadcast::Sender<ChatMessage>,
    pub peers: broadcast::Sender<PeerEvent>,
}
```

### Event Types
```rust
pub enum UiEvent {
    NewMessage(Message),
    NewPeer(Peer),
    PeerDisconnected(PeerId),
    ConnectionEstablished(PeerId),
}
```

## Feature Flags

```toml
[features]
default = ["mdns", "tracing", "quic", "tui"]
tui = ["dep:ratatui", ...]
dioxus-desktop = ["dioxus", "dioxus-desktop"]
dioxus-web = ["dioxus", "dioxus-web"]
```

## Directory Structure

```
p2p_app/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Shared core
│   ├── schema.rs           # Database schema
│   ├── models_*.rs        # Database models
│   └── bin/
│       ├── p2p_chat.rs     # CLI (no TUI)
│       ├── p2p_chat_tui.rs # TUI frontend
│       └── p2p_chat_dioxus.rs  # Dioxus frontend
├── migrations/
├── docs/
└── dioxus/                # Dioxus components (optional)
```

## Frontend Responsibilities

### TUI Frontend
- Terminal rendering with ratatui
- Keyboard navigation
- Mouse click handling (as implemented)
- Tab-based navigation (Chat, Peers, Direct, Log)
- ANSI color styling
- SSH-compatible

### Dioxus Frontend
- WebView rendering
- CSS styling
- Native window management
- Web deployment option
- Mobile deployment option
- Hot reloading during development

## State Synchronization

Both frontends subscribe to the same channels:

```rust
// Frontend initialization
let mut message_rx = app_state.messages.subscribe();
let mut peer_rx = app_state.peers.subscribe();

// Event loop
loop {
    tokio::select! {
        Some(msg) = message_rx.recv() => {
            // Update UI
        }
        Some(event) = peer_rx.recv() => {
            // Update peer list
        }
    }
}
```

## Build Targets

| Target | Command | Output |
|--------|---------|--------|
| TUI (default) | `cargo run` | ~2-3MB binary |
| TUI explicit | `cargo run -F tui` | ~2-3MB binary |
| Dioxus Desktop | `cargo run -F dioxus-desktop` | ~10-50MB binary |
| Dioxus Web | `dx serve` | WASM + HTML |

## Future Considerations

1. **Shared UI components** - Extract common components that work in both contexts
2. **Web deployment** - Serve Dioxus GUI as web app
3. **Mobile app** - Compile to iOS/Android
4. **Embedded mode** - Library mode for embedding in other applications
