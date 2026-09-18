//! Swarm event handler that translates libp2p events to application events

use crate::types::{SwarmCommand, SwarmEvent};
use crate::{
    AppBehaviour, BroadcastMessage, behavior::AppBehaviourEvent as AppEv, current_timestamp,
    p2plog_debug, p2plog_error,
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

            // Route per-group topics (group:<group_id>) to the group-chat path
            // before the broadcast path below.
            if crate::groups::is_group_topic(&message.topic.to_string()) {
                handle_group_message(message, peer_id, event_tx).await;
                return;
            }

            if let Ok(bcast) = serde_json::from_slice::<BroadcastMessage>(&message.data) {
                let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

                let _ = event_tx
                    .send(SwarmEvent::BroadcastMessage(crate::MessageEvent {
                        content: bcast.content,
                        peer_id: peer_id_str,
                        latency,
                        nickname: bcast.nickname.clone(),
                        msg_id: bcast.msg_id.clone(),
                    }))
                    .await;

                // Best-effort receipt confirmation for broadcasts:
                // send a receipt-only DM back to the propagation source.
                if let Some(ack_for) = bcast.msg_id.clone() {
                    let receipt = make_ack_dm(String::new(), Some(ack_for));
                    swarm
                        .behaviour_mut()
                        .request_response
                        .send_request(&peer_id, receipt);
                }
            } else {
                p2plog_debug(format!(
                    "Failed to parse broadcast message from peer {peer_id_str}"
                ));
            }
        }
        Libp2pSwarmEvent::Behaviour(AppEv::RequestResponse(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            handle_request_response(message, peer, event_tx, swarm).await;
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
        #[cfg(feature = "mdns")]
        Libp2pSwarmEvent::Behaviour(AppEv::Mdns(libp2p::mdns::Event::Expired(list))) => {
            for (peer_id, _multiaddr) in list {
                let _ = event_tx
                    .send(SwarmEvent::PeerExpired {
                        peer_id: peer_id.to_string(),
                    })
                    .await;
            }
        }
        // Ping results drive connection liveness; the resulting close is
        // handled by the ConnectionClosed arm below, so nothing to do here.
        Libp2pSwarmEvent::Behaviour(AppEv::Ping(_)) => {}
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

async fn handle_group_message(
    message: gossipsub::Message,
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    let peer_id_str = peer_id.to_string();
    let topic = message.topic.to_string();
    let Some(group_id) = crate::groups::group_id_from_topic(&topic) else {
        p2plog_debug(format!("Ignoring malformed group topic: {topic:?}"));
        return;
    };

    match serde_json::from_slice::<crate::GroupMessage>(&message.data) {
        Ok(group_msg) => {
            // Public groups discover membership from traffic; recording is
            // idempotent per (group_id, peer_id).
            if let Err(e) = crate::groups::record_group_member(group_id, &peer_id_str) {
                p2plog_debug(format!("Failed to record group member: {e:?}"));
            }
            match crate::groups::save_incoming_group_message(
                group_id,
                &peer_id_str,
                &group_msg.content,
                crate::groups::GroupMessageMeta {
                    sender_nickname: group_msg.nickname.clone(),
                    msg_id: group_msg.msg_id.clone(),
                    sent_at: group_msg.sent_at,
                },
            ) {
                Ok(Some(_)) => {}
                Ok(None) => p2plog_debug(format!(
                    "Dropped duplicate group message {} in {group_id}",
                    group_msg.msg_id.clone().unwrap_or_default()
                )),
                Err(e) => p2plog_debug(format!("Failed to save group message: {e:?}")),
            }
            let _ = event_tx
                .send(SwarmEvent::GroupMessage(crate::GroupMessageEvent {
                    group_id: group_id.to_string(),
                    content: group_msg.content,
                    peer_id: peer_id_str,
                    nickname: group_msg.nickname,
                    msg_id: group_msg.msg_id,
                }))
                .await;
        }
        Err(e) => {
            p2plog_debug(format!(
                "Failed to parse group message from peer {peer_id_str} on {topic:?}: {e}"
            ));
        }
    }
}

async fn handle_request_response(
    message: libp2p::request_response::Message<crate::DirectMessage, crate::DirectMessage>,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            if request.content.trim().is_empty() {
                if let Some(ack_for) = &request.ack_for {
                    let _ = event_tx
                        .send(SwarmEvent::Receipt {
                            peer_id: peer_id_str.clone(),
                            ack_for: ack_for.clone(),
                            received_at: request.received_at,
                        })
                        .await;
                } else if request.nickname.is_some() {
                    // Nickname-only DM: the empty-content exchange we send on
                    // connect (and that the peer echoes). Deliver it as an empty
                    // `DirectMessage` so the receiving frontend records the
                    // announced nickname (and touches the peer's last-seen)
                    // without persisting a chat message.
                    let _ = event_tx
                        .send(SwarmEvent::DirectMessage(crate::MessageEvent {
                            content: String::new(),
                            peer_id: peer_id_str,
                            latency: Some(crate::format_latency(
                                request.sent_at,
                                SystemTime::now(),
                            )),
                            nickname: request.nickname,
                            msg_id: None,
                        }))
                        .await;
                } else {
                    p2plog_debug("Dropped empty DM with no ack_for or nickname".to_string());
                }
            } else {
                let msg_id = request.msg_id.clone();
                let latency = Some(crate::format_latency(request.sent_at, SystemTime::now()));
                if let Some(group_id) = request.group_id.clone() {
                    let local_peer = swarm.local_peer_id().to_string();
                    if crate::groups::is_group_member(&group_id, &local_peer).unwrap_or(false) {
                        let _ = event_tx
                            .send(SwarmEvent::GroupMessage(crate::GroupMessageEvent {
                                group_id,
                                content: request.content,
                                peer_id: peer_id_str,
                                nickname: request.nickname,
                                msg_id,
                            }))
                            .await;
                    } else {
                        let inviter_peer = peer_id_str;
                        let _ = event_tx
                            .send(SwarmEvent::GroupInvite {
                                group_id,
                                inviter_peer,
                            })
                            .await;
                    }
                } else {
                    let _ = event_tx
                        .send(SwarmEvent::DirectMessage(crate::MessageEvent {
                            content: request.content,
                            peer_id: peer_id_str,
                            latency,
                            nickname: request.nickname,
                            msg_id,
                        }))
                        .await;
                }
            }

            let response = make_ack_dm("ok".to_string(), request.msg_id);
            let _ = swarm
                .behaviour_mut()
                .request_response
                .send_response(channel, response);
        }
        libp2p::request_response::Message::Response { response, .. } => {
            if let Some(ack_for) = response.ack_for {
                let _ = event_tx
                    .send(SwarmEvent::Receipt {
                        peer_id: peer_id_str,
                        ack_for,
                        received_at: response.received_at,
                    })
                    .await;
            }
        }
    }
}

