//! TUI binary — ratatui-based terminal interface and headless fallback entry point.

use libp2p::gossipsub;
use p2p_app::build_swarm;
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
    pub mod main_loop;
    pub mod message_handlers;
    pub mod render_loop;
    pub mod scroll_handlers;
    pub mod state;
    #[cfg(test)]
    #[path = "../../../tests/unit/unit_bin_tui_test_helpers.rs"]
    pub mod test_helpers;
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();
    p2p_app::logging::set_tui_callback(|_| p2p_app::logging::request_tui_redraw());

    // Initialize database once at startup (logs database path and peer ID)
    let _db = p2p_app::init_database()?;

    let topic = gossipsub::IdentTopic::new(p2p_app::CHAT_TOPIC);

    let network_size = match p2p_app::get_network_size() {
        Ok(size) => {
            p2plog_info(format!("Network size detected: {size:?}"));
            size
        }
        Err(e) => {
            p2plog_info(format!(
                "Could not determine network size, defaulting to Small: {e}"
            ));
            p2p_app::NetworkSize::Small
        }
    };

    let mut swarm = build_swarm(network_size)?;

    let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(listen_addr)?;

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Use new 4-task architecture instead of monolithic tokio::select!
    tui::main_loop::run_new_tui(swarm, p2p_app::CHAT_TOPIC.to_string()).await
}
