
#[cfg(feature = "tui")]
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();

    let logs = std::sync::Arc::new(std::sync::Mutex::new(VecDeque::new()));
    let logs_callback = logs.clone();
    p2p_app::logging::set_tui_log_callback(move |msg| {
        if let Ok(mut l) = logs_callback.lock() {
            l.push_back(msg);
            if l.len() > 1000 {
                l.pop_front();
            }
        }
    });

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

    tui::run_tui(swarm, "test-net".to_string(), logs).await
}
