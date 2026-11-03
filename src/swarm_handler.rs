//! Swarm event handler that translates libp2p events to application events

use crate::types::{StringMap, SwarmCommand, SwarmEvent};
use crate::{
    AppBehaviour, BroadcastMessage, behavior::AppBehaviourEvent as AppEv, current_timestamp,
    p2plog_debug, p2plog_error,
};
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::SystemTime;
use tokio::sync::mpsc;

enum Event {
    Swarm(Box<Libp2pSwarmEvent<AppEv>>),
    Command(SwarmCommand),
}

/// Dispatch a decoded broadcast: surface it as an event, pull its attachment
/// from the propagation source (files lane §12), and send a best-effort
/// receipt confirmation back.
async fn handle_broadcast_message(
    bcast: BroadcastMessage,
    peer_id: libp2p::PeerId,
    peer_id_str: String,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

    let _ = event_tx
        .send(SwarmEvent::BroadcastMessage(crate::MessageEvent {
            content: bcast.content,
            peer_id: peer_id_str,
            latency,
            nickname: bcast.nickname.clone(),
            msg_id: bcast.msg_id.clone(),
            attachment: bcast.attachment.clone(),
            encrypted: false,
        }))
        .await;

    // Attachments are pulled from the propagation source — the peer that
    // re-sent the message retains the blob on disk to serve the chunk
    // requests.
    if let Some(attachment) = bcast.attachment.as_ref() {
        maybe_start_fetch(&peer_id, attachment, swarm);
    }

    // Best-effort receipt confirmation for broadcasts: send a receipt-only
    // DM back to the propagation source.
    if let Some(ack_for) = bcast.msg_id.clone() {
        let receipt = make_ack_dm(String::new(), Some(ack_for));
        swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer_id, receipt);
    }
}

#[cfg(feature = "kad")]
fn handle_identify_received(
    peer_id: libp2p::PeerId,
    info: &libp2p::identify::Info,
    swarm: &mut Swarm<AppBehaviour>,
) {
    for addr in &info.listen_addrs {
        swarm
            .behaviour_mut()
            .kad
            .add_address(&peer_id, addr.clone());
    }
    let addrs: Vec<String> = info.listen_addrs.iter().map(ToString::to_string).collect();
    if let Err(e) = crate::save_peer(&peer_id.to_string(), &addrs) {
        p2plog_debug(format!("Failed to save peer on identify: {e}"));
    }
    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
}

/// Refresh the cached DHT routing-table size from Kademlia's k-buckets.
#[cfg(feature = "kad")]
fn refresh_dht_routing(swarm: &mut Swarm<AppBehaviour>) {
    let peers: usize = swarm
        .behaviour_mut()
        .kad
        .kbuckets()
        .map(|bucket| libp2p::kad::KBucketRef::num_entries(&bucket))
        .sum();
    crate::net_diag::record_dht_routing_peers(u32::try_from(peers).unwrap_or(u32::MAX));
}

/// Translate a Kademlia query result into the diagnostics snapshot.
#[cfg(feature = "kad")]
fn handle_kad_query(result: libp2p::kad::QueryResult) {
    match result {
        libp2p::kad::QueryResult::Bootstrap(Ok(_)) => crate::net_diag::record_dht_bootstrap(true),
        libp2p::kad::QueryResult::Bootstrap(Err(_)) => crate::net_diag::record_dht_bootstrap(false),
        libp2p::kad::QueryResult::PutRecord(Ok(_)) => {
            crate::net_diag::record_dht_announce_result(true, "Announced to the DHT");
        }
        libp2p::kad::QueryResult::PutRecord(Err(e)) => {
            crate::net_diag::record_dht_announce_result(false, &format!("Announce failed: {e}"));
        }
        _ => {}
    }
}

