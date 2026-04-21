use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::logging::init_logging;
use p2p_app::build_behaviour;
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "tui")]
mod tui {
    use ratatui_textarea::TextArea;
    pub use p2p_app::tui_tabs::DynamicTabs;

    pub mod constants;
    pub mod main_loop;
    mod state;
    mod tracing_writer;
    mod command_processor;
    mod input_handler;
    mod render_loop;
}

#[cfg(feature = "tui")]
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();

    let logs = Arc::new(std::sync::Mutex::new(VecDeque::new()));

    let topic = gossipsub::IdentTopic::new("test-net");

    let network_size = match p2p_app::get_network_size() {
        Ok(size) => {
            eprintln!("Network size detected: {:?}", size);
            size
        }
        Err(e) => {
            eprintln!(
                "Could not determine network size, defaulting to Small: {}",
                e
            );
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
    tui::main_loop::run_new_tui(swarm, "test-net".to_string(), logs).await
}
