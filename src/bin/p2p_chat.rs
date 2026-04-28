use libp2p::{futures::StreamExt, gossipsub, noise, tcp, yamux};
use p2p_app::logging::p2plog_info;
use p2p_app::{BroadcastMessage, build_behaviour, get_libp2p_identity, get_network_size};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, BufReader};

enum Event {
    Swarm(Box<libp2p::swarm::SwarmEvent<p2p_app::behavior::AppBehaviourEvent>>),
    Stdin(Option<String>),
}

fn extract_tcp_port(address: &libp2p::Multiaddr) -> Option<i32> {
    address
        .iter()
        .find_map(|p| match p {
            libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
            _ => None,
        })
}

#[cfg(feature = "quic")]
fn extract_udp_port(address: &libp2p::Multiaddr) -> Option<i32> {
    address
        .iter()
        .find_map(|p| match p {
            libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
            _ => None,
        })
}

fn current_timestamp() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn handle_listen_addr_event(address: &libp2p::Multiaddr) {
    p2p_app::logging::p2plog_info(format!("Listening on: {}", address));
    if let Some(port) = extract_tcp_port(address) {
        let _ = p2p_app::save_listen_ports(Some(port), None);
    }
    #[cfg(feature = "quic")]
    if let Some(port) = extract_udp_port(address) {
        let (tcp, _) = p2p_app::load_listen_ports().unwrap_or((None, None));
        let _ = p2p_app::save_listen_ports(tcp, Some(port));
    }
}

fn handle_message_event(propagation_source: &libp2p::PeerId, message: &gossipsub::Message) {
    let sender = propagation_source.to_string();
    let sender_short = &sender[..8.min(sender.len())];
    let msg_str = String::from_utf8_lossy(&message.data);
    if let Ok(bcast) = serde_json::from_str::<BroadcastMessage>(&msg_str) {
        p2p_app::logging::p2plog_info(format!("[{}] {}", sender_short, bcast.content));
    } else {
        p2p_app::logging::p2plog_info(format!("[{}] {}", sender_short, msg_str));
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    p2p_app::logging::init_logging();
    p2plog_info("Starting P2P Chat CLI".to_string());

    let db_url = p2p_app::get_database_url();
    p2plog_info(format!("Using database: {}", db_url));

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
        let event = tokio::select! {
            swarm_event = swarm.select_next_some() => Some(Event::Swarm(Box::new(swarm_event))),
            line = lines.next_line() => Some(Event::Stdin(line.ok().flatten())),
        };

        match event {
            Some(Event::Swarm(event)) => {
                match *event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        handle_listen_addr_event(&address);
                    }
                    libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        p2p_app::logging::p2plog_info(format!("Connected to: {} (peers: 1)", peer_id));
                    }
                    libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        p2p_app::logging::p2plog_info(format!("Disconnected from: {}", peer_id));
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(p2p_app::behavior::AppBehaviourEvent::Gossipsub(
                        libp2p::gossipsub::Event::Message { propagation_source, message, .. }
                    )) => {
                        handle_message_event(&propagation_source, &message);
                    }
                    _ => {}
                }
            }
            Some(Event::Stdin(Some(text))) if !text.is_empty() => {
                let msg = BroadcastMessage {
                    content: text,
                    sent_at: Some(current_timestamp()),
                    nickname: None,
                    msg_id: None,
                };
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = swarm.behaviour_mut().gossipsub.publish(topic.clone(), json.as_bytes());
                }
            }
            Some(Event::Stdin(_)) => {}
            None => break,
        }
    }
    Ok(())
}
