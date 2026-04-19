use libp2p::Swarm;
use libp2p::gossipsub;
#[cfg(feature = "mdns")]
use libp2p::mdns;
use p2p_app::AppBehaviour;
use p2p_app::AppBehaviourEvent as AppEv;
use p2p_app::logging::{p2plog_error, p2plog_info};
use p2p_app::{
    load_direct_messages, load_listen_ports, save_listen_ports, save_peer, save_peer_session,
};
use std::time::Duration;

pub async fn run_headless_mode(mut swarm: Swarm<AppBehaviour>) -> color_eyre::Result<()> {
    p2p_app::logging::init_logging();
    p2plog_info("Starting non-interactive mode".to_string());
    p2plog_info(format!("Our peer ID: {}", swarm.local_peer_id()));
    p2plog_info("Press Ctrl+C to exit".to_string());

    let mut concurrent_peers: usize = 0;

    loop {
        tokio::select! {
            event = swarm.select_next_some() => {
                match event {
                    SwarmEvent::NewListenAddr { address, .. } => {
                        p2plog_info(format!("Listening on: {}", address));
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Tcp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let _ = save_listen_ports(Some(port), None);
                        }
                        #[cfg(feature = "quic")]
                        if let Some(port) = address
                            .iter()
                            .find_map(|p| match p {
                                libp2p::multiaddr::Protocol::Udp(port) => Some(port as i32),
                                _ => None,
                            })
                        {
                            let (tcp, _) = load_listen_ports().unwrap_or((None, None));
                            let _ = save_listen_ports(tcp, Some(port));
                        }
                    }
                    SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                        p2plog_info(format!("Dial failed: peer={:?}, error={}", peer_id, error));
                    }
                    SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                        concurrent_peers += 1;
                        p2plog_info(format!("Connected to: {} (peers: {})", peer_id, concurrent_peers));
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                    SwarmEvent::ConnectionClosed { peer_id, .. } => {
                        concurrent_peers = concurrent_peers.saturating_sub(1);
                        p2plog_info(format!("Disconnected from: {} (peers: {})", peer_id, concurrent_peers));
                        if let Err(e) = save_peer_session(concurrent_peers as i32) {
                            p2plog_error(format!("Failed to save peer session: {}", e));
                        }
                    }
                    SwarmEvent::Dialing { peer_id, .. } => {
                        p2plog_info(format!("Dialing: {:?}", peer_id));
                    }
                    SwarmEvent::Behaviour(AppEv::Gossipsub(
                        gossipsub::Event::Message { propagation_source, message, .. }
                    )) => {
                        let ps = propagation_source.to_string();
                        let suffix = &ps[ps.len().saturating_sub(8)..];
                        p2plog_info(format!(
                            "[{}] {}",
                            suffix,
                            String::from_utf8_lossy(&message.data)
                        ));
                    }
                    #[cfg(feature = "mdns")]
                    SwarmEvent::Behaviour(AppEv::Mdns(mdns::Event::Discovered(list))) => {
                        for (peer_id, multiaddr) in list {
                            p2plog_info(format!("mDNS discovered: {} at {}", peer_id, multiaddr));
                            let addresses = vec![multiaddr.to_string()];
                            if let Err(e) = save_peer(&peer_id.to_string(), &addresses) {
                                p2plog_error(format!("Failed to save peer: {}", e));
                            }
                            let _ = swarm.dial(multiaddr.clone());
                            swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        }
                    }
                    _ => {}
                }
            }
            _ = tokio::time::sleep(Duration::from_secs(3600)) => {}
        }
    }
}
