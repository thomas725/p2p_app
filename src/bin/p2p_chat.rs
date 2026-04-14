use libp2p::{futures::StreamExt, gossipsub, noise, tcp, yamux};
use p2p_app::{build_behaviour, get_libp2p_identity, get_network_size, init_logging, p2plog_info};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, BufReader};
use tokio::select;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();
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

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(get_libp2p_identity()?)
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

    let topic = gossipsub::IdentTopic::new("test-net");
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        select! {
            event = swarm.select_next_some() => {
                match event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        p2plog_info(format!("Listening on: {}", address));
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let _ = p2p_app::save_listen_ports(Some(port), None);
                        }
                        #[cfg(feature = "quic")]
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let (tcp, _) = p2p_app::load_listen_ports().unwrap_or((None, None));
                            let _ = p2p_app::save_listen_ports(tcp, Some(port));
                        }
                    }
                    libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        p2plog_info(format!("Connected to: {} (peers: 1)", peer_id));
                    }
                    libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        p2plog_info(format!("Disconnected from: {}", peer_id));
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(p2p_app::AppBehaviourEvent::Gossipsub(
                        libp2p::gossipsub::Event::Message { propagation_source, message, .. }
                    )) => {
                        let msg = String::from_utf8_lossy(&message.data);
                        let sender = propagation_source.to_string();
                        p2plog_info(format!("[{}] {}", &sender[..8.min(sender.len())], msg));
                    }
                    _ => {}
                }
            }
            line = lines.next_line() => {
                if let Ok(Some(text)) = line {
                    let text_str: String = text;
                    if !text_str.is_empty() {
                        let _ = swarm.behaviour_mut().gossipsub.publish(topic.clone(), text_str.as_bytes());
                    }
                }
            }
        }
    }
}
