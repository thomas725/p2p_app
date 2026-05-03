use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::build_behaviour;
use p2p_app::logging::{init_logging, p2plog_info};
use std::time::Duration;

#[cfg(feature = "tui")]
mod tui {
    pub mod helpers;
    pub use p2p_app::tui_tabs::DynamicTabs;
    use ratatui_textarea::TextArea;

    pub mod click_handlers;
    mod command_processor;
    pub mod constants;
    mod event_source;
    mod input_processor;
    pub mod main_loop;
    mod message_handlers;
    mod render_loop;
    pub mod scroll_handlers;
    mod state;
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();
    p2p_app::logging::set_tui_callback(|_| {});

    // Initialize database once at startup (logs database path and peer ID)
    let _db = p2p_app::init_database()?;

    let topic = gossipsub::IdentTopic::new("test-net");

    let network_size = match p2p_app::get_network_size() {
        Ok(size) => {
            p2plog_info(format!("Network size detected: {:?}", size));
            size
        }
        Err(e) => {
            p2plog_info(format!(
                "Could not determine network size, defaulting to Small: {}",
                e
            ));
            p2p_app::NetworkSize::Small
        }
    };

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?;

        #[cfg(feature = "quic")]
        let swarm = base
            .with_quic()
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        #[cfg(not(feature = "quic"))]
        let swarm = base
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm
    };

    let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(listen_addr)?;

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    // Use new 4-task architecture instead of monolithic tokio::select!
    tui::main_loop::run_new_tui(swarm, "test-net".to_string()).await
}