/// Publish our peer record to the DHT (rate-limited), so other nodes can find
/// and reach us. The record is keyed by our peer ID and carries the addresses
/// we believe are reachable (confirmed external addresses first).
#[cfg(feature = "kad")]
fn maybe_announce(swarm: &mut Swarm<AppBehaviour>) {
    if !crate::net_diag::announce_due() {
        return;
    }
    let details = crate::net_diag::snapshot();
    if details.dht_routing_peers == 0 {
        return;
    }
    let addresses = details.reachable_addresses();
    if addresses.is_empty() {
        return;
    }
    let peer_id = swarm.local_peer_id().to_string();
    let key = libp2p::kad::RecordKey::new(&format!("/p2p-chat/peer/{peer_id}"));
    let Some(value) =
        crate::net_diag::announce_record_json(&peer_id, &addresses, details.nat_status == "Public")
    else {
        return;
    };
    let record = libp2p::kad::Record::new(key, value);
    crate::net_diag::record_dht_announce_attempt();
    if let Err(e) = swarm
        .behaviour_mut()
        .kad
        .put_record(record, libp2p::kad::Quorum::One)
    {
        crate::net_diag::record_dht_announce_result(false, &format!("Announce failed: {e}"));
    }
}

#[allow(clippy::too_many_lines)]
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
                handle_group_message(message, peer_id, event_tx, swarm).await;
                return;
            }

            if let Ok(bcast) = serde_json::from_slice::<BroadcastMessage>(&message.data) {
                handle_broadcast_message(bcast, peer_id, peer_id_str, event_tx, swarm).await;
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
        Libp2pSwarmEvent::Behaviour(AppEv::FileTransfer(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            handle_file_transfer(message, peer, event_tx, swarm).await;
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
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Identify(libp2p::identify::Event::Received {
            peer_id,
            info,
            ..
        })) => {
            handle_identify_received(peer_id, &info, swarm);
        }
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Identify(_)) => {}
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Kad(libp2p::kad::Event::RoutingUpdated { .. })) => {
            refresh_dht_routing(swarm);
            maybe_announce(swarm);
        }
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Kad(libp2p::kad::Event::ModeChanged { new_mode })) => {
            crate::net_diag::record_dht_mode(&format!("{new_mode:?}"));
            refresh_dht_routing(swarm);
            maybe_announce(swarm);
        }
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Kad(libp2p::kad::Event::OutboundQueryProgressed {
            result,
            ..
        })) => {
            handle_kad_query(result);
        }
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Kad(_)) => {}
        #[cfg(feature = "relay")]
        Libp2pSwarmEvent::Behaviour(AppEv::RelayServer(_)) => {}
        #[cfg(feature = "relay")]
        Libp2pSwarmEvent::Behaviour(AppEv::RelayClient(
            libp2p::relay::client::Event::ReservationReqAccepted {
                relay_peer_id,
                renewal,
                ..
            },
        )) => {
            crate::net_diag::record_relay_reservation(&relay_peer_id.to_string());
            if renewal {
                crate::net_diag::record_relay_status(&format!(
                    "Reservation renewed via {relay_peer_id}"
                ));
            }
        }
        #[cfg(feature = "relay")]
        Libp2pSwarmEvent::Behaviour(AppEv::RelayClient(
            libp2p::relay::client::Event::OutboundCircuitEstablished { relay_peer_id, .. },
        )) => {
            crate::net_diag::record_relay_status(&format!("Outbound circuit via {relay_peer_id}"));
        }
        #[cfg(feature = "relay")]
        Libp2pSwarmEvent::Behaviour(AppEv::RelayClient(
            libp2p::relay::client::Event::InboundCircuitEstablished { src_peer_id, .. },
        )) => {
            crate::net_diag::record_relay_status(&format!("Inbound circuit from {src_peer_id}"));
        }
        #[cfg(feature = "nat")]
        Libp2pSwarmEvent::Behaviour(AppEv::Autonat(libp2p::autonat::Event::StatusChanged {
            new,
            ..
        })) => {
            let (label, public_addr) = match new {
                libp2p::autonat::NatStatus::Public(addr) => ("Public", Some(addr.to_string())),
                libp2p::autonat::NatStatus::Private => ("Private", None),
                libp2p::autonat::NatStatus::Unknown => ("Unknown", None),
            };
            crate::net_diag::record_nat_status(label, public_addr.as_deref());
            let _ = event_tx
                .send(SwarmEvent::NatStatusChanged(label.to_string()))
                .await;
            #[cfg(feature = "kad")]
            maybe_announce(swarm);
        }
        #[cfg(feature = "nat")]
        Libp2pSwarmEvent::Behaviour(AppEv::Autonat(libp2p::autonat::Event::OutboundProbe(
            event,
        ))) => match event {
            libp2p::autonat::OutboundProbeEvent::Response { .. } => {
                crate::net_diag::record_nat_probe(true);
            }
            libp2p::autonat::OutboundProbeEvent::Error { .. } => {
                crate::net_diag::record_nat_probe(false);
            }
            libp2p::autonat::OutboundProbeEvent::Request { .. } => {}
        },
        #[cfg(feature = "nat")]
        Libp2pSwarmEvent::Behaviour(AppEv::Autonat(_)) => {}
        #[cfg(feature = "nat")]
        Libp2pSwarmEvent::Behaviour(AppEv::Dcutr(libp2p::dcutr::Event {
            remote_peer_id,
            result,
        })) => {
            let success = result.is_ok();
            crate::net_diag::record_hole_punch(success);
            let _ = event_tx
                .send(SwarmEvent::HolePunchResult {
                    peer_id: remote_peer_id.to_string(),
                    success,
                })
                .await;
        }
        Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } => {
            handle_connection_established(peer_id, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::ConnectionClosed { peer_id, .. } => {
            handle_connection_closed(peer_id, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::NewListenAddr { address, .. } => {
            crate::net_diag::record_listen_addr(&address.to_string());
            let _ = event_tx
                .send(SwarmEvent::ListenAddrEstablished(address.to_string()))
                .await;
        }
        Libp2pSwarmEvent::NewExternalAddrCandidate { address } => {
            crate::net_diag::record_external_candidate(&address.to_string());
        }
        Libp2pSwarmEvent::ExternalAddrConfirmed { address } => {
            crate::net_diag::record_external_addr(&address.to_string());
            let _ = event_tx
                .send(SwarmEvent::ExternalAddressAdded(address.to_string()))
                .await;
            #[cfg(feature = "kad")]
            maybe_announce(swarm);
        }
        Libp2pSwarmEvent::ExternalAddrExpired { address } => {
            crate::net_diag::expire_external_addr(&address.to_string());
        }
        _ => {}
    }
}

#[cfg(feature = "kad")]
async fn handle_connection_established(
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let _ = event_tx
        .send(SwarmEvent::PeerConnected(peer_id.to_string()))
        .await;
    let _ = swarm.behaviour_mut().kad.bootstrap();
    refresh_dht_routing(swarm);
    maybe_announce(swarm);
}

#[cfg(not(feature = "kad"))]
async fn handle_connection_established(
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    _swarm: &mut Swarm<AppBehaviour>,
) {
    let _ = event_tx
        .send(SwarmEvent::PeerConnected(peer_id.to_string()))
        .await;
}

async fn handle_connection_closed(
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let _ = event_tx
        .send(SwarmEvent::PeerDisconnected(peer_id.to_string()))
        .await;
    #[cfg(feature = "kad")]
    refresh_dht_routing(swarm);
}

async fn handle_group_message(
    message: gossipsub::Message,
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
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
                    attachment: group_msg.attachment.clone(),
                },
            ) {
                Ok(Some(_)) => {}
                Ok(None) => p2plog_debug(format!(
                    "Dropped duplicate group message {} in {group_id}",
                    group_msg.msg_id.clone().unwrap_or_default()
                )),
                Err(e) => p2plog_debug(format!("Failed to save group message: {e:?}")),
            }

            // Attachments (files lane §12) are pulled from the group sender.
            if let Some(attachment) = group_msg.attachment.as_ref() {
                maybe_start_fetch(&peer_id, attachment, swarm);
            }

            let _ = event_tx
                .send(SwarmEvent::GroupMessage(crate::GroupMessageEvent {
                    group_id: group_id.to_string(),
                    content: group_msg.content,
                    peer_id: peer_id_str,
                    nickname: group_msg.nickname,
                    msg_id: group_msg.msg_id,
                    attachment: group_msg.attachment,
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
            attachment: row
                .attachment_json
                .as_deref()
                .and_then(crate::files::attachment_from_json),
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
            // An attachment-only DM is a real message despite empty content
            // (files lane §12), so the empty-content control paths below only
            // apply when no attachment rides along.
            if request.content.trim().is_empty() && request.attachment.is_none() {
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
                            attachment: None,
                            encrypted: false,
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
                                attachment: request.attachment.clone(),
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
                            attachment: request.attachment.clone(),
                            encrypted: false,
                        }))
                        .await;
                }

                // Attachments (files lane §12) are pulled from the sender of
                // the DM — the peer that has the blob staged to serve it.
                if let Some(attachment) = request.attachment.as_ref() {
                    maybe_start_fetch(&peer, attachment, swarm);
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
        attachment: None,
        encrypted: None,
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
    sender: &libp2p::PeerId,
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
        Some(&sender.to_string()),
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
                "claim" => match verify_and_bind_claim(&request, &peer, swarm) {
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

/// Process-wide receiver state for in-flight attachment pulls.
///
/// The files lane (§12) is receiver-driven: each `"chunk"` request is a
/// separate round-trip, so the partial blob and its metadata must live outside
/// any single handler frame. Keyed by content address (the `file_id`).
static RECEIVING_FILES: OnceLock<Mutex<StringMap<crate::files::ReceivingFile>>> = OnceLock::new();

fn receiving_files() -> &'static Mutex<StringMap<crate::files::ReceivingFile>> {
    RECEIVING_FILES.get_or_init(|| Mutex::new(StringMap::default()))
}

/// Garbage-collect dead attachment blobs (files lane §12): `.part` partials of
/// interrupted pulls and orphaned blobs/rows nothing references any more.
///
/// The `attachment-gc` Cargo feature is the enable/disable switch: with the
/// feature off this returns a zeroed report and the store never deletes
/// anything, so a build can opt out of blob cleanup entirely. Blobs with a
/// transfer currently in flight are excluded, and anything younger than
/// [`GC_GRACE`](crate::files::GC_GRACE) is skipped.
///
/// # Errors
/// Returns an error if a sweep fails (see [`crate::files::gc`]).
#[cfg(feature = "attachment-gc")]
pub fn gc_stale_attachments() -> color_eyre::Result<crate::files::GcStats> {
    let active: std::collections::HashSet<String> = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .keys()
        .cloned()
        .collect();
    crate::files::gc(&active, crate::files::GC_GRACE)
}

/// No-op build of [`gc_stale_attachments`] when the `attachment-gc` feature is
/// off (see the feature's doc comment in Cargo.toml).
#[cfg(not(feature = "attachment-gc"))]
pub fn gc_stale_attachments() -> color_eyre::Result<crate::files::GcStats> {
    Ok(crate::files::GcStats::default())
}

/// What applying one chunk response should do next.
enum ChunkOutcome {
    /// All bytes are in: hand the assembled transfer to the finalizer.
    Complete(crate::files::ReceivingFile),
    /// Append succeeded but the file is incomplete: pull chunk `u64` next.
    Continue(u64),
    /// The transfer cannot go on (unknown/errored); any partial is discarded.
    Failed,
}

/// Append one chunk's bytes to the in-flight transfer and decide what happens
/// next. Kept fully synchronous so no `MutexGuard` (not `Send`) ever crosses an
/// `.await` in the handler task.
fn apply_chunk(file_id: &str, data: Option<Vec<u8>>) -> ChunkOutcome {
    let mut transfers = receiving_files()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    let Some(receiving) = transfers.get_mut(file_id) else {
        p2plog_debug(format!("chunk for unknown transfer {file_id}"));
        return ChunkOutcome::Failed;
    };
    if let Some(data) = data
        && let Err(e) = receiving.write_chunk(&data)
    {
        p2plog_debug(format!("failed to write chunk for {file_id}: {e}"));
        let partial = transfers.remove(file_id);
        if let Some(partial) = partial {
            partial.discard();
        }
        return ChunkOutcome::Failed;
    }
    let complete = receiving.is_complete();
    let received = receiving.received;
    if complete {
        let partial = transfers.remove(file_id);
        drop(transfers);
        return partial.map_or(ChunkOutcome::Failed, ChunkOutcome::Complete);
    }
    drop(transfers);
    ChunkOutcome::Continue(received.div_euclid(crate::files::FILE_CHUNK_SIZE))
}

/// Receiver side of the attachment pull: if we neither hold the blob in the
/// store nor have a transfer in flight, ask the source peer for its metadata
/// to begin the chunk chain. Content-addressed, so an already-stored file
/// short-circuits the whole transfer.
fn maybe_start_fetch(
    peer_id: &libp2p::PeerId,
    attachment: &crate::files::MessageAttachment,
    swarm: &mut Swarm<AppBehaviour>,
) {
    if crate::files::file_path(&attachment.file_id).is_none()
        && !receiving_files()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(&attachment.file_id)
    {
        request_file_meta(swarm, peer_id, &attachment.file_id);
    }
}

fn request_file_meta(swarm: &mut Swarm<AppBehaviour>, peer_id: &libp2p::PeerId, file_id: &str) {
    let request = crate::FileTransfer {
        kind: "meta".to_string(),
        file_id: file_id.to_string(),
        chunk_index: None,
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_request(peer_id, request);
}

fn request_file_chunk(
    swarm: &mut Swarm<AppBehaviour>,
    peer_id: &libp2p::PeerId,
    file_id: &str,
    chunk_index: u64,
) {
    let request = crate::FileTransfer {
        kind: "chunk".to_string(),
        file_id: file_id.to_string(),
        chunk_index: Some(chunk_index),
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_request(peer_id, request);
}

/// Handle the file-transfer lane (§12).
///
/// Requests come from a peer who wants a blob we hold in the store: serve
/// either its metadata (name/size/mime plus the chunking parameters) or one
/// chunk of bytes. Responses come from a peer we are pulling from: `"meta"`
/// starts a transfer, each `"chunk"` appends its bytes (and requests the next
/// one unless the transfer just completed), and `"error"` abandons it.
async fn handle_file_transfer(
    message: libp2p::request_response::Message<crate::FileTransfer, crate::FileTransferResponse>,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => serve_file_transfer_request(&request, channel, swarm),
        libp2p::request_response::Message::Response { response, .. } => {
            handle_file_transfer_response(response, peer, peer_id_str, event_tx, swarm).await;
        }
    }
}

/// Serve a transfer request from a peer who wants a blob we hold: answer
/// `"meta"` with the descriptor + chunking parameters, `"chunk"` with the
/// requested slice, or `"error"` for anything else.
fn serve_file_transfer_request(
    request: &crate::FileTransfer,
    channel: libp2p::request_response::ResponseChannel<crate::FileTransferResponse>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let file_id = request.file_id.clone();
    let response = match request.kind.as_str() {
        "meta" => match crate::files::serve_file_meta(&file_id) {
            Some((name, size, mime)) => crate::FileTransferResponse {
                kind: "meta".to_string(),
                file_id,
                name: Some(name),
                size: Some(size),
                mime,
                total_chunks: Some(crate::files::total_chunks(size)),
                chunk_size: Some(crate::files::FILE_CHUNK_SIZE),
                ..Default::default()
            },
            None => crate::FileTransferResponse {
                kind: "error".to_string(),
                file_id,
                reason: Some("unknown file".to_string()),
                ..Default::default()
            },
        },
        "chunk" => {
            let chunk_index = request.chunk_index.unwrap_or_default();
            match crate::files::serve_chunk(&file_id, chunk_index) {
                Some(data) => crate::FileTransferResponse {
                    kind: "chunk".to_string(),
                    file_id,
                    chunk_index: Some(chunk_index),
                    data: Some(data),
                    ..Default::default()
                },
                None => crate::FileTransferResponse {
                    kind: "error".to_string(),
                    file_id: file_id.clone(),
                    reason: Some(format!("no chunk {chunk_index} for {file_id}")),
                    ..Default::default()
                },
            }
        }
        other => crate::FileTransferResponse {
            kind: "error".to_string(),
            file_id,
            reason: Some(format!("unknown file-transfer kind: {other}")),
            ..Default::default()
        },
    };
    let _ = swarm
        .behaviour_mut()
        .file_transfer
        .send_response(channel, response);
}

/// Receiver side of the pull: `"meta"` starts a transfer (refusing oversized
/// files and finishing instantly when empty), each `"chunk"` appends its bytes
/// and pulls the next one when incomplete, and `"error"` abandons the transfer.
async fn handle_file_transfer_response(
    response: crate::FileTransferResponse,
    peer: libp2p::PeerId,
    peer_id_str: String,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match response.kind.as_str() {
        "meta" => {
            let file_id = response.file_id.clone();
            let size = response.size.unwrap_or_default();
            if size > crate::files::MAX_ATTACHMENT_SIZE {
                receiving_files()
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&file_id);
                p2plog_debug(format!(
                    "refusing oversized attachment {file_id} ({size} bytes)"
                ));
                return;
            }
            let receiving = crate::files::ReceivingFile::new(
                &file_id,
                response.name.as_deref().unwrap_or("attachment"),
                response.mime.clone(),
                size,
            );
            match receiving {
                Ok(receiving) => {
                    receiving_files()
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .insert(file_id.clone(), receiving);
                    // A zero-length file completes immediately.
                    if size == 0 {
                        let receiving = receiving_files()
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .remove(&file_id);
                        if let Some(receiving) = receiving {
                            finish_received_file(receiving, peer_id_str, event_tx).await;
                        }
                    } else {
                        request_file_chunk(swarm, &peer, &file_id, 0);
                    }
                }
                Err(e) => {
                    p2plog_debug(format!("failed to open receiving file for {file_id}: {e}"));
                }
            }
        }
        "chunk" => {
            let file_id = response.file_id.clone();
            match apply_chunk(&file_id, response.data) {
                ChunkOutcome::Complete(partial) => {
                    finish_received_file(partial, peer_id_str, event_tx).await;
                }
                ChunkOutcome::Continue(next_chunk) => {
                    request_file_chunk(swarm, &peer, &file_id, next_chunk);
                }
                ChunkOutcome::Failed => {
                    p2plog_debug(format!("file transfer {file_id} could not continue"));
                }
            }
        }
        "error" => {
            let file_id = response.file_id.clone();
            let reason = response.reason.unwrap_or_default();
            let partial = receiving_files()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&file_id);
            if let Some(partial) = partial {
                partial.discard();
            }
            p2plog_debug(format!("file transfer {file_id} failed: {reason}"));
        }
        other => p2plog_debug(format!("unknown file-transfer response kind: {other}")),
    }
}

/// Surface an assembled attachment to the frontends. The transfer is already
/// drained from the in-flight map and fully stored by the time this runs.
async fn finish_received_file(
    receiving: crate::files::ReceivingFile,
    peer_id: String,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    let file_id = receiving.file_id.clone();
    let name = receiving.name.clone();
    let path = match receiving.finalize() {
        Ok(path) => path,
        Err(e) => {
            p2plog_debug(format!("failed to finalize attachment {file_id}: {e}"));
            return;
        }
    };
    let _ = event_tx
        .send(SwarmEvent::FileReceived {
            peer_id,
            file_id,
            name,
            path,
        })
        .await;
}

/// Build a `BroadcastMessage` from component parts
#[must_use]
pub fn build_broadcast_message(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
) -> BroadcastMessage {
    build_broadcast_message_with_attachment(content, nickname, msg_id, None)
}

/// Build a `BroadcastMessage` from component parts, optionally with an
/// attachment descriptor (files lane §12).
#[must_use]
pub fn build_broadcast_message_with_attachment(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
) -> BroadcastMessage {
    BroadcastMessage {
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
        attachment,
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
    build_group_message_with_attachment(group_id, content, nickname, msg_id, None)
}

/// Build a `GroupMessage` from component parts, optionally with an attachment
/// descriptor (files lane §12).
#[must_use]
pub fn build_group_message_with_attachment(
    group_id: String,
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
) -> crate::GroupMessage {
    crate::GroupMessage {
        group_id,
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
        attachment,
    }
}

fn handle_command(cmd: SwarmCommand, swarm: &mut Swarm<AppBehaviour>, topic: &str) {
    match cmd {
        SwarmCommand::Publish {
            content,
            nickname,
            msg_id,
            attachment,
        } => publish_broadcast(content, nickname, msg_id, attachment, swarm, topic),
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
            attachment,
        } => send_dm(
            &peer_id, content, nickname, msg_id, ack_for, attachment, swarm,
        ),
        SwarmCommand::PublishGroup {
            group_id,
            content,
            nickname,
            msg_id,
            attachment,
        } => publish_group(content, nickname, msg_id, group_id, attachment, swarm),
        SwarmCommand::SubscribeGroup { group_id } => subscribe_to_group(&group_id, swarm),
        SwarmCommand::SendDeviceBinding { peer_id, request } => {
            send_device_binding(&peer_id, request, swarm);
        }
        SwarmCommand::SendCallSignal { peer_id, signal } => {
            send_call_signal(&peer_id, *signal, swarm);
        }
        SwarmCommand::UnsubscribeGroup { group_id } => unsubscribe_from_group(&group_id, swarm),
        SwarmCommand::Dial { address } => dial_peer(&address, swarm),
    }
}

fn publish_broadcast(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &str,
) {
    let msg = build_broadcast_message_with_attachment(content, nickname, msg_id, attachment);
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

/// Dial a peer by multiaddr (manual peer address entry, US-053).
fn dial_peer(address: &str, swarm: &mut Swarm<AppBehaviour>) {
    match address.trim().parse::<libp2p::Multiaddr>() {
        Ok(addr) => match swarm.dial(addr.clone()) {
            Ok(()) => p2plog_debug(format!("Dialing {addr}")),
            Err(e) => p2plog_error(format!("Failed to dial {addr}: {e}")),
        },
        Err(e) => p2plog_error(format!("Invalid multiaddr {address}: {e}")),
    }
}

fn send_dm(
    peer_id: &str,
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    ack_for: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
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
            attachment,
            encrypted: None,
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
    attachment: Option<crate::files::MessageAttachment>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
    let msg = build_group_message_with_attachment(group_id, content, nickname, msg_id, attachment);
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
        // Bootstrap (US-050): dial peers so the node can join a public network
        // instead of staying LAN-only. Source of the list:
        //   1. P2P_BOOTSTRAP env var (comma-separated multiaddrs), if set.
        //   2. Otherwise the public IPFS bootstrap nodes, so discovery works
        //      out of the box without hosting our own. Set P2P_BOOTSTRAP to an
        //      empty string to disable bootstrapping entirely.
        // The Kademlia behaviour is forced into client mode (see behavior.rs),
        // so we route through the public DHT without storing/serving its
        // records.
        #[cfg(feature = "kad")]
        {
            // Public IPFS bootstrap is opt-in: only fall back to it when the user
            // explicitly enables it (P2P_PUBLIC_DHT=1/true) or sets P2P_BOOTSTRAP.
            // Otherwise we stay LAN-only (mDNS), avoiding global DHT traffic.
            let env_bootstrap = std::env::var("P2P_BOOTSTRAP").ok();
            let public_enabled = std::env::var("P2P_PUBLIC_DHT")
                .is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"));
            let list = env_bootstrap.map_or_else(
                || {
                    if public_enabled {
                        [
                            "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfu53Fn2A9xSfDMGcR6fYSdrNj4HgjCZNnTE5",
                            "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJ9GD89Nm3HLQQpXePmCRp",
                            "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
                            "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
                        ]
                        .join(",")
                    } else {
                        String::new()
                    }
                },
                |v| {
                    if v.trim().is_empty() {
                        String::new()
                    } else {
                        v
                    }
                },
            );
            if !list.is_empty() {
                for addr in list.split(',') {
                    let addr = addr.trim();
                    if addr.is_empty() {
                        continue;
                    }
                    if let Ok(ma) = addr.parse::<libp2p::Multiaddr>() {
                        let _ = swarm.dial(ma);
                    } else {
                        p2plog_debug(format!("Invalid bootstrap multiaddr: {addr}"));
                    }
                }
            }
        }
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
