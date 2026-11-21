//! Headless CLI binary — minimal footprint, reads from stdin, writes to stderr.

use libp2p::{futures::StreamExt, gossipsub};
use p2p_app::current_timestamp;
use p2p_app::logging::p2plog_info;
use p2p_app::{BroadcastMessage, build_swarm, get_network_size};
use tokio::io::{AsyncBufReadExt as _, BufReader};

enum Event {
    Swarm(Box<libp2p::swarm::SwarmEvent<p2p_app::behavior::AppBehaviourEvent>>),
    Stdin(Option<String>),
}

fn extract_tcp_port(address: &libp2p::Multiaddr) -> Option<i32> {
    address.iter().find_map(|p| match p {
        libp2p::multiaddr::Protocol::Tcp(port) => Some(i32::from(port)),
        _ => None,
    })
}

#[cfg(feature = "quic")]
fn extract_udp_port(address: &libp2p::Multiaddr) -> Option<i32> {
    address.iter().find_map(|p| match p {
        libp2p::multiaddr::Protocol::Udp(port) => Some(i32::from(port)),
        _ => None,
    })
}

fn handle_listen_addr_event(address: &libp2p::Multiaddr) {
    p2p_app::logging::p2plog_info(format!("Listening on: {address}"));
    if let Some(port) = extract_tcp_port(address) {
        let _ = p2p_app::save_listen_ports(Some(port), None);
    }
    #[cfg(feature = "quic")]
    if let Some(port) = extract_udp_port(address) {
        let (tcp, _) = p2p_app::load_listen_ports().unwrap_or((None, None));
        let _ = p2p_app::save_listen_ports(tcp, Some(port));
    }
}

/// Format one gossipsub message for the CLI log.
///
/// Gossipsub signs every message (`MessageAuthenticity::Signed`), so `source` is
/// the verified publisher while `propagation_source` is only the last hop. Use
/// the canonical helper so a relayed message is attributed to the peer that
/// actually authored it, matching every other frontend.
fn format_message_event(
    propagation_source: &libp2p::PeerId,
    message: &gossipsub::Message,
) -> String {
    let author = p2p_app::gossipsub_author(message.source, *propagation_source);
    let sender = author.to_string();
    let sender_short = sender.get(..8.min(sender.len())).unwrap_or(&sender);
    let msg_str = String::from_utf8_lossy(&message.data);
    if let Ok(bcast) = serde_json::from_str::<BroadcastMessage>(&msg_str) {
        format!("[{sender_short}] {}", bcast.content)
    } else {
        format!("[{sender_short}] {msg_str}")
    }
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    p2p_app::logging::init_logging();
    p2plog_info("Starting P2P Chat CLI".to_string());

    // Initialize database once at startup (logs database path and peer ID)
    let _db = p2p_app::init_database()?;

    let network_size = match get_network_size() {
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

    let topic = gossipsub::IdentTopic::new(p2p_app::CHAT_TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();

    loop {
        let event = tokio::select! {
            swarm_event = swarm.select_next_some() => Some(Event::Swarm(Box::new(swarm_event))),
            line = lines.next_line() => Some(Event::Stdin(line.ok().flatten())),
        };

        match event {
            Some(Event::Swarm(event)) => match *event {
                libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                    handle_listen_addr_event(&address);
                }
                libp2p::swarm::SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Connected to: {peer_id} (peers: 1)"));
                }
                libp2p::swarm::SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    p2p_app::logging::p2plog_info(format!("Disconnected from: {peer_id}"));
                }
                libp2p::swarm::SwarmEvent::Behaviour(
                    p2p_app::behavior::AppBehaviourEvent::Gossipsub(
                        libp2p::gossipsub::Event::Message {
                            propagation_source,
                            message,
                            ..
                        },
                    ),
                ) => {
                    p2plog_info(format_message_event(&propagation_source, &message));
                }
                _ => {}
            },
            Some(Event::Stdin(Some(text))) if !text.is_empty() => {
                let msg = BroadcastMessage {
                    content: text,
                    sent_at: Some(current_timestamp()),
                    nickname: None,
                    msg_id: None,
                    attachment: None,
                    reply_to: None,
                };
                if let Ok(json) = serde_json::to_string(&msg) {
                    let _ = swarm
                        .behaviour_mut()
                        .gossipsub
                        .publish(topic.clone(), json.as_bytes());
                }
            }
            Some(Event::Stdin(_)) => {}
            None => break,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_message(source: Option<libp2p::PeerId>) -> gossipsub::Message {
        gossipsub::Message {
            source,
            data: b"hello".to_vec(),
            sequence_number: None,
            topic: gossipsub::IdentTopic::new("test").hash(),
        }
    }

    fn short_id(peer: libp2p::PeerId) -> String {
        let full = peer.to_string();
        full.get(..8.min(full.len())).unwrap_or(&full).to_string()
    }

    #[test]
    fn relayed_message_is_attributed_to_the_signed_publisher() {
        let publisher = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let relay = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let line = format_message_event(&relay, &test_message(Some(publisher)));
        assert!(
            line.starts_with(&format!("[{}]", short_id(publisher))),
            "a relayed message must be shown under its publisher, not the relay"
        );
    }

    #[test]
    fn unsigned_message_falls_back_to_the_relay() {
        let relay = libp2p::identity::Keypair::generate_ed25519()
            .public()
            .to_peer_id();
        let line = format_message_event(&relay, &test_message(None));
        assert!(
            line.starts_with(&format!("[{}]", short_id(relay))),
            "an unsigned local message has no publisher, so the relay is the best label"
        );
    }
}
