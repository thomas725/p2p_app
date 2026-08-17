# Dioxus Frontend Implementation

## Status: Initial Setup Complete

A minimal Dioxus frontend has been created at `src/bin/p2p_chat_dioxus.rs`.

## Features Implemented

### Basic UI
- Header with app title
- Tab bar (Chat, Peers, Log)
- Dark theme styling
- Message display area
- Text input with send button

### Components
- `app()` - Main component with tab navigation
- `chat_view()` - Chat messages display and input
- `peers_view()` - Peer list display (placeholder)
- `log_view()` - Log display (placeholder)

## Next Steps

### High Priority
1. **Integrate with shared P2P networking** - Connect Dioxus UI to libp2p swarm
2. **Shared state channels** - Create broadcast channels for messages/peers
3. **Real peer discovery** - Display discovered peers from mDNS
4. **Message sending** - Wire up input to gossipsub broadcasting

### Medium Priority
1. **Direct messages** - DM tab per peer
2. **Peer nicknames** - Display local nicknames
3. **Connection status** - Show connected/disconnected state
4. **Auto-scroll** - Scroll to newest messages

### Low Priority
1. **Styling polish** - Better colors, animations
2. **Keyboard shortcuts** - Ctrl+N for next tab, etc.
3. **Notifications** - Desktop notifications for new messages
4. **Settings panel** - Nickname configuration

## Running the Dioxus Frontend

```bash
# Install Dioxus CLI
cargo install dioxus-cli

# Install system dependencies (Linux)
sudo apt install libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev

# Run
cargo run --bin p2p_chat_dioxus --features dioxus-desktop
```

## File Structure

```
src/
├── lib.rs                    # Shared P2P networking
├── bin/
│   ├── p2p_chat.rs          # CLI binary
│   ├── p2p_chat_tui.rs      # TUI frontend
│   └── p2p_chat_dioxus.rs   # Dioxus frontend
```

## Architecture Notes

### State Management
Dioxus uses Signal-based state:
```rust
let mut messages = use_signal(Vec::<Message>::new);
messages.write().push(new_message);
```

### Event Handling
```rust
onclick: move |_| { /* handler */ }
onkeydown: move |evt| { if evt.key() == Key::Enter { ... } }
```

### Components
```rust
#[component]
fn MyComponent(prop: String) -> Element {
    rsx! { div { "{prop}" } }
}
```
