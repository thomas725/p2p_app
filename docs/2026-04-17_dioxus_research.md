# Dioxus Frontend Research

## Overview

Dioxus is a framework for building cross-platform apps in Rust. It supports:
- **Web** (WASM)
- **Desktop** (Windows, macOS, Linux) - uses WebView
- **Mobile** (iOS, Android) - uses WebView
- **Native** (experimental, uses WGPU)

## Key Features

### Cross-Platform Single Codebase
One codebase can target all platforms with platform-specific code via feature flags:

```toml
[features]
default = []
web = ["dioxus/web"]
desktop = ["dioxus/desktop"]
mobile = ["dioxus/mobile"]
```

### React-Like API
Familiar mental model with RSX! macro for UI:

```rust
use dioxus::prelude::*;

fn app() -> Element {
    let mut count = use_signal(0);
    
    rsx! {
        div {
            button { onclick: move |_| count -= 1, "Decrement" }
            p { "{count}" }
            button { onclick: move |_| count += 1, "Increment" }
        }
    }
}

fn main() {
    dioxus::launch(app);
}
```

### Hot Reloading
Binary patching (Subsecond) enables runtime Rust code updates without restart.

### Signal-Based State
```rust
let mut count = use_signal(0);
count.set(count() + 1);  // Clone-free reactive state
```

### Fullstack/Server Functions
Type-safe RPC between client and server with Axum integration.

### Styling
- CSS support
- Built-in Tailwind integration
- CSS-in-RSX inline styles

## Comparison to TUI (ratatui)

| Aspect | TUI | Dioxus |
|--------|-----|--------|
| Platforms | Terminal only | Web, Desktop, Mobile |
| Binary Size | Small (~2MB) | Larger (~10-50MB) |
| Styling | ANSI colors, limited | Full CSS |
| Native Feel | Terminal | Native WebView |
| SSH Support | Yes | No (unless web-based) |
| Learning Curve | Lower | React-like learning curve |
| Ecosystem | Mature | Growing |

## Dioxus Renderer Types

1. **Web** - WASM compilation, DOM manipulation
2. **Desktop** - WebView (uses `wry`/`tao`)
3. **Mobile** - WebView (uses `tauri` mobile)
4. **Native (Blitz)** - WGPU-based, experimental
5. **LiveView** - Server-rendered via WebSocket
6. **TUI** - **Deprecated**, may return with Blitz

## Architecture for P2P Chat App

### Shared Core
```
src/
├── lib.rs              # Core networking, DB, shared logic
├── bin/
│   ├── p2p_chat_tui/   # TUI frontend
│   └── p2p_chat_gui/   # Dioxus frontend (future)
```

### Integration Strategy

1. **Add Dioxus binary** with feature flag
2. **Shared state via channels** - TUI and Dioxus both read from same channels
3. **Platform-specific code** in `#[cfg(feature = "dioxus")]` blocks
4. **Component abstraction** - UI components isolated for potential reuse

### Cargo.toml Setup

```toml
[features]
default = [ "mdns", "tracing", "quic", "tui" ]
tui = [ "dep:ratatui", ... ]
dioxus-desktop = [ "dioxus", "dioxus-desktop" ]

[dependencies]
dioxus = { version = "0.7", optional = true }
dioxus-desktop = { version = "0.7", optional = true }

[[bin]]
name = "p2p_chat_dioxus"
path = "src/bin/p2p_chat_dioxus.rs"
required-features = ["dioxus-desktop"]
```

## Resources

- [Dioxus Documentation](https://dioxuslabs.com/learn/0.7/)
- [Dioxus GitHub](https://github.com/dioxuslabs/dioxus)
- [Awesome Dioxus](https://github.com/dioxuslabs/awesome-dioxus)

## System Dependencies

### Linux (Debian/Ubuntu)
```bash
sudo apt install libgtk-3-dev libsoup-3.0-dev libwebkit2gtk-4.1-dev libglib2.0-dev libpango1.0-dev libcairo2-dev libgdk-pixbuf2.0-dev
```

### Linux (NixOS/flakes)
Add to `flake.nix`:
```nix
gtk3
libsoup
webkitgtk_4_1
glib
pango
cairo
gdk-pixbuf
```

### macOS
```bash
brew installwebkitkit gtk+3
```

### Windows
Install [GTK3 from msys2](https://github.com/tschoonj/GTK-for-Windows-Runtime-Environment-Installer) or use WSL.

## Version History

- **0.7** (2025) - Native renderer (Blitz), hot-patching, Radix-UI primitives
- **0.6** (Dec 2024) - Mobile support, improved CLI, SSG
- **0.5** - Signal-based state, cross-platform launch
