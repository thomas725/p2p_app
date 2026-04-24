use crate::types::SwarmEvent;
use crate::{AppBehaviour, behavior::AppBehaviourEvent as AppEv};
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Spawns the swarm handler task that processes libp2p events
/// and translates them to app-level SwarmEvent messages.
pub fn spawn_swarm_handler(
    mut swarm: Swarm<AppBehaviour>,
) -> (tokio::task::JoinHandle<()>, mpsc::Receiver<SwarmEvent>) {
    let (tx, rx) = mpsc::channel(100);

    let handle = tokio::spawn(async move {
        loop {
            match swarm.select_next_some().await {
                Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(gossipsub::Event::Message {
                    propagation_source: peer_id,
                    message,
                    ..
                })) => {
                    let peer_id_str = peer_id.to_string();
                    let raw = String::from_utf8_lossy(&message.data).to_string();

                    // Try to parse as BroadcastMessage
                    if let Ok(bcast) = serde_json::from_str::<crate::BroadcastMessage>(&raw) {
                        let content = bcast.content.clone();
                        let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

                        let _ = tx
                            .send(SwarmEvent::BroadcastMessage {
                                content,
                                peer_id: peer_id_str.clone(),
                                latency,
                            })
                            .await;
                    }
                }
                Libp2pSwarmEvent::Behaviour(AppEv::RequestResponse(
                    libp2p::request_response::Event::Message {
                        peer,
                        message:
                            libp2p::request_response::Message::Request {
                                request, channel, ..
                            },
                        connection_id: _,
                    },
                )) => {
                    let peer_id_str = peer.to_string();
                    let content = request.content.clone();
                    let now = SystemTime::now();
                    let latency = Some(crate::format_latency(request.sent_at, now));

                    let _ = tx
                        .send(SwarmEvent::DirectMessage {
                            content,
                            peer_id: peer_id_str.clone(),
                            latency,
                        })
                        .await;

                    // Send ACK response
                    let response = crate::DirectMessage {
                        content: "ok".to_string(),
                        timestamp: chrono::Utc::now().timestamp(),
                        sent_at: Some(
                            SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64(),
                        ),
                        nickname: None,
                    };
                    let _ = swarm
                        .behaviour_mut()
                        .request_response
                        .send_response(channel, response);
                }
                #[cfg(feature = "mdns")]
                Libp2pSwarmEvent::Behaviour(AppEv::Mdns(libp2p::mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        let _ = tx
                            .send(SwarmEvent::PeerDiscovered {
                                peer_id: peer_id.to_string(),
                                addresses: vec![multiaddr.clone()],
                            })
                            .await;
                        swarm.dial(multiaddr.clone()).ok();
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    let _ = tx
                        .send(SwarmEvent::PeerConnected(peer_id.to_string()))
                        .await;
                }
                Libp2pSwarmEvent::ConnectionClosed { peer_id, .. } => {
                    let _ = tx
                        .send(SwarmEvent::PeerDisconnected(peer_id.to_string()))
                        .await;
                }
                Libp2pSwarmEvent::NewListenAddr { address, .. } => {
                    let _ = tx
                        .send(SwarmEvent::ListenAddrEstablished(address.to_string()))
                        .await;
                }
                _ => {}
            }
        }
    });

    (handle, rx)
}
