use crate::types::{SwarmCommand, SwarmEvent};
use crate::{
    AppBehaviour, BroadcastMessage, behavior::AppBehaviourEvent as AppEv, p2plog_debug,
    p2plog_error,
};
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use std::time::SystemTime;
use tokio::sync::mpsc;

enum Event {
    Swarm(Box<Libp2pSwarmEvent<AppEv>>),
    Command(SwarmCommand),
}

async fn handle_swarm_event(
    swarm_event: Libp2pSwarmEvent<AppEv>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match swarm_event {
        Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(gossipsub::Event::Message {
            propagation_source: peer_id,
            message,
            ..
        })) => {
            let peer_id_str = peer_id.to_string();

            if let Ok(bcast) = serde_json::from_slice::<BroadcastMessage>(&message.data) {
                let content = bcast.content.clone();
                let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

                let _ = event_tx
                    .send(SwarmEvent::BroadcastMessage {
                        content,
                        peer_id: peer_id_str,
                        latency,
                        nickname: bcast.nickname.clone(),
                    })
                    .await;
            } else {
                p2plog_debug(format!("Failed to parse broadcast message from peer {}", peer_id_str));
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
            let latency = Some(crate::format_latency(request.sent_at, SystemTime::now()));
            let nickname = request.nickname.clone();

            let _ = event_tx
                .send(SwarmEvent::DirectMessage {
                    content,
                    peer_id: peer_id_str,
                    latency,
                    nickname,
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

fn current_timestamp() -> f64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn handle_command(cmd: SwarmCommand, swarm: &mut Swarm<AppBehaviour>, topic: &str) {
    match cmd {
        SwarmCommand::Publish { content, nickname } => {
            let msg = BroadcastMessage {
                content,
                sent_at: Some(current_timestamp()),
                nickname,
            };
            if let Ok(json) = serde_json::to_string(&msg) {
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(gossipsub::IdentTopic::new(topic), json.as_bytes())
                {
                    Ok(gossipsub::MessageId(id)) => {
                        p2plog_debug(format!("Published broadcast: {}", String::from_utf8_lossy(&id)));
                    }
                    Err(e) => {
                        p2plog_error(format!("Failed to publish: {:?}", e));
                    }
                }
            }
        }
        SwarmCommand::SendDm { peer_id, content, nickname } => {
            use libp2p::PeerId;
            if let Ok(peer) = peer_id.parse::<PeerId>() {
                let msg = crate::DirectMessage {
                    content,
                    timestamp: chrono::Utc::now().timestamp(),
                    sent_at: Some(current_timestamp()),
                    nickname,
                };
                swarm.behaviour_mut().request_response.send_request(&peer, msg);
            }
        }
    }
}

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
            let event = tokio::select! {
                swarm_event = swarm.select_next_some() => Some(Event::Swarm(Box::new(swarm_event))),
                Some(cmd) = cmd_rx.recv() => Some(Event::Command(cmd)),
                else => None,
            };

            match event {
                Some(Event::Swarm(swarm_event)) => {
                    handle_swarm_event(*swarm_event, &event_tx, &mut swarm).await;
                }
                Some(Event::Command(cmd)) => {
                    handle_command(cmd, &mut swarm, &topic);
                }
                None => break,
            }
        }
    });

    (handle, event_rx, cmd_tx)
}
