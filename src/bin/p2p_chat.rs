use libp2p::{gossipsub, noise, tcp, yamux};
use p2p_app::logging::p2plog_info;
use p2p_app::{build_behaviour, get_libp2p_identity, get_network_size};
use tokio::io::{AsyncBufReadExt as _, BufReader};

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    p2p_app::logging::init_logging();
    p2plog_info("Starting P2P Chat CLI".to_string());

    let network_size = match get_network_size() {
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

    let (swarm, topic) = if cfg!(feature = "quic") {
        let swarm_builder = libp2p::SwarmBuilder::with_existing_identity(get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?;
        let swarm = swarm_builder
            .with_quic()
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))
            .unwrap()
            .build();
        let mut swarm = swarm;
        let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
        swarm.listen_on(listen_addr);
        let topic = gossipsub::IdentTopic::new("test-net");
        swarm.behaviour_mut().gossipsub.subscribe(&topic);
        (swarm, topic)
    } else {
        let swarm_builder = libp2p::SwarmBuilder::with_existing_identity(get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?;
        let swarm = swarm_builder
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))
            .unwrap()
            .build();
        let mut swarm = swarm;
        let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
        swarm.listen_on(listen_addr);
        let topic = gossipsub::IdentTopic::new("test-net");
        swarm.behaviour_mut().gossipsub.subscribe(&topic);
        (swarm, topic)
    };

    let swarm = std::sync::Arc::new(swarm);
    let swarm_clone = swarm.clone();
    let topic_clone = topic.clone();
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    let swarm_handle = tokio::spawn(async move {
        use futures::StreamExt;
        let mut swarm_stream = futures::stream::poll_fn(move |cx| swarm_clone.poll_next_unpin(cx));
        while let Some(event) = swarm_stream.next().await {
            match event {
                libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                    p2p_app::logging::p2plog_info(format!("Listening on: {}", address));
                    if let Some(port) = address.iter().find_map(|p| match p {
                        libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                        _ => None,
                    }) {
                        let _ = p2p_app::save_listen_ports(Some(port), None);
                    }
                    #[cfg(feature = "quic")]
                    if let Some(port) = address.iter().find_map(|p| match p {
                        libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                        _ => None,
                    }) {
                        let (tcp, _) = p2p_app::load_listen_ports().unwrap_or((None, None));
                        let _ = p2p_app::save_listen_ports(tcp, Some(port));
                    }
                }
                libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Connected to: {} (peers: 1)", peer_id));
                }
                libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Disconnected from: {}", peer_id));
                }
                libp2p::swarm::SwarmEvent::Behaviour(p2p_app::AppBehaviourEvent::Gossipsub(
                    libp2p::gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    },
                )) => {
                    let msg = String::from_utf8_lossy(&message.data);
                    let sender = propagation_source.to_string();
                    p2p_app::logging::p2plog_info(format!("[{}] {}", &sender[..8.min(sender.len())], msg));
                }
                _ => {}
            }
        }
    });
        while let Some(event) = swarm_stream.next().await {
            match event {
                libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                    p2p_app::logging::p2plog_info(format!("Listening on: {}", address));
                    if let Some(port) = address.iter().find_map(|p| match p {
                        libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                        _ => None,
                    }) {
                        let _ = p2p_app::save_listen_ports(Some(port), None);
                    }
                    #[cfg(feature = "quic")]
                    if let Some(port) = address.iter().find_map(|p| match p {
                        libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                        _ => None,
                    }) {
                        let (tcp, _) = p2p_app::load_listen_ports().unwrap_or((None, None));
                        let _ = p2p_app::save_listen_ports(tcp, Some(port));
                    }
                }
                libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Connected to: {} (peers: 1)", peer_id));
                }
                libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Disconnected from: {}", peer_id));
                }
                libp2p::swarm::SwarmEvent::Behaviour(p2p_app::AppBehaviourEvent::Gossipsub(
                    libp2p::gossipsub::Event::Message {
                        propagation_source,
                        message,
                        ..
                    },
                )) => {
                    let msg = String::from_utf8_lossy(&message.data);
                    let sender = propagation_source.to_string();
                    p2p_app::logging::p2plog_info(format!("[{}] {}", &sender[..8.min(sender.len())], msg));
                }
                _ => {}
            }
        }
    });

    let lines_handle = tokio::spawn(async move {
        while let Some(line) = lines.next_line().await.unwrap_or_default() {
            if !line.is_empty() {
                let _ = swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic_clone.clone(), line.as_bytes());
            }
        }
    });

    let _ = tokio::join!(swarm_handle, lines_handle);
    Ok(())
}
