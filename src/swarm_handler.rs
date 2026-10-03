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

            // Route per-group control/chat topics (group:<group_id> and the
            // derived group:<group_id>:backfill) before the broadcast path.
            // Backfill goes first: `group:X:backfill` also starts with the
            // group prefix, so the more specific check must win.
            if crate::groups::is_backfill_topic(&message.topic.to_string()) {
                handle_backfill_request(&message, peer_id, swarm);
                return;
            }
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
        Libp2pSwarmEvent::Behaviour(AppEv::DeviceBinding(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            handle_device_binding(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::CallSignaling(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            handle_call_signal(message, peer, event_tx, swarm).await;
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

/// Answer a group backfill request (backlog §11.5) by re-publishing our stored
/// messages newer than the requester's watermark on the main group topic.
///
/// Only members that know the group *and* hold history newer than the
/// requester's `after` respond — they are the ones that were online across
/// part of the gap. Answers flow through the normal group-message path, so the
/// requester's `(group_id, msg_id)` unique index (plus the `nil:` synthetic
/// key for id-less messages) absorbs every overlap and the user sees the gap
/// fill exactly once. Bounded to [`crate::types::MAX_MESSAGE_HISTORY`].
fn handle_backfill_request(
    message: &gossipsub::Message,
    peer_id: libp2p::PeerId,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let topic = message.topic.to_string();
    let Some(group_id) = crate::groups::group_id_from_backfill_topic(&topic) else {
        p2plog_debug(format!("Ignoring malformed backfill topic: {topic:?}"));
        return;
    };
    let Ok(request) = serde_json::from_slice::<crate::GroupBackfillRequest>(&message.data) else {
        p2plog_debug(format!("Ignoring malformed backfill request on {topic:?}"));
        return;
    };
    if request.group_id != group_id {
        p2plog_debug(format!(
            "Ignoring backfill request for {} on {topic:?}",
            request.group_id
        ));
        return;
    }
    // Never revive a deleted/unknown group, and never answer when our history
    // is no newer than the requester's watermark (nothing to contribute).
    if crate::groups::find_group(group_id).ok().flatten().is_none() {
        return;
    }
    let Ok(Some(latest)) = crate::groups::latest_group_sent_at(group_id) else {
        return;
    };
    if latest <= request.after {
        return;
    }

    // Public groups discover membership from traffic; recording is idempotent.
    if let Err(e) = crate::groups::record_group_member(group_id, &peer_id.to_string()) {
        p2plog_debug(format!("Failed to record group member: {e:?}"));
    }

    let Ok(history) = crate::groups::load_group_messages_after(
        group_id,
        request.after,
        crate::types::MAX_MESSAGE_HISTORY,
    ) else {
        return;
    };
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(group_id));
    for row in history {
        let msg = crate::GroupMessage {
            group_id: row.group_id,
            content: row.content,
            sent_at: row.sent_at,
            nickname: row.sender_nickname,
            msg_id: row.msg_id,
        };
        let Ok(json) = serde_json::to_string(&msg) else {
            continue;
        };
        if let Err(e) = swarm
            .behaviour_mut()
            .gossipsub
            .publish(topic.clone(), json.as_bytes())
        {
            p2plog_debug(format!("Failed to publish backfill answer: {e:?}"));
            break;
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

/// Verify an incoming claim about **our** device and, only on success, bind it.
///
/// Enforcement order (hostile-device invariant held end-to-end, not just at
/// `save_device_binding`):
///
/// 1. The claim must reference a user: `user_id`, `account_pubkey`, `claim_id`,
///    `signature`.
/// 2. `save_user` is (re)applied so the root exists for verification. The
///    upsert refreshes only the display-name seams; the stored `account_pubkey`
///    is the root and never changes.
/// 3. The claim must be about **us**: `request.peer_id` must equal the local
///    peer ID and `request.device_pubkey` the local install key's pubkey.
/// 4. `save_device_binding` re-verifies `signature` against the account pubkey
///    stored on *that user's* row. A claim signed by any other key is rejected
///    before anything is persisted.
///
/// A retransmit of an identical claim (`claim_id`) that already bound this
/// device is treated as accepted.
fn verify_and_bind_claim(
    request: &crate::behavior::DeviceBindingRequest,
    swarm: &Swarm<AppBehaviour>,
) -> color_eyre::Result<String> {
    let user_id = request
        .user_id
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no user_id"))?;
    let account_pubkey = request
        .account_pubkey
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no account_pubkey"))?;
    let claim_id = request
        .claim_id
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no claim_id"))?;
    let signature = request
        .signature
        .clone()
        .ok_or_else(|| color_eyre::eyre::eyre!("claim carries no signature"))?;

    crate::identity::save_user(
        &user_id,
        &account_pubkey,
        request.self_username.as_deref(),
        request.self_nickname.as_deref(),
    )?;

    if request.peer_id != swarm.local_peer_id().to_string() {
        return Err(color_eyre::eyre::eyre!(
            "claim names a different device than this node"
        ));
    }
    let local_pubkey = crate::get_libp2p_identity()?.public().encode_protobuf();
    if request.device_pubkey != local_pubkey {
        return Err(color_eyre::eyre::eyre!(
            "claim names a different device key than this install's"
        ));
    }

    match crate::identity::save_device_binding(
        &user_id,
        &request.peer_id,
        &request.device_pubkey,
        &claim_id,
        &signature,
    ) {
        Ok(_) => Ok(user_id),
        Err(_)
            if crate::identity::find_user_id_for_peer(&request.peer_id)
                .unwrap_or(None)
                .as_deref()
                == Some(user_id.as_str()) =>
        {
            // Identical claim already bound this device into the same user.
            Ok(user_id)
        }
        Err(e) => Err(e),
    }
}

async fn handle_device_binding(
    message: libp2p::request_response::Message<
        crate::behavior::DeviceBindingRequest,
        crate::behavior::DeviceBindingAck,
    >,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            let response = match request.kind.as_str() {
                "intro" => {
                    // Advertising device, optionally carrying its account. We
                    // never auto-mint a claim here: binding a stranger's device
                    // into our user would route DMs addressed to us to them.
                    // Surfacing the intro lets the operator decide explicitly.
                    let _ = event_tx
                        .send(SwarmEvent::DeviceBindingIntro {
                            peer_id: peer_id_str,
                            device_pubkey: request.device_pubkey.clone(),
                        })
                        .await;
                    crate::behavior::DeviceBindingAck {
                        ok: true,
                        reason: None,
                    }
                }
                "claim" => match verify_and_bind_claim(&request, swarm) {
                    Ok(user_id) => {
                        let _ = event_tx
                            .send(SwarmEvent::DeviceBindingReceived { user_id })
                            .await;
                        crate::behavior::DeviceBindingAck {
                            ok: true,
                            reason: None,
                        }
                    }
                    Err(e) => crate::behavior::DeviceBindingAck {
                        ok: false,
                        reason: Some(format!("{e}")),
                    },
                },
                other => crate::behavior::DeviceBindingAck {
                    ok: false,
                    reason: Some(format!("unknown device-binding kind: {other}")),
                },
            };
            let _ = swarm
                .behaviour_mut()
                .device_binding
                .send_response(channel, response);
        }
        libp2p::request_response::Message::Response { response, .. } => {
            if let Some(reason) = response.reason {
                if response.ok {
                    p2plog_debug(format!(
                        "device-binding request accepted by {peer_id_str}: {reason}"
                    ));
                } else {
                    p2plog_debug(format!(
                        "device-binding request rejected by {peer_id_str}: {reason}"
                    ));
                }
            }
        }
    }
}

/// Handle a call-signaling exchange (calls lane).
///
/// Requests are surfaced as [`SwarmEvent`]s (invite → ringing prompt on the
/// callee; accept/reject/end → session transitions on the caller) and acked
/// with a [`CallSignalAck`]. Responses are the ack to one of our own signals —
/// the receiving UI already advanced its session from the matching event, so
/// the ack is informational (mostly the failure reason).
async fn handle_call_signal(
    message: libp2p::request_response::Message<
        crate::behavior::CallSignal,
        crate::behavior::CallSignalAck,
    >,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            let (event, ack) = match request.kind.as_str() {
                "invite" => (
                    Some(SwarmEvent::CallInvite {
                        call_id: request.call_id.clone(),
                        peer_id: peer_id_str,
                    }),
                    crate::behavior::CallSignalAck {
                        ok: true,
                        reason: None,
                    },
                ),
                "accept" => (
                    Some(SwarmEvent::CallAccepted {
                        call_id: request.call_id.clone(),
                        peer_id: peer_id_str,
                    }),
                    crate::behavior::CallSignalAck {
                        ok: true,
                        reason: None,
                    },
                ),
                "reject" => (
                    Some(SwarmEvent::CallRejected {
                        call_id: request.call_id.clone(),
                        peer_id: peer_id_str,
                    }),
                    crate::behavior::CallSignalAck {
                        ok: true,
                        reason: None,
                    },
                ),
                "end" => (
                    Some(SwarmEvent::CallEnded {
                        call_id: request.call_id.clone(),
                        peer_id: peer_id_str,
                    }),
                    crate::behavior::CallSignalAck {
                        ok: true,
                        reason: None,
                    },
                ),
                other => (
                    None,
                    crate::behavior::CallSignalAck {
                        ok: false,
                        reason: Some(format!("unknown call-signal kind: {other}")),
                    },
                ),
            };
            if let Some(ev) = event {
                let _ = event_tx.send(ev).await;
            }
            let _ = swarm
                .behaviour_mut()
                .call_signaling
                .send_response(channel, ack);
        }
        libp2p::request_response::Message::Response { response, .. } => {
            if let Some(reason) = response.reason {
                if response.ok {
                    p2plog_debug(format!("call signal accepted by {peer_id_str}: {reason}"));
                } else {
                    p2plog_debug(format!("call signal rejected by {peer_id_str}: {reason}"));
                }
            }
        }
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
    match cmd {
        SwarmCommand::Publish {
            content,
            nickname,
            msg_id,
        } => publish_broadcast(content, nickname, msg_id, swarm, topic),
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
        } => send_dm(&peer_id, content, nickname, msg_id, ack_for, swarm),
        SwarmCommand::PublishGroup {
            group_id,
            content,
            nickname,
            msg_id,
        } => publish_group(content, nickname, msg_id, group_id, swarm),
        SwarmCommand::SubscribeGroup { group_id } => subscribe_to_group(&group_id, swarm),
        SwarmCommand::SendDeviceBinding { peer_id, request } => {
            send_device_binding(&peer_id, request, swarm);
        }
        SwarmCommand::SendCallSignal { peer_id, signal } => {
            send_call_signal(&peer_id, *signal, swarm);
        }
        SwarmCommand::UnsubscribeGroup { group_id } => unsubscribe_from_group(&group_id, swarm),
    }
}

fn publish_broadcast(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &str,
) {
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

fn send_dm(
    peer_id: &str,
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    ack_for: Option<String>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
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

fn publish_group(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    group_id: String,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
    let msg = build_group_message(group_id, content, nickname, msg_id);
    if let Ok(json) = serde_json::to_string(&msg) {
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

fn subscribe_to_group(group_id: &str, swarm: &mut Swarm<AppBehaviour>) {
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(group_id));
    match swarm.behaviour_mut().gossipsub.subscribe(&topic) {
        Ok(true) => p2plog_debug(format!("Subscribed to group {group_id}")),
        Ok(false) => p2plog_debug(format!("Already subscribed to group {group_id}")),
        Err(e) => p2plog_error(format!("Failed to subscribe to group {group_id}: {e:?}")),
    }
    // Backfill (backlog §11.5): join the group's backfill topic and request
    // everything newer than our newest stored message. Members online across
    // the gap answer by re-publishing that history on the main topic; dedup
    // absorbs overlaps. On (re)subscribe of an already-current group the
    // watermark matches our latest, so nothing is answered.
    let request = crate::GroupBackfillRequest {
        group_id: group_id.to_string(),
        after: crate::groups::latest_group_sent_at(group_id)
            .ok()
            .flatten()
            .unwrap_or(0.0),
    };
    let backfill_topic = gossipsub::IdentTopic::new(crate::groups::group_backfill_topic(group_id));
    if let Err(e) = swarm.behaviour_mut().gossipsub.subscribe(&backfill_topic) {
        p2plog_error(format!(
            "Failed to subscribe to backfill topic for {group_id}: {e:?}"
        ));
    }
    if let Ok(json) = serde_json::to_string(&request) {
        let _ = swarm
            .behaviour_mut()
            .gossipsub
            .publish(backfill_topic, json.as_bytes());
    }
}

fn send_device_binding(
    peer_id: &str,
    request: Box<crate::behavior::DeviceBindingRequest>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
        // `send_request` returns the outbound request ID; failures surface as
        // `OutboundFailure` events on the behaviour, so a successful dispatch
        // is all we can assert here.
        let _ = swarm
            .behaviour_mut()
            .device_binding
            .send_request(&peer, *request);
    }
}

fn send_call_signal(
    peer_id: &str,
    signal: crate::behavior::CallSignal,
    swarm: &mut Swarm<AppBehaviour>,
) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
        // See `send_device_binding`: dispatch is fire-and-forget from here;
        // protocol failures surface as `OutboundFailure` behaviour events.
        let _ = swarm
            .behaviour_mut()
            .call_signaling
            .send_request(&peer, signal);
    }
}

fn unsubscribe_from_group(group_id: &str, swarm: &mut Swarm<AppBehaviour>) {
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(group_id));
    if swarm.behaviour_mut().gossipsub.unsubscribe(&topic) {
        p2plog_debug(format!("Unsubscribed from group {group_id}"));
    } else {
        p2plog_debug(format!("Was not subscribed to group {group_id}"));
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
