// TUI binary — ratatui-based terminal interface and headless fallback entry point.

use std::sync::Arc;

use p2p_app::logging::{init_logging, p2plog_info};

#[cfg(feature = "tui")]
mod tui {
    //! Four-task TUI architecture for `p2p_app`
    //!
    //! ## Overview
    //!
    //! This module implements a concurrent, event-driven terminal UI using async Rust.
    //! Four independent tasks communicate via MPSC channels, each responsible for a specific concern:
    //!
    //! 1. **`SwarmHandler`** - Listens to libp2p network events and translates them to app-level `SwarmEvent`
    //! 2. **`InputHandler`** - Polls terminal for keyboard/mouse input and sends `InputEvent`
    //! 3. **`CommandProcessor`** - Receives `InputEvent` and `SwarmEvent`, mutates `AppState`, sends `SwarmCommand`
    //! 4. **`RenderLoop`** - Reads `AppState` and renders the TUI on demand
    //!
    //! ## Architecture
    //!
    //! ```text
    //! ┌─────────────────────────────────────────────────────┐
    //! │                  libp2p Swarm                       │
    //! └────────────────────────┬────────────────────────────┘
    //!                          │
    //!                          ↓
    //!                  SwarmHandler Task
    //!                    (libp2p events)
    //!                          │
    //!                          ↓ SwarmEvent (mpsc)
    //!          ┌───────────────────────────────┐
    //!          │    CommandProcessor Task      │
    //!          │  ┌─────────────────────────┐  │
    //!          │  │  Shared AppState        │  │
    //!          │  │ (Arc<Mutex<AppState>>)  │  │
    //!          │  └─────────────────────────┘  │
    //!          └───────────────────────────────┘
    //!                 ↑              ↑
    //!    InputEvent   │              │ SwarmEvent
    //!      (mpsc)     │              │ (mpsc)
    //!                 │              │
    //!          InputHandler Task  SwarmHandler Task
    //!                 ↑
    //!          Terminal Input
    //!
    //! RenderLoop continuously reads AppState and redraws terminal.
    //! ```
    //!
    //! ## State Management
    //!
    //! All TUI state is centralized in `AppState` behind an `Arc<Mutex<>>` for safe concurrent access.
    //! Only `CommandProcessor` mutates state; other tasks read-only or signal mutations through channels.
    //!
    //! Channel Capacity: 100 events max in flight per channel (prevents unbounded buffering).
    //!
    //! ## Key Design Decisions
    //!
    //! - **`Arc<Mutex>` over `RwLock`**: Simplicity. Most operations read and modify multiple fields atomically.
    //! - **Polling input instead of event subscriptions**: Works on all platforms with crossterm.
    //! - **Event-driven redraws**: UI updates only when input or network state changes.
    //! - **Immutable channel types**: Each task has dedicated input/output channels, no shared mutable channels.

    pub use p2p_app::tui_tabs::DynamicTabs;
    use ratatui_textarea::TextArea;

    pub mod click_handlers;
    mod command_processor;
    pub mod event_source;
    pub mod input_processor;
    pub mod key_probe;
    pub mod main_loop;
    pub mod message_handlers;
    pub mod render_loop;
    pub mod scroll_handlers;
    pub mod state;
    #[cfg(all(test, not(feature = "unified")))]
    #[path = "../../../tests/unit/unit_bin_tui_test_helpers.rs"]
    pub mod test_helpers;
}

#[tokio::main]
#[allow(clippy::large_futures)]
async fn main() -> color_eyre::Result<()> {
    run_tui().await
}

/// Run the TUI frontend.
///
/// # Errors
/// Returns an error if terminal/database/swarm setup fails.
#[allow(clippy::large_futures)]
pub async fn run_tui() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();
    p2p_app::logging::register_log_callback(Arc::new(|_| p2p_app::logging::request_tui_redraw()));

    // Diagnostic mode: dump raw terminal key bytes so Tab vs Ctrl+I encoding
    // can be inspected per terminal. Runs before any database/network setup.
    if std::env::args().nth(1).as_deref() == Some("key-probe") {
        return tui::key_probe::run();
    }

    // Public DHT (IPFS) opt-in flags. --public-dht enables joining the
    // public IPFS Kademlia DHT for peer discovery; --no-public-dht forces
    // LAN-only mode. Defaults to off (LAN-only) unless P2P_PUBLIC_DHT or
    // P2P_BOOTSTRAP is set.
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--public-dht" => unsafe { std::env::set_var("P2P_PUBLIC_DHT", "1") },
            "--no-public-dht" => unsafe { std::env::set_var("P2P_PUBLIC_DHT", "0") },
            _ => {}
        }
    }

    // Initialize database once at startup (logs database path and peer ID)
    let _db = p2p_app::init_database()?;

    // Start the shared mobile_node backend (single node) so the TUI and any
    // web frontend share one swarm. (Option A step 2.)
    let peer_id = p2p_app::mobile_node::start_node_auto()
        .map_err(|e| color_eyre::eyre::eyre!(e))?;
    p2plog_info(format!("Node started: peer_id={peer_id}"));

    // Use new 4-task architecture instead of monolithic tokio::select!
    tui::main_loop::run_new_tui().await
}
