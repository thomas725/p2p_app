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
    #[cfg(test)]
    #[path = "../../../tests/unit/unit_bin_tui_test_helpers.rs"]
    pub mod test_helpers;
}

/// Options for the optional in-process web frontend ("both frontends, one
/// node" mode — the replacement for the removed combined selector binary).
#[derive(Debug, PartialEq, Eq)]
struct WebOptions {
    /// Serve the web frontend alongside the TUI.
    enabled: bool,
    /// TCP port on `127.0.0.1`.
    port: u16,
    /// `flutter build web` output directory, when serving the SPA.
    assets: Option<std::path::PathBuf>,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 8080,
            assets: None,
        }
    }
}

impl WebOptions {
    /// Environment defaults: `P2P_WEB` (truthy), `P2P_WEB_PORT`, `P2P_WEB_ASSETS`.
    fn from_env() -> Self {
        Self {
            enabled: std::env::var("P2P_WEB")
                .is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on")),
            port: std::env::var("P2P_WEB_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            assets: std::env::var_os("P2P_WEB_ASSETS").map(std::path::PathBuf::from),
        }
    }

    /// Apply command-line overrides. `--web-assets` implies `--web`.
    fn apply_args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--web" => self.enabled = true,
                "--web-port" => {
                    if let Some(value) = args.next()
                        && let Ok(port) = value.parse()
                    {
                        self.port = port;
                    }
                }
                "--web-assets" => {
                    if let Some(value) = args.next() {
                        self.assets = Some(std::path::PathBuf::from(value));
                        self.enabled = true;
                    }
                }
                _ => {}
            }
        }
        self
    }
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
    let peer_id =
        p2p_app::mobile_node::start_node_auto().map_err(|e| color_eyre::eyre::eyre!(e))?;
    p2plog_info(format!("Node started: peer_id={peer_id}"));

    // Optional: also serve the web frontend on the same node. This is the
    // "both frontends, one node" mode: the TUI and the browser drive one swarm.
    let web = WebOptions::from_env().apply_args(std::env::args().skip(1));
    #[cfg(feature = "web")]
    if web.enabled {
        let addr: std::net::SocketAddr = format!("127.0.0.1:{}", web.port)
            .parse::<std::net::SocketAddr>()
            .map_err(|e| color_eyre::eyre::eyre!(e))?;
        let serving_assets = web.assets.is_some();
        let assets = web.assets;
        tokio::spawn(async move {
            if let Err(e) = p2p_app::web::serve(p2p_app::web::WebConfig {
                bind: addr,
                credentials: None,
                assets,
            })
            .await
            {
                p2p_app::logging::p2plog_info(format!("web server error: {e}"));
            }
        });
        p2plog_info(format!(
            "Web frontend listening on http://127.0.0.1:{}{}",
            web.port,
            if serving_assets {
                " (serving Flutter web assets)"
            } else {
                " (API only; pass --web-assets <dir> to serve the Flutter bundle)"
            }
        ));
    }
    #[cfg(not(feature = "web"))]
    let _ = web;

    // Use new 4-task architecture instead of monolithic tokio::select!
    tui::main_loop::run_new_tui().await
}

#[cfg(test)]
mod web_options_tests {
    use super::WebOptions;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> WebOptions {
        WebOptions::default().apply_args(args.iter().copied())
    }

    #[test]
    fn defaults_are_off_loopback_8080_without_assets() {
        let options = parse(&[]);
        assert!(!options.enabled);
        assert_eq!(options.port, 8080);
        assert_eq!(options.assets, None);
    }

    #[test]
    fn web_flag_enables_the_frontend() {
        assert!(parse(&["--web"]).enabled);
    }

    #[test]
    fn web_port_overrides_the_default_and_rejects_garbage() {
        assert_eq!(parse(&["--web-port", "9000"]).port, 9000);
        // A non-numeric value leaves the previous port in place.
        assert_eq!(parse(&["--web-port", "nope"]).port, 8080);
        // The last flag wins.
        assert_eq!(
            parse(&["--web-port", "9000", "--web-port", "9100"]).port,
            9100
        );
    }

    #[test]
    fn web_assets_implies_web_and_keeps_the_path() {
        let options = parse(&["--web-assets", "apps/flutter_app/build/web"]);
        assert!(options.enabled);
        assert_eq!(
            options.assets,
            Some(PathBuf::from("apps/flutter_app/build/web"))
        );
    }

    #[test]
    fn unknown_args_are_ignored() {
        assert_eq!(parse(&["--public-dht", "key-probe"]), WebOptions::default());
    }
}
