use crate::types::{SwarmCommand, SwarmEvent};
use crate::{AppBehaviour, BroadcastMessage, behavior::AppBehaviourEvent as AppEv};
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Spawns the swarm handler task that processes libp2p events
/// and translates them to app-level SwarmEvent messages.
///
/// The returned sender can be used to send SwarmCommand (Publish, SendDm).
pub fn spawn_swarm_handler(
    mut swarm: Swarm<AppBehaviour>,
    topic: String,
) -> (
    tokio::task::JoinHandle<()>,
    mpsc::Receiver<SwarmEvent>,
    mpsc::Sender<SwarmCommand>,
) {
    let (event_tx, event_rx) = mpsc::channel(100);
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<SwarmCommand>(100);

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                // Process swarm events
                swarm_event = swarm.select_next_some() => {
                    match swarm_event {
                        Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(gossipsub::Event::Message {
                            propagation_source: peer_id,
                            message,
                            ..
                        })) => {
                            let peer_id_str = peer_id.to_string();
                            let raw = String::from_utf8_lossy(&message.data).to_string();

                            if let Ok(bcast) = serde_json::from_str::<BroadcastMessage>(&raw) {
                                let content = bcast.content.clone();
                                let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

                                let _ = event_tx
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

                            let _ = event_tx
                                .send(SwarmEvent::DirectMessage {
                                    content,
                                    peer_id: peer_id_str.clone(),
                                    latency,
                                })
                                .await;

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
                                let _ = event_tx
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
                            let _ = event_tx
                                .send(SwarmEvent::PeerConnected(peer_id.to_string()))
                                .await;
                        }
                        Libp2pSwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let _ = event_tx
                                .send(SwarmEvent::PeerDisconnected(peer_id.to_string()))
                                .await;
                        }
                        Libp2pSwarmEvent::NewListenAddr { address, .. } => {
                            let _ = event_tx
                                .send(SwarmEvent::ListenAddrEstablished(address.to_string()))
                                .await;
                        }
                        _ => {}
                    }
                }
                // Process commands from other tasks
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        SwarmCommand::Publish(content) => {
                            let msg = BroadcastMessage {
                                content,
                                sent_at: Some(
                                    SystemTime::now()
                                        .duration_since(SystemTime::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs_f64(),
                                ),
                                nickname: None,
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                match swarm
                                    .behaviour_mut()
                                    .gossipsub
                                    .publish(gossipsub::IdentTopic::new(&topic), json.as_bytes())
                                {
                                    Ok(gossipsub::MessageId(id)) => {
                                        eprintln!("[SWARM] Published message: {}", String::from_utf8_lossy(&id));
                                    }
                                    Err(e) => {
                                        eprintln!("[SWARM] Failed to publish: {:?}", e);
                                    }
                                }
                            }
                        }
                        SwarmCommand::SendDm { peer_id, content } => {
                            use libp2p::PeerId;
                            if let Ok(peer) = peer_id.parse::<PeerId>() {
                                let msg = crate::DirectMessage {
                                    content,
                                    timestamp: chrono::Utc::now().timestamp(),
                                    sent_at: Some(
                                        SystemTime::now()
                                            .duration_since(SystemTime::UNIX_EPOCH)
                                            .unwrap_or_default()
                                            .as_secs_f64(),
                                    ),
                                    nickname: None,
                                };
                                swarm.behaviour_mut().request_response.send_request(&peer, msg);
                            }
                        }
                    }
                }
            }
        }
    });

    (handle, event_rx, cmd_tx)
}