fn make_ack_dm(content: String, ack_for: Option<String>) -> crate::DirectMessage {
    crate::DirectMessage {
        content,
        timestamp: chrono::Utc::now().timestamp(),
        sent_at: Some(current_timestamp()),
        nickname: None,
        msg_id: None,
        ack_for,
        received_at: Some(current_timestamp()),
        group_id: None,
    }
}

/// Build a `BroadcastMessage` from component parts
#[must_use]
pub fn build_broadcast_message(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
) -> BroadcastMessage {
    BroadcastMessage {
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
    }
}

/// Build a `GroupMessage` from component parts
#[must_use]
pub fn build_group_message(
    group_id: String,
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
) -> crate::GroupMessage {
    crate::GroupMessage {
        group_id,
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
    }
}

fn handle_command(cmd: SwarmCommand, swarm: &mut Swarm<AppBehaviour>, topic: &str) {
    use libp2p::PeerId;
    match cmd {
        SwarmCommand::Publish {
            content,
            nickname,
            msg_id,
        } => {
            let msg = build_broadcast_message(content, nickname, msg_id);
            if let Ok(json) = serde_json::to_string(&msg) {
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(gossipsub::IdentTopic::new(topic), json.as_bytes())
                {
                    Ok(gossipsub::MessageId(id)) => {
                        p2plog_debug(format!(
                            "Published broadcast: {}",
                            String::from_utf8_lossy(&id)
                        ));
                    }
                    Err(e) => {
                        p2plog_error(format!("Failed to publish: {e:?}"));
                    }
                }
            }
        }
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
        } => {
            if let Ok(peer) = peer_id.parse::<PeerId>() {
                let msg = crate::DirectMessage {
                    content,
                    timestamp: chrono::Utc::now().timestamp(),
                    sent_at: Some(current_timestamp()),
                    nickname,
                    msg_id,
                    ack_for,
                    received_at: None,
                    group_id: None,
                };
                swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, msg);
            }
        }
        SwarmCommand::PublishGroup {
            group_id,
            content,
            nickname,
            msg_id,
        } => {
            let msg = build_group_message(group_id.clone(), content, nickname, msg_id);
            if let Ok(json) = serde_json::to_string(&msg) {
                let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
                match swarm
                    .behaviour_mut()
                    .gossipsub
                    .publish(topic, json.as_bytes())
                {
                    Ok(gossipsub::MessageId(id)) => {
                        p2plog_debug(format!(
                            "Published group message: {}",
                            String::from_utf8_lossy(&id)
                        ));
                    }
                    Err(e) => {
                        p2plog_error(format!("Failed to publish group message: {e:?}"));
                    }
                }
            }
        }
        SwarmCommand::SubscribeGroup { group_id } => {
            let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
            match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
                Ok(true) => p2plog_debug(format!("Subscribed to group {group_id}")),
                Ok(false) => p2plog_debug(format!("Already subscribed to group {group_id}")),
                Err(e) => p2plog_error(format!("Failed to subscribe to group {group_id}: {e:?}")),
            }
        }
        SwarmCommand::UnsubscribeGroup { group_id } => {
            let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
            if swarm.behaviour_mut().gossipsub.unsubscribe(&topic) {
                p2plog_debug(format!("Unsubscribed from group {group_id}"));
            } else {
                p2plog_debug(format!("Was not subscribed to group {group_id}"));
            }
        }
    }
}

/// Spawns the swarm handler task that processes libp2p events
/// and translates them to app-level `SwarmEvent` messages.
///
/// The returned sender can be used to send `SwarmCommand` (Publish, `SendDm`).
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

#[cfg(test)]
#[path = "../tests/unit/unit_swarm_handler.rs"]
mod tests;
