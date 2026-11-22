//! Swarm event handler that translates libp2p events to application events

use crate::call_media::{AudioFrame, FrameAck, MediaSession};
use crate::file_transfer::{
    FILE_TRANSFER_IDLE_TIMEOUT, abandon_idle_transfers, abandon_transfers_from,
    handle_file_transfer, maybe_start_fetch,
};
use crate::types::{SwarmCommand, SwarmEvent};
use crate::{
    AppBehaviour, BroadcastMessage, behavior::AppBehaviourEvent as AppEv, current_timestamp,
    p2plog_debug, p2plog_error,
};
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::mpsc;

enum Event {
    Swarm(Box<Libp2pSwarmEvent<AppEv>>),
    // Boxed so the select-loop event stays small; `SwarmCommand` grows with
    // each outbound message field (attachment, reply snapshot, …).
    Command(Box<SwarmCommand>),
    // Periodic store-and-forward retry/prune tick.
    Flush,
}

/// Whether a peer is muted (US-054). Fail-open: an unreadable database never
/// mutes anyone, so this can be called on every event without gating traffic
/// behind a healthy DB.
fn peer_is_blocked(peer_id: &libp2p::PeerId) -> bool {
    crate::blocklist::is_blocked(&peer_id.to_string())
}

/// Resolve the authenticated author of a gossipsub message.
///
/// Gossipsub runs with [`gossipsub::MessageAuthenticity::Signed`], so `source`
/// is the signature-verified publisher. `propagation_source` is only the last
/// hop that forwarded the message to us. Attributing a relayed message to the
/// last hop would show the wrong sender, record the wrong group member, give
/// the same message a different synthetic id on each relay path (breaking
/// dedup), and send the delivery receipt to a peer that never recorded the
/// broadcast recipient row. Unsigned/local messages have no `source`, in which
/// case the last hop is the best available attribution.
#[must_use]
pub const fn gossipsub_author(
    source: Option<libp2p::PeerId>,
    propagation_source: libp2p::PeerId,
) -> libp2p::PeerId {
    match source {
        Some(author) => author,
        None => propagation_source,
    }
}

/// Dispatch a decoded broadcast: surface it as an event attributed to the
/// signed publisher, pull its attachment from the relay that forwarded it
/// (files lane §12), and send a best-effort receipt confirmation to the
/// publisher so it can confirm the right `broadcast_recipients` row.
async fn handle_broadcast_message(
    bcast: BroadcastMessage,
    author: libp2p::PeerId,
    relay: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let latency = Some(crate::format_latency(bcast.sent_at, SystemTime::now()));

    let _ = event_tx
        .send(SwarmEvent::BroadcastMessage(crate::MessageEvent {
            content: bcast.content,
            peer_id: author.to_string(),
            latency,
            nickname: bcast.nickname.clone(),
            msg_id: bcast.msg_id.clone(),
            attachment: bcast.attachment.clone(),
            encrypted: false,
            reply_to: bcast.reply_to.clone(),
        }))
        .await;

    // Attachments are pulled from the relay — the last hop that re-sent the
    // message keeps the blob on disk to serve the chunk requests.
    if let Some(attachment) = bcast.attachment.as_ref() {
        maybe_start_fetch(&relay, attachment, swarm);
    }

    // Best-effort receipt confirmation for broadcasts: send a receipt-only
    // DM to the publisher, whose `broadcast_recipients` row can then be
    // confirmed. A relay never recorded that row, so acking it would be lost.
    if let Some(ack_for) = bcast.msg_id.clone() {
        let receipt = make_ack_dm(String::new(), Some(ack_for));
        swarm
            .behaviour_mut()
            .request_response
            .send_request(&author, receipt);
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
    // LAN-only mode (US-051): never publish our record to the global DHT.
    if crate::settings::lan_only() {
        return;
    }
    if !crate::net_diag::announce_due() {
        return;
    }
    let details = crate::net_diag::snapshot();
    if details.dht_routing_peers == 0 {
        return;
    }
    let addresses = details.announce_addresses(&swarm.local_peer_id().to_string());
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

/// Circuit-relay reservation bookkeeping for one running node.
///
/// A reservation is requested by listening on the relayed address (see
/// `crate::relay`); this tracks the configured relays, which ones we already
/// asked, and the listeners we opened so a closed listener can free the relay
/// for a retry.
#[cfg_attr(not(feature = "relay"), allow(dead_code))]
struct RelayState {
    /// Configured relay multiaddrs (`P2P_RELAY` + the persisted list, each
    /// with `/p2p/<relay-id>`).
    addrs: Vec<libp2p::Multiaddr>,
    /// Relays we have already asked for a reservation.
    requested: std::collections::HashSet<libp2p::PeerId>,
    /// Open relay listeners, so a close can clear its relay's reservation.
    listeners: std::collections::HashMap<libp2p::core::transport::ListenerId, libp2p::PeerId>,
}

impl RelayState {
    fn new() -> Self {
        Self {
            addrs: Vec::new(),
            requested: std::collections::HashSet::new(),
            listeners: std::collections::HashMap::new(),
        }
    }

    /// Load the configured relays and request a reservation with each.
    #[cfg(feature = "relay")]
    fn configure(&mut self, swarm: &mut Swarm<AppBehaviour>) {
        self.addrs = crate::relay::effective_relay_addrs();
        let peers: Vec<libp2p::PeerId> = self
            .addrs
            .iter()
            .filter_map(crate::network_config::multiaddr_peer_id)
            .collect();
        for peer_id in peers {
            self.request_reservation(peer_id, swarm);
        }
    }

    #[cfg(not(feature = "relay"))]
    fn configure(&mut self, _swarm: &mut Swarm<AppBehaviour>) {}

    /// Request a reservation if `peer_id` is one of the configured relays.
    #[cfg(feature = "relay")]
    fn request_reservation(&mut self, peer_id: libp2p::PeerId, swarm: &mut Swarm<AppBehaviour>) {
        if self.requested.contains(&peer_id) {
            return;
        }
        let Some(addr) = self
            .addrs
            .iter()
            .find(|addr| crate::network_config::multiaddr_peer_id(addr) == Some(peer_id))
        else {
            return;
        };
        let Some(listen_addr) = crate::relay::relayed_listen_addr(addr) else {
            return;
        };
        match swarm.listen_on(listen_addr) {
            Ok(listener_id) => {
                self.requested.insert(peer_id);
                self.listeners.insert(listener_id, peer_id);
                crate::net_diag::record_relay_status(&format!(
                    "Requesting reservation via {peer_id}"
                ));
            }
            Err(e) => crate::net_diag::record_relay_status(&format!(
                "Reservation request via {peer_id} failed: {e}"
            )),
        }
    }

    #[cfg(not(feature = "relay"))]
    fn request_reservation(&mut self, _peer_id: libp2p::PeerId, _swarm: &mut Swarm<AppBehaviour>) {}

    /// A relay listener closed: drop the reservation and allow a retry.
    fn on_listener_closed(
        &mut self,
        listener_id: libp2p::core::transport::ListenerId,
        reason: &str,
    ) {
        let Some(peer_id) = self.listeners.remove(&listener_id) else {
            return;
        };
        self.requested.remove(&peer_id);
        crate::net_diag::expire_relay_reservation(&peer_id.to_string());
        crate::net_diag::record_relay_status(&format!("Reservation via {peer_id} ended: {reason}"));
    }
}

async fn handle_swarm_event(
    swarm_event: Libp2pSwarmEvent<AppEv>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    relays: &mut RelayState,
    announced: &mut AnnouncedConnections,
    media: &mut Option<MediaSession>,
) {
    match swarm_event {
        Libp2pSwarmEvent::Behaviour(_) => {
            handle_behaviour_event(swarm_event, event_tx, swarm, media).await;
        }
        _ => {
            handle_connection_event(swarm_event, event_tx, swarm, relays, announced).await;
        }
    }
}

async fn handle_behaviour_event(
    swarm_event: Libp2pSwarmEvent<AppEv>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    media: &mut Option<MediaSession>,
) {
    match swarm_event {
        Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(gossipsub::Event::Message {
            propagation_source,
            message,
            ..
        })) => {
            handle_gossipsub_message(message, propagation_source, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::RequestResponse(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("DM", &peer) {
                return;
            }
            handle_request_response(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::DeviceBinding(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("device-binding traffic", &peer) {
                return;
            }
            handle_device_binding(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::CallSignaling(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("call signal", &peer) {
                return;
            }
            handle_call_signal(message, peer, event_tx, swarm, media).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::CallMedia(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("call media", &peer) {
                return;
            }
            handle_call_media(message, peer, event_tx, swarm, media).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::Typing(libp2p::request_response::Event::Message {
            peer,
            message,
            ..
        })) => {
            if drop_blocked_lane("typing signal", &peer) {
                return;
            }
            handle_typing_signal(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::Reactions(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("reaction signal", &peer) {
                return;
            }
            handle_reaction_signal(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::FileTransfer(
            libp2p::request_response::Event::Message { peer, message, .. },
        )) => {
            if drop_blocked_lane("file transfer", &peer) {
                return;
            }
            handle_file_transfer(message, peer, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::Behaviour(AppEv::Shares(libp2p::request_response::Event::Message {
            peer,
            message,
            ..
        })) => {
            if drop_blocked_lane("file share", &peer) {
                return;
            }
            handle_share_lane(message, peer, event_tx, swarm).await;
        }
        other => handle_discovery_event(other, event_tx, swarm).await,
    }
}

/// Blocklist gate shared by every request-response lane (US-054). Keeping the
/// log text assembled from one label makes it obvious no lane is unguarded.
fn drop_blocked_lane(label: &str, peer: &libp2p::PeerId) -> bool {
    if peer_is_blocked(peer) {
        p2plog_debug(format!("Dropped {label} from blocked peer {peer}"));
        true
    } else {
        false
    }
}

/// Route a verified gossipsub message: blocklist gate first (relay *and*
/// signed publisher), then group/backfill topics, then the broadcast path.
async fn handle_gossipsub_message(
    message: gossipsub::Message,
    propagation_source: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    // Gossipsub verifies the signature of every message (Signed
    // authenticity), so `source` is the true publisher and
    // `propagation_source` is only the peer that relayed it to us.
    let author = gossipsub_author(message.source, propagation_source);
    let propagation_str = propagation_source.to_string();

    // Blocklist (US-054): a muted peer's traffic never reaches the
    // parsers, so nothing it sends can be persisted. Check both the
    // relay (a muted forwarder) and the signed author (a muted
    // publisher whose message arrived via an unmuted relay).
    if peer_is_blocked(&propagation_source) || peer_is_blocked(&author) {
        p2plog_debug(format!(
            "Dropped gossipsub traffic from blocked peer (relay {propagation_str}, author {author})"
        ));
        return;
    }

    // Route per-group control/chat topics (group:<group_id> and the
    // derived group:<group_id>:backfill) before the broadcast path.
    // Backfill goes first: `group:X:backfill` also starts with the
    // group prefix, so the more specific check must win.
    if crate::groups::is_backfill_topic(&message.topic.to_string()) {
        handle_backfill_request(&message, author, swarm);
        return;
    }
    if crate::groups::is_group_topic(&message.topic.to_string()) {
        handle_group_message(message, author, propagation_source, event_tx, swarm).await;
        return;
    }

    if let Ok(bcast) = serde_json::from_slice::<BroadcastMessage>(&message.data) {
        handle_broadcast_message(bcast, author, propagation_source, event_tx, swarm).await;
    } else {
        p2plog_debug(format!(
            "Failed to parse broadcast message from peer {author}"
        ));
    }
}

/// Handle discovery, connection-quality and diagnostic behaviour events:
/// `mDNS`, `ping`, `Identify`, `Kademlia`, `relay`, `AutoNAT` and `DCUtR`.
async fn handle_discovery_event(
    swarm_event: Libp2pSwarmEvent<AppEv>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match swarm_event {
        #[cfg(feature = "mdns")]
        Libp2pSwarmEvent::Behaviour(AppEv::Mdns(libp2p::mdns::Event::Discovered(list))) => {
            handle_mdns_discovered(list, event_tx, swarm).await;
        }
        #[cfg(feature = "mdns")]
        Libp2pSwarmEvent::Behaviour(AppEv::Mdns(libp2p::mdns::Event::Expired(list))) => {
            handle_mdns_expired(list, event_tx).await;
        }
        // Ping results drive connection liveness and the per-peer connection
        // quality shown on the Peer Info page (RTT + loss). The close a ping
        // failure can cause is handled by the ConnectionClosed arm below.
        Libp2pSwarmEvent::Behaviour(AppEv::Ping(libp2p::ping::Event { peer, result, .. })) => {
            record_ping(peer, &result);
        }
        #[cfg(feature = "kad")]
        Libp2pSwarmEvent::Behaviour(AppEv::Identify(libp2p::identify::Event::Received {
            peer_id,
            info,
            ..
        })) if !peer_is_blocked(&peer_id) => {
            // A blocked peer is not added to the DHT/roster either.
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
        Libp2pSwarmEvent::Behaviour(AppEv::RelayClient(event)) => {
            handle_relay_client_event(&event);
        }
        #[cfg(feature = "nat")]
        Libp2pSwarmEvent::Behaviour(AppEv::Autonat(libp2p::autonat::Event::StatusChanged {
            new,
            ..
        })) => {
            handle_autonat_status_changed(new, event_tx, swarm).await;
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
        _ => {}
    }
}

/// Connections we have actually announced to the frontends as `PeerConnected`.
///
/// A blocked peer is disconnected in `handle_connection_established` *before*
/// `PeerConnected` is sent, but libp2p still emits its `ConnectionClosed`. Without
/// this tracker that close would emit an unmatched `PeerDisconnected`, decrementing
/// the frontends' connection count (and recording a phantom session sample) for a
/// peer that was never counted. The count handles libp2p's multiple connections
/// to one peer: every announced connect is balanced by exactly one disconnect.
#[derive(Default)]
struct AnnouncedConnections(HashMap<String, usize>);

impl AnnouncedConnections {
    /// Record a connection that is about to be announced as `PeerConnected`.
    fn on_connected(&mut self, peer_id: &str) {
        let count = self.0.entry(peer_id.to_string()).or_default();
        *count = count.saturating_add(1);
    }

    /// Record a closed connection. Returns `true` only when a matching
    /// `PeerConnected` was previously announced, so the caller may emit a
    /// `PeerDisconnected` event.
    fn on_disconnected(&mut self, peer_id: &str) -> bool {
        match self.0.get_mut(peer_id) {
            Some(count) if *count > 1 => {
                *count = count.saturating_sub(1);
                true
            }
            Some(_) => {
                self.0.remove(peer_id);
                true
            }
            None => false,
        }
    }
}

async fn handle_connection_event(
    swarm_event: Libp2pSwarmEvent<AppEv>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    relays: &mut RelayState,
    announced: &mut AnnouncedConnections,
) {
    match swarm_event {
        Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } => {
            let peer_id_str = peer_id.to_string();
            handle_connection_established(peer_id, event_tx, swarm, relays, announced).await;
            // Store-and-forward: retry any sealed DMs addressed to this peer
            // that were parked while it was offline.
            match crate::dm_outbox::pending_for(&peer_id_str) {
                Ok(rows) => retry_outbox_rows(rows, swarm),
                Err(e) => {
                    p2plog_debug(format!("Failed to load pending DMs on reconnect: {e}"));
                }
            }
        }
        Libp2pSwarmEvent::ConnectionClosed { peer_id, .. } => {
            handle_connection_closed(peer_id, event_tx, swarm, announced).await;
        }
        Libp2pSwarmEvent::NewListenAddr { address, .. } => {
            handle_new_listen_addr(address, event_tx).await;
        }
        Libp2pSwarmEvent::NewExternalAddrCandidate { address } => {
            crate::net_diag::record_external_candidate(&address.to_string());
        }
        Libp2pSwarmEvent::ExternalAddrConfirmed { address } => {
            handle_external_addr_confirmed(address, event_tx, swarm).await;
        }
        Libp2pSwarmEvent::ExternalAddrExpired { address } => {
            crate::net_diag::expire_external_addr(&address.to_string());
        }
        Libp2pSwarmEvent::ListenerClosed {
            listener_id,
            reason,
            ..
        } => {
            let reason = reason.map_or_else(|e| e.to_string(), |()| "closed".to_string());
            relays.on_listener_closed(listener_id, &reason);
        }
        Libp2pSwarmEvent::ListenerError { listener_id, error } => {
            relays.on_listener_closed(listener_id, &error.to_string());
        }
        _ => {}
    }
}

async fn handle_connection_established(
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    relays: &mut RelayState,
    announced: &mut AnnouncedConnections,
) {
    // Blocklist (US-054): refuse the connection instead of tracking it. The
    // peer can still resolve us via mDNS/DHT, so dropping the inbound dial is
    // the only way to keep it from meshing with us. Never announce it as
    // connected — otherwise its close emits an unmatched disconnect.
    if peer_is_blocked(&peer_id) {
        p2plog_debug(format!("Disconnecting blocked peer {peer_id}"));
        let _ = swarm.disconnect_peer_id(peer_id);
        return;
    }
    announced.on_connected(&peer_id.to_string());
    let _ = event_tx
        .send(SwarmEvent::PeerConnected(peer_id.to_string()))
        .await;
    #[cfg(feature = "kad")]
    if !crate::settings::lan_only() {
        let _ = swarm.behaviour_mut().kad.bootstrap();
        refresh_dht_routing(swarm);
    }
    relays.request_reservation(peer_id, swarm);
    #[cfg(feature = "kad")]
    maybe_announce(swarm);
}

async fn handle_connection_closed(
    peer_id: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    announced: &mut AnnouncedConnections,
) {
    // Only a peer we actually announced as connected may emit a disconnect.
    // A blocked peer is disconnected before `PeerConnected`, so its close must
    // not decrement the frontends' counts or record a phantom session sample.
    if announced.on_disconnected(&peer_id.to_string()) {
        let _ = event_tx
            .send(SwarmEvent::PeerDisconnected(peer_id.to_string()))
            .await;
    }
    // Once the peer's last connection is gone it can no longer serve any pull
    // it started: drop its stale ping samples and abandon its in-flight file
    // transfers so they neither wedge future fetches nor block blob GC.
    if !swarm.is_connected(&peer_id) {
        let peer_id_str = peer_id.to_string();
        crate::net_diag::clear_peer_quality(&peer_id_str);
        abandon_transfers_from(&peer_id_str);
        crate::shares::abandon_pending_from(&peer_id_str);
    }
    #[cfg(feature = "kad")]
    refresh_dht_routing(swarm);
}

/// Feed one ping outcome into the per-peer connection-quality sample.
fn record_ping(peer: libp2p::PeerId, result: &Result<std::time::Duration, libp2p::ping::Failure>) {
    match result {
        Ok(rtt) => {
            crate::net_diag::record_ping_rtt(&peer.to_string(), rtt.as_secs_f64() * 1000.0);
        }
        Err(_) => crate::net_diag::record_ping_failure(&peer.to_string()),
    }
}

async fn handle_new_listen_addr(address: libp2p::Multiaddr, event_tx: &mpsc::Sender<SwarmEvent>) {
    crate::net_diag::record_listen_addr(&address.to_string());
    let _ = event_tx
        .send(SwarmEvent::ListenAddrEstablished(address.to_string()))
        .await;
}

async fn handle_external_addr_confirmed(
    address: libp2p::Multiaddr,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    crate::net_diag::record_external_addr(&address.to_string());
    let _ = event_tx
        .send(SwarmEvent::ExternalAddressAdded(address.to_string()))
        .await;
    #[cfg(feature = "kad")]
    maybe_announce(swarm);
    #[cfg(not(feature = "kad"))]
    let _ = swarm;
}

#[cfg(feature = "mdns")]
async fn handle_mdns_discovered(
    list: Vec<(libp2p::PeerId, libp2p::Multiaddr)>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    for (peer_id, multiaddr) in list {
        // Blocklist (US-054): never dial or announce a muted peer, even when
        // it keeps advertising itself on the LAN.
        if peer_is_blocked(&peer_id) {
            continue;
        }
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
async fn handle_mdns_expired(
    list: Vec<(libp2p::PeerId, libp2p::Multiaddr)>,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    for (peer_id, _multiaddr) in list {
        let _ = event_tx
            .send(SwarmEvent::PeerExpired {
                peer_id: peer_id.to_string(),
            })
            .await;
    }
}

#[cfg(feature = "nat")]
async fn handle_autonat_status_changed(
    new: libp2p::autonat::NatStatus,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
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

#[cfg(feature = "relay")]
fn handle_relay_client_event(event: &libp2p::relay::client::Event) {
    match event {
        libp2p::relay::client::Event::ReservationReqAccepted {
            relay_peer_id,
            renewal,
            ..
        } => {
            crate::net_diag::record_relay_reservation(&relay_peer_id.to_string());
            if *renewal {
                crate::net_diag::record_relay_status(&format!(
                    "Reservation renewed via {relay_peer_id}"
                ));
            }
        }
        libp2p::relay::client::Event::OutboundCircuitEstablished { relay_peer_id, .. } => {
            crate::net_diag::record_relay_status(&format!("Outbound circuit via {relay_peer_id}"));
        }
        libp2p::relay::client::Event::InboundCircuitEstablished { src_peer_id, .. } => {
            crate::net_diag::record_relay_status(&format!("Inbound circuit from {src_peer_id}"));
        }
    }
}

/// Whether an incoming-message save actually inserted a new row.
///
/// `Ok(None)` is a mesh retransmit already persisted on an earlier delivery;
/// `Err` is a failed write. Neither may be forwarded to a frontend: the TUI
/// appends live events to its in-memory history without its own dedup, and a
/// row that never persisted would vanish on the next reload. Mirrors the DM
/// replay guard (`messages::incoming_dm_exists`).
const fn newly_persisted<T, E>(saved: &Result<Option<T>, E>) -> bool {
    matches!(saved, Ok(Some(_)))
}

/// Run a fallible operation up to `attempts` times, returning the first success.
///
/// Gossipsub dedups by message id, so it never redelivers a message we already
/// saw — a single transient failure saving an incoming message (brief lock
/// contention, a connection/migration hiccup) would lose that message forever.
/// The save is `ON CONFLICT DO NOTHING`, so a retry is safe: if the first attempt
/// did commit before reporting an error, the retry returns `Ok(None)` and the
/// normal duplicate path absorbs it. No sleep between attempts — SQLite's own
/// `busy_timeout` already covers contention, so an immediate retry suffices.
fn retry_transient<T, E>(mut attempts: usize, mut f: impl FnMut() -> Result<T, E>) -> Result<T, E> {
    loop {
        match f() {
            Ok(value) => return Ok(value),
            Err(_) if attempts > 1 => {
                attempts = attempts.saturating_sub(1);
            }
            Err(err) => return Err(err),
        }
    }
}

/// Number of attempts for a transient receive-path DB write.
const INCOMING_SAVE_ATTEMPTS: usize = 3;

async fn handle_group_message(
    message: gossipsub::Message,
    author: libp2p::PeerId,
    relay: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    // `author` is the signed publisher; `relay` is only the last hop that
    // forwarded this message to us. Attribution, membership and the synthetic
    // id must use the author so the same message gets the same id on every
    // path (and dedup holds); only the attachment fetch uses the relay.
    let peer_id_str = author.to_string();
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
            // The persisted row and the live event must expose the same id, so a
            // reply captured from a transient event stays replyable after a
            // reload (including the synthetic id for wire-less messages).
            let event_msg_id = crate::groups::incoming_group_msg_id(
                group_id,
                &peer_id_str,
                &group_msg.content,
                group_msg.sent_at,
                group_msg.msg_id.clone(),
            );
            let saved = retry_transient(INCOMING_SAVE_ATTEMPTS, || {
                crate::groups::save_incoming_group_message(
                    group_id,
                    &peer_id_str,
                    &group_msg.content,
                    crate::groups::GroupMessageMeta {
                        sender_nickname: group_msg.nickname.clone(),
                        msg_id: group_msg.msg_id.clone(),
                        sent_at: group_msg.sent_at,
                        attachment: group_msg.attachment.clone(),
                        reply_to: group_msg.reply_to.clone(),
                    },
                )
            });
            // Only a freshly persisted row reaches a frontend. The DB is the
            // single source of truth for group history, and the TUI appends
            // live events to its in-memory history without its own dedup — so
            // a mesh retransmit (Ok(None)) would render twice. This mirrors the
            // DM replay guard (`incoming_dm_exists`). A failed save must not
            // surface a row that is absent on the next reload either.
            match &saved {
                Ok(Some(_)) => {}
                Ok(None) => p2plog_debug(format!(
                    "Dropped duplicate group message {} in {group_id}",
                    group_msg.msg_id.clone().unwrap_or_default()
                )),
                Err(e) => p2plog_debug(format!("Failed to save group message: {e:?}")),
            }
            if !newly_persisted(&saved) {
                return;
            }

            // Attachments (files lane §12) are pulled from the last hop that
            // can serve the blob, not from the signed publisher (which may be
            // several relays away).
            if let Some(attachment) = group_msg.attachment.as_ref() {
                maybe_start_fetch(&relay, attachment, swarm);
            }

            let _ = event_tx
                .send(SwarmEvent::GroupMessage(crate::GroupMessageEvent {
                    group_id: group_id.to_string(),
                    content: group_msg.content,
                    peer_id: peer_id_str,
                    nickname: group_msg.nickname,
                    msg_id: Some(event_msg_id),
                    attachment: group_msg.attachment,
                    reply_to: group_msg.reply_to,
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
            reply_to: row.reply_to_json.as_deref().and_then(crate::reply::decode),
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

/// Open a sealed DM body (US-042) before any control check. Returns `true`
/// when the request carried a valid envelope whose plaintext fields were
/// restored into `request`; `false` means the request was never sealed.
///
/// Error means a malformed/foreign envelope: the caller acks and drops it so it
/// never reaches a frontend.
fn open_encrypted_request(request: &mut crate::DirectMessage) -> color_eyre::Result<bool> {
    let Some(envelope) = request.encrypted.take() else {
        return Ok(false);
    };
    let opened = crate::e2e::open_incoming(request.msg_id.as_deref(), &envelope)?;
    request.content = opened.content;
    request.nickname = opened.nickname;
    request.attachment = opened.attachment;
    request.reply_to = opened.reply_to;
    Ok(true)
}

/// Dispatch the empty-content control paths of an incoming DM: a delivery
/// receipt (`ack_for`), a nickname-only greeting, or an empty message with
/// neither (dropped). Called only when the body is empty and carries no
/// attachment, so an attachment-only DM never takes this path.
async fn handle_empty_dm_control(
    request: &crate::DirectMessage,
    peer_id_str: &str,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    if let Some(ack_for) = &request.ack_for {
        let _ = event_tx
            .send(SwarmEvent::Receipt {
                peer_id: peer_id_str.to_string(),
                ack_for: ack_for.clone(),
                received_at: request.received_at,
            })
            .await;
    } else if request.nickname.is_some() {
        // Nickname-only DM: the empty-content exchange we send on connect (and
        // that the peer echoes). Deliver it as an empty `DirectMessage` so the
        // receiving frontend records the announced nickname (and touches the
        // peer's last-seen) without persisting a chat message.
        let _ = event_tx
            .send(SwarmEvent::DirectMessage(crate::MessageEvent {
                content: String::new(),
                peer_id: peer_id_str.to_string(),
                latency: Some(crate::format_latency(request.sent_at, SystemTime::now())),
                nickname: request.nickname.clone(),
                msg_id: None,
                attachment: None,
                encrypted: false,
                reply_to: None,
            }))
            .await;
    } else {
        p2plog_debug("Dropped empty DM with no ack_for or nickname".to_string());
    }
}

/// Ack a DM request so the sender can clear its outbox row, closing the
/// one-shot response channel. Every inbound DM path answers exactly once.
fn send_dm_ack(
    swarm: &mut Swarm<AppBehaviour>,
    channel: libp2p::request_response::ResponseChannel<crate::DirectMessage>,
    ack_for: Option<String>,
) {
    let response = make_ack_dm("ok".to_string(), ack_for);
    let _ = swarm
        .behaviour_mut()
        .request_response
        .send_response(channel, response);
}

/// Route one persisted-worthy DM to the frontends. Group traffic goes to the
/// group topic (or becomes an invite when we are not a member); everything
/// else is a direct-message event.
async fn dispatch_incoming_dm(
    request: crate::DirectMessage,
    peer_id_str: String,
    local_peer: String,
    encrypted: bool,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    let msg_id = request.msg_id;
    let latency = Some(crate::format_latency(request.sent_at, SystemTime::now()));
    if let Some(group_id) = request.group_id {
        if crate::groups::is_group_member(&group_id, &local_peer).unwrap_or(false) {
            let _ = event_tx
                .send(SwarmEvent::GroupMessage(crate::GroupMessageEvent {
                    group_id,
                    content: request.content,
                    peer_id: peer_id_str,
                    nickname: request.nickname,
                    msg_id,
                    attachment: request.attachment,
                    reply_to: request.reply_to,
                }))
                .await;
        } else {
            let _ = event_tx
                .send(SwarmEvent::GroupInvite {
                    group_id,
                    inviter_peer: peer_id_str,
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
                attachment: request.attachment,
                encrypted,
                reply_to: request.reply_to,
            }))
            .await;
    }
}

/// (peer, `msg_id`) markers for DMs already dispatched to a frontend, with the
/// dispatch time. The `incoming_dm_exists` DB check happens before dispatch,
/// but the frontend persists asynchronously after receiving the event; a
/// store-and-forward replay arriving in that window passes the DB check again
/// and renders twice (the `messages` table has no unique `(peer, msg_id)` key).
/// Marking at dispatch time — inside the single-threaded swarm loop — makes the
/// check-and-mark atomic; the marker is only needed until the frontend has had
/// time to persist, after which the DB dedup takes over and the marker is pruned.
#[derive(Default)]
struct DispatchedDms(HashMap<(String, String), Instant>);

impl DispatchedDms {
    /// Returns `true` if this `(peer, msg_id)` was already dispatched. Otherwise
    /// records it at `now` and returns `false`.
    fn mark(&mut self, peer_id: &str, msg_id: &str, now: Instant) -> bool {
        self.0
            .insert((peer_id.to_string(), msg_id.to_string()), now)
            .is_some()
    }

    /// Drop markers older than `ttl` so the set stays bounded. A message whose
    /// marker is gone is either long persisted (DB dedup still catches a replay)
    /// or was never a real message; either way it needs no in-memory guard.
    fn prune(&mut self, now: Instant, ttl: Duration) {
        self.0
            .retain(|_, dispatched| now.saturating_duration_since(*dispatched) <= ttl);
    }
}

static DISPATCHED_DMS: OnceLock<Mutex<DispatchedDms>> = OnceLock::new();

/// How long a dispatch marker is kept before the flush tick prunes it. Two
/// flush intervals comfortably exceed the time for a frontend to persist an
/// event while still bounding the set to recently seen messages.
const DM_DISPATCH_MARKER_TTL: Duration = Duration::from_secs(60);

fn dispatched_dms() -> &'static Mutex<DispatchedDms> {
    DISPATCHED_DMS.get_or_init(|| Mutex::new(DispatchedDms::default()))
}

/// Atomically check-and-mark an incoming DM as dispatched. Returns `true` when
/// it is a replay of one already dispatched (and must therefore be dropped).
fn dm_already_dispatched(peer_id: &str, msg_id: &str) -> bool {
    dispatched_dms()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .mark(peer_id, msg_id, Instant::now())
}

/// Handle one inbound DM request: open a sealed body, drop control/replay
/// traffic, dispatch real messages, start any attachment pull, and ack once.
async fn handle_dm_request(
    mut request: crate::DirectMessage,
    peer: &libp2p::PeerId,
    peer_id_str: String,
    channel: libp2p::request_response::ResponseChannel<crate::DirectMessage>,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    // US-042: open a sealed body *before* any control check — the plaintext
    // fields of an envelope are deliberately empty. A malformed/foreign
    // envelope is acked (so it is not retransmitted) and never reaches a UI.
    let encrypted = match open_encrypted_request(&mut request) {
        Ok(encrypted) => encrypted,
        Err(e) => {
            p2plog_debug(format!("Dropped sealed DM we could not open: {e}"));
            send_dm_ack(swarm, channel, request.msg_id);
            return;
        }
    };
    // Attachment-only and reply-only DMs are real messages despite empty
    // content (files lane §12 / US-006): a reply's quote snapshot is user
    // content, so the nickname/greeting paths only apply when neither rides.
    if request.content.trim().is_empty()
        && request.attachment.is_none()
        && request.reply_to.is_none()
    {
        handle_empty_dm_control(&request, &peer_id_str, event_tx).await;
        send_dm_ack(swarm, channel, request.msg_id);
        return;
    }
    // Store-and-forward: a reconnect replay can deliver the same sealed
    // envelope twice. Drop a DM we already persisted but still ack it.
    // `dm_already_dispatched` also marks this id as in-flight, closing the
    // window between this check and the frontend's async persist: a replay in
    // that window is caught by the marker before the DB row exists. Short-circuit
    // means a DB-confirmed duplicate is never marked, and a fresh id is marked
    // exactly once before dispatch.
    if let Some(id) = request.msg_id.clone()
        && (crate::messages::incoming_dm_exists(&peer_id_str, &id)
            || dm_already_dispatched(&peer_id_str, &id))
    {
        p2plog_debug(format!("Dropped duplicate DM {id} from {peer_id_str}"));
        send_dm_ack(swarm, channel, Some(id));
        return;
    }
    let ack_for = request.msg_id.clone();
    let attachment = request.attachment.clone();
    let local_peer = swarm.local_peer_id().to_string();
    dispatch_incoming_dm(request, peer_id_str, local_peer, encrypted, event_tx).await;
    // Attachments (files lane §12) are pulled from the sender of the DM — the
    // peer that has the blob staged to serve it.
    if let Some(attachment) = attachment.as_ref() {
        maybe_start_fetch(peer, attachment, swarm);
    }
    send_dm_ack(swarm, channel, ack_for);
}

/// Handle an ack from the recipient: clear the outbox row and notify the UI.
async fn handle_dm_response(
    response: crate::DirectMessage,
    peer_id_str: &str,
    event_tx: &mpsc::Sender<SwarmEvent>,
) {
    if let Some(ack_for) = response.ack_for {
        // Store-and-forward: the recipient confirmed receipt, so the sealed
        // envelope is no longer needed. Best-effort — a missing row (e.g. a
        // broadcast receipt) is a harmless no-op.
        if let Err(e) = crate::dm_outbox::remove(peer_id_str, &ack_for) {
            p2plog_debug(format!("Failed to clear outbox row for ack {ack_for}: {e}"));
        }
        let _ = event_tx
            .send(SwarmEvent::Receipt {
                peer_id: peer_id_str.to_string(),
                ack_for,
                received_at: response.received_at,
            })
            .await;
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
            handle_dm_request(request, &peer, peer_id_str, channel, event_tx, swarm).await;
        }
        libp2p::request_response::Message::Response { response, .. } => {
            handle_dm_response(response, &peer_id_str, event_tx).await;
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
        reply_to: None,
        encrypted: None,
    }
}

/// Verify an incoming claim about **our** device and, only on success, bind it.
///
/// The full enforcement order (about-us check, root materialization without
/// display data, signature verification, binding, then display seams) lives in
/// [`crate::identity::receive_device_binding_claim`] so it can be tested with a
/// plain DB instead of a live swarm. This wrapper only supplies the local
/// identity material the claim must match.
fn verify_and_bind_claim(
    request: &crate::behavior::DeviceBindingRequest,
    sender: &libp2p::PeerId,
    swarm: &Swarm<AppBehaviour>,
) -> color_eyre::Result<String> {
    let local_peer_id = swarm.local_peer_id().to_string();
    let local_pubkey = crate::get_libp2p_identity()?.public().encode_protobuf();
    crate::identity::receive_device_binding_claim(
        request,
        &sender.to_string(),
        &local_peer_id,
        &local_pubkey,
    )
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

/// Whether an inbound call signal may tear down the live media session.
///
/// Only a `reject`/`end` from the session's own peer echoing its own
/// caller-minted `call_id` ends the media. Both frontends ignore signals with a
/// foreign/stale id, so clearing the session for those would silently kill a
/// healthy call's audio while the UI still shows "In call". Kept pure so the
/// gate is directly testable without a live `Swarm`.
#[must_use]
fn media_ends_for_signal(
    media: Option<&MediaSession>,
    kind: &str,
    call_id: &str,
    peer_id: &str,
) -> bool {
    matches!(kind, "reject" | "end")
        && media.is_some_and(|session| session.is_for(call_id, peer_id))
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
    media: &mut Option<MediaSession>,
) {
    let peer_id_str = peer.to_string();
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            // Decide before the match consumes `peer_id_str` into the event.
            let ends_media = media_ends_for_signal(
                media.as_ref(),
                request.kind.as_str(),
                &request.call_id,
                &peer_id_str,
            );
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
            // A rejected or ended call has no audio left to carry. Drop the
            // session only when the signal belongs to it; a foreign/stale
            // reject or end must not silently kill a healthy call's audio.
            if ends_media {
                *media = None;
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

/// Receive one voice frame (calls lane, US-044).
///
/// A frame is only accepted while a media session is open, from the exact peer
/// and call that session was opened for. Every frame gets an ack so the sender
/// can distinguish a live receiver from a dead one; decoded PCM is forwarded to
/// the frontend once the jitter buffer has a complete, in-order block. Nothing
/// here is logged per frame — at 50 Hz that would drown the log tab.
async fn handle_call_media(
    message: libp2p::request_response::Message<AudioFrame, FrameAck>,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
    media: &mut Option<MediaSession>,
) {
    let libp2p::request_response::Message::Request {
        request, channel, ..
    } = message
    else {
        // The ack to one of our own frames carries no application data.
        return;
    };
    let call_id = request.call_id.clone();
    let peer_id_str = peer.to_string();
    let (accepted, pcm) = match media.as_mut() {
        Some(session) if session.is_for(&call_id, &peer_id_str) => {
            let accepted = session.accept(request);
            (accepted, session.take_pcm())
        }
        _ => (false, None),
    };
    if let Some(pcm) = pcm {
        let _ = event_tx
            .send(SwarmEvent::CallMediaFrame {
                call_id,
                peer_id: peer_id_str,
                pcm,
            })
            .await;
    }
    let ack = FrameAck {
        ok: accepted,
        reason: (!accepted).then(|| "no active media session for this frame".into()),
    };
    let _ = swarm.behaviour_mut().call_media.send_response(channel, ack);
}

/// Receive a transient typing signal and surface it to both frontends.
///
/// The response only closes the request stream; typing state is never persisted
/// and has no delivery semantics beyond the frontend's short TTL.
async fn handle_typing_signal(
    message: libp2p::request_response::Message<
        crate::behavior::TypingSignal,
        crate::behavior::TypingAck,
    >,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            let _ = event_tx
                .send(SwarmEvent::Typing {
                    peer_id: peer.to_string(),
                    is_typing: request.is_typing,
                })
                .await;
            let _ = swarm
                .behaviour_mut()
                .typing
                .send_response(channel, crate::behavior::TypingAck { ok: true });
        }
        libp2p::request_response::Message::Response { .. } => {}
    }
}

/// Apply one inbound reaction change, returning whether it was accepted.
///
/// Kept separate from the transport handler so the ack can be driven by the
/// real persistence result: a rejected signal (bad emoji, failed write) must
/// not be acked as `ok: true`, which would leave the sender believing a chip
/// was stored when it was not.
fn apply_inbound_reaction(msg_id: &str, actor: &str, emoji: &str, remove: bool) -> bool {
    match crate::reactions::apply(msg_id, actor, emoji, remove) {
        Ok(_) => true,
        Err(err) => {
            p2plog_debug(format!("Rejected reaction from {actor}: {err}"));
            false
        }
    }
}

/// Apply an inbound reaction (US-008) and surface the updated summary to both
/// frontends. The reactor is taken from the connection's peer id, never the
/// signal's `actor` field, so a peer cannot post a reaction on another's behalf.
async fn handle_reaction_signal(
    message: libp2p::request_response::Message<
        crate::behavior::ReactionSignal,
        crate::behavior::ReactionAck,
    >,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    match message {
        libp2p::request_response::Message::Request {
            request, channel, ..
        } => {
            let actor = peer.to_string();
            let accepted =
                apply_inbound_reaction(&request.msg_id, &actor, &request.emoji, request.remove);
            if accepted {
                let _ = event_tx
                    .send(SwarmEvent::Reaction {
                        msg_id: request.msg_id,
                    })
                    .await;
            }
            let _ = swarm
                .behaviour_mut()
                .reactions
                .send_response(channel, crate::behavior::ReactionAck { ok: accepted });
        }
        libp2p::request_response::Message::Response { .. } => {}
    }
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
    build_broadcast_message_with_reply(content, nickname, msg_id, attachment, None)
}

/// Build a `BroadcastMessage` carrying an immutable reply snapshot (US-006).
#[must_use]
pub fn build_broadcast_message_with_reply(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
) -> BroadcastMessage {
    BroadcastMessage {
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
        attachment,
        reply_to,
    }
}

/// Build a `GroupMessage` carrying an immutable reply snapshot (US-006).
#[must_use]
pub fn build_group_message_with_reply(
    group_id: String,
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
) -> crate::GroupMessage {
    crate::GroupMessage {
        group_id,
        content,
        sent_at: Some(current_timestamp()),
        nickname,
        msg_id,
        attachment,
        reply_to,
    }
}

fn handle_command(
    cmd: SwarmCommand,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &str,
    media: &mut Option<MediaSession>,
) {
    match cmd {
        SwarmCommand::Publish {
            content,
            nickname,
            msg_id,
            attachment,
            reply_to,
        } => publish_broadcast(
            content, nickname, msg_id, attachment, reply_to, swarm, topic,
        ),
        SwarmCommand::SendDm {
            peer_id,
            content,
            nickname,
            msg_id,
            ack_for,
            attachment,
            reply_to,
        } => send_dm(
            &peer_id,
            crate::DirectMessage {
                content,
                timestamp: chrono::Utc::now().timestamp(),
                sent_at: Some(current_timestamp()),
                nickname,
                msg_id,
                ack_for,
                received_at: None,
                group_id: None,
                attachment,
                reply_to,
                encrypted: None,
            },
            swarm,
        ),
        SwarmCommand::PublishGroup {
            group_id,
            content,
            nickname,
            msg_id,
            attachment,
            reply_to,
        } => publish_group(
            content, nickname, msg_id, group_id, attachment, reply_to, swarm,
        ),
        SwarmCommand::SubscribeGroup { group_id } => subscribe_to_group(&group_id, swarm),
        SwarmCommand::SendDeviceBinding { peer_id, request } => {
            send_device_binding(&peer_id, request, swarm);
        }
        SwarmCommand::SendCallSignal { peer_id, signal } => {
            send_call_signal(&peer_id, *signal, swarm);
        }
        SwarmCommand::StartCallMedia { call_id, peer_id } => {
            if peer_id.parse::<libp2p::PeerId>().is_err() {
                p2plog_error(format!("Invalid peer id for call media session: {peer_id}"));
                return;
            }
            p2plog_debug(format!(
                "Started call media session {call_id} with {peer_id}"
            ));
            *media = Some(MediaSession::new(call_id, peer_id));
        }
        SwarmCommand::StopCallMedia => {
            p2plog_debug("Stopped call media session");
            *media = None;
        }
        SwarmCommand::SendCallMedia { call_id, pcm } => {
            send_call_media(&call_id, &pcm, swarm, media);
        }
        SwarmCommand::SetCallMediaMute { muted } => {
            if let Some(session) = media.as_mut() {
                session.set_muted(muted);
            }
        }
        SwarmCommand::SendTyping { peer_id, is_typing } => {
            send_typing(&peer_id, is_typing, swarm);
        }
        SwarmCommand::SendReaction { peer_ids, signal } => {
            send_reaction(&peer_ids, signal.as_ref(), swarm);
        }
        SwarmCommand::UnsubscribeGroup { group_id } => unsubscribe_from_group(&group_id, swarm),
        SwarmCommand::Dial { address } => dial_peer(&address, swarm),
        SwarmCommand::DisconnectPeer { peer_id } => disconnect_peer(&peer_id, swarm),
        cmd @ (SwarmCommand::ListShares { .. }
        | SwarmCommand::RequestShareManifest { .. }
        | SwarmCommand::SendShareOffer { .. }
        | SwarmCommand::PublishShareSnapshot { .. }) => handle_share_command(cmd, swarm),
    }
}

/// Dispatch the share-lane subset of [`SwarmCommand`]. Split out of the main
/// command handler so both stay inside the project's per-function lint budget.
fn handle_share_command(cmd: SwarmCommand, swarm: &mut Swarm<AppBehaviour>) {
    match cmd {
        SwarmCommand::ListShares { peer_id } => send_share_request(
            &peer_id,
            crate::behavior::ShareRequest {
                kind: "list".to_string(),
                ..Default::default()
            },
            swarm,
        ),
        SwarmCommand::RequestShareManifest { peer_id, share_id } => send_share_request(
            &peer_id,
            crate::behavior::ShareRequest {
                kind: "manifest".to_string(),
                share_id,
                ..Default::default()
            },
            swarm,
        ),
        SwarmCommand::SendShareOffer { peer_ids, offer } => {
            let request = crate::behavior::ShareRequest {
                kind: "offer".to_string(),
                offer: Some(*offer),
                ..Default::default()
            };
            for peer_id in peer_ids {
                send_share_request(&peer_id, request.clone(), swarm);
            }
        }
        SwarmCommand::PublishShareSnapshot {
            peer_id,
            base_revision,
            manifest,
        } => send_share_request(
            &peer_id,
            crate::behavior::ShareRequest {
                kind: "publish".to_string(),
                share_id: manifest.share_id.clone(),
                base_revision,
                manifest: Some(*manifest),
                offer: None,
            },
            swarm,
        ),
        other => p2plog_error(format!("unexpected non-share command: {other:?}")),
    }
}

/// Encode and send one PCM block on the active media session (calls lane).
///
/// A stale call id (a UI that queued frames just before a hang-up) is ignored
/// rather than leaking audio into the next call. Likewise a muted session or an
/// empty block produces no frame, so silence costs no bandwidth.
fn send_call_media(
    call_id: &str,
    pcm: &[i16],
    swarm: &mut Swarm<AppBehaviour>,
    media: &mut Option<MediaSession>,
) {
    let Some(session) = media.as_mut() else {
        return;
    };
    if session.call_id != call_id {
        return;
    }
    let Ok(peer) = session.peer_id.parse::<libp2p::PeerId>() else {
        return;
    };
    let Some(frame) = session.next_frame(pcm) else {
        return;
    };
    let _ = swarm.behaviour_mut().call_media.send_request(&peer, frame);
}

fn publish_broadcast(
    content: String,
    nickname: Option<String>,
    msg_id: Option<String>,
    attachment: Option<crate::files::MessageAttachment>,
    reply_to: Option<crate::ReplyRef>,
    swarm: &mut Swarm<AppBehaviour>,
    topic: &str,
) {
    let msg = build_broadcast_message_with_reply(content, nickname, msg_id, attachment, reply_to);
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

/// Close the live connection to a peer (blocklist toggle, US-054).
fn disconnect_peer(peer_id: &str, swarm: &mut Swarm<AppBehaviour>) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
        let _ = swarm.disconnect_peer_id(peer);
        p2plog_debug(format!("Disconnected blocked peer {peer_id}"));
    }
}

/// Dial a peer by multiaddr (manual peer address entry, US-053).
fn dial_peer(address: &str, swarm: &mut Swarm<AppBehaviour>) {
    match address.trim().parse::<libp2p::Multiaddr>() {
        Ok(addr) => {
            if crate::network_config::multiaddr_peer_id(&addr)
                .is_some_and(|peer| crate::blocklist::is_blocked(&peer.to_string()))
            {
                p2plog_debug(format!("Refusing to dial blocked peer in {addr}"));
                return;
            }
            match swarm.dial(addr.clone()) {
                Ok(()) => p2plog_debug(format!("Dialing {addr}")),
                Err(e) => p2plog_error(format!("Failed to dial {addr}: {e}")),
            }
        }
        Err(e) => p2plog_error(format!("Invalid multiaddr {address}: {e}")),
    }
}

/// Retry a batch of sealed envelopes from the store-and-forward outbox.
///
/// Best-effort: libp2p queues the request when the peer is connected (or has a
/// known address) and fails silently otherwise; the row stays until the ack
/// removes it. A malformed envelope is dropped; a blocked recipient is skipped
/// (its row eventually expires) so neither is retried forever.
fn retry_outbox_rows(
    rows: Vec<crate::generated::models_queryable::DmOutboxRow>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    for row in rows {
        let Ok(peer) = row.recipient_peer_id.parse::<libp2p::PeerId>() else {
            continue;
        };
        if crate::blocklist::is_blocked(&peer.to_string()) {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<crate::DirectMessage>(&row.envelope_json) else {
            p2plog_debug(format!("Dropping malformed outbox envelope {}", row.msg_id));
            let _ = crate::dm_outbox::remove(&row.recipient_peer_id, &row.msg_id);
            continue;
        };
        swarm
            .behaviour_mut()
            .request_response
            .send_request(&peer, msg);
        if let Err(e) = crate::dm_outbox::record_attempt(row.id) {
            p2plog_debug(format!("Failed to record DM retry attempt: {e}"));
        }
    }
}

/// Retry every pending sealed DM and prune expired rows (periodic sweep).
fn flush_pending_outbox(swarm: &mut Swarm<AppBehaviour>) {
    match crate::dm_outbox::prune_expired() {
        Ok(0) | Err(_) => {}
        Ok(n) => p2plog_debug(format!("Pruned {n} expired DMs from outbox")),
    }
    match crate::dm_outbox::pending() {
        Ok(rows) => retry_outbox_rows(rows, swarm),
        Err(e) => p2plog_debug(format!("Failed to load DM outbox for retry: {e}")),
    }
}

fn send_dm(peer_id: &str, mut msg: crate::DirectMessage, swarm: &mut Swarm<AppBehaviour>) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
        if crate::blocklist::is_blocked(&peer.to_string()) {
            p2plog_debug(format!("Refusing to send to blocked peer {peer_id}"));
            return;
        }
        // US-042: seal user traffic for the recipient device. The routing
        // fields and control traffic stay in the clear; a sealed message carries
        // no plaintext body/nickname/attachment/reply snapshot.
        crate::e2e::seal_dm_payload(&mut msg, &peer);
        // Store-and-forward: park the sealed envelope until an ack arrives.
        // Only sealed user DMs are queued — acks, nickname announcements and
        // DM-lane group control stay plaintext and are intentionally not cached.
        if msg.encrypted.is_some()
            && let (Ok(json), Some(id)) = (serde_json::to_string(&msg), msg.msg_id.as_ref())
        {
            match crate::dm_outbox::queue(peer_id, id, &json, crate::dm_outbox::DEFAULT_TTL_SECS) {
                Ok(true) => p2plog_debug(format!("Queued sealed DM {id} for {peer_id}")),
                Ok(false) => {}
                Err(e) => p2plog_debug(format!("Failed to queue sealed DM {id}: {e}")),
            }
        }
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
    reply_to: Option<crate::ReplyRef>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let topic = gossipsub::IdentTopic::new(crate::groups::group_topic(&group_id));
    let msg =
        build_group_message_with_reply(group_id, content, nickname, msg_id, attachment, reply_to);
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

/// The gossipsub topics a group membership spans: the main chat topic and the
/// store-and-forward backfill topic (§11.5). Both [`subscribe_to_group`] and
/// [`unsubscribe_from_group`] derive their topics from this one list, so
/// leaving a group can never forget a topic that joining it subscribed to.
fn group_topics(group_id: &str) -> [gossipsub::IdentTopic; 2] {
    [
        gossipsub::IdentTopic::new(crate::groups::group_topic(group_id)),
        gossipsub::IdentTopic::new(crate::groups::group_backfill_topic(group_id)),
    ]
}

fn subscribe_to_group(group_id: &str, swarm: &mut Swarm<AppBehaviour>) {
    let [topic, backfill_topic] = group_topics(group_id);
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
        if crate::blocklist::is_blocked(&peer.to_string()) {
            p2plog_debug(format!("Refusing to send to blocked peer {peer_id}"));
            return;
        }
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
        if crate::blocklist::is_blocked(&peer.to_string()) {
            p2plog_debug(format!("Refusing to send to blocked peer {peer_id}"));
            return;
        }
        // See `send_device_binding`: dispatch is fire-and-forget from here;
        // protocol failures surface as `OutboundFailure` behaviour events.
        let _ = swarm
            .behaviour_mut()
            .call_signaling
            .send_request(&peer, signal);
    }
}

/// Send a transient typing state on the dedicated lane. Blocked peers are
/// skipped at the swarm boundary like every other outbound lane.
fn send_typing(peer_id: &str, is_typing: bool, swarm: &mut Swarm<AppBehaviour>) {
    if let Ok(peer) = peer_id.parse::<libp2p::PeerId>() {
        if crate::blocklist::is_blocked(&peer.to_string()) {
            p2plog_debug(format!(
                "Refusing to send typing signal to blocked peer {peer_id}"
            ));
            return;
        }
        let _ = swarm
            .behaviour_mut()
            .typing
            .send_request(&peer, crate::behavior::TypingSignal::new(is_typing));
    }
}

/// Fan a reaction change out to every peer that may hold the message (US-008).
/// Blocked peers are skipped at the swarm boundary like every other lane.
fn send_reaction(
    peer_ids: &[String],
    signal: &crate::behavior::ReactionSignal,
    swarm: &mut Swarm<AppBehaviour>,
) {
    for peer_id in peer_ids {
        let Ok(peer) = peer_id.parse::<libp2p::PeerId>() else {
            continue;
        };
        if crate::blocklist::is_blocked(&peer.to_string()) {
            p2plog_debug(format!(
                "Refusing to send reaction to blocked peer {peer_id}"
            ));
            continue;
        }
        let _ = swarm
            .behaviour_mut()
            .reactions
            .send_request(&peer, signal.clone());
    }
}

/// Dispatch one share-lane message. Requests are ACL-checked; responses feed
/// offers/manifests back into the canonical share store and the file lane.
async fn handle_share_lane(
    message: libp2p::request_response::Message<
        crate::behavior::ShareRequest,
        crate::behavior::ShareResponse,
    >,
    peer: libp2p::PeerId,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    use libp2p::request_response::Message as RrMessage;
    let peer_id_str = peer.to_string();
    match message {
        RrMessage::Request {
            request, channel, ..
        } => {
            let response = handle_share_request(&request, &peer_id_str, event_tx).await;
            let _ = swarm
                .behaviour_mut()
                .shares
                .send_response(channel, response);
        }
        RrMessage::Response { response, .. } => {
            handle_share_response(&response, &peer_id_str, event_tx, swarm).await;
        }
    }
}

/// Build a rejected share response carrying a human-readable reason.
fn share_error(
    base: crate::behavior::ShareResponse,
    reason: &str,
) -> crate::behavior::ShareResponse {
    crate::behavior::ShareResponse {
        ok: false,
        reason: Some(reason.to_string()),
        ..base
    }
}

/// Answer an inbound share request. Every ACL decision uses the connection's
/// peer id, so a claimed user id in the payload has no effect.
async fn handle_share_request(
    request: &crate::behavior::ShareRequest,
    peer_id: &str,
    event_tx: &mpsc::Sender<SwarmEvent>,
) -> crate::behavior::ShareResponse {
    use crate::behavior::ShareResponse;
    let base = ShareResponse {
        kind: request.kind.clone(),
        share_id: request.share_id.clone(),
        ..Default::default()
    };
    match request.kind.as_str() {
        "offer" => {
            let Some(offer) = request.offer.clone() else {
                return share_error(base, "missing share offer");
            };
            if offer.share_id.is_empty() || crate::shares::validate_share_name(&offer.name).is_err()
            {
                return share_error(base, "invalid share offer");
            }
            let _ = event_tx
                .send(SwarmEvent::ShareOffered {
                    peer_id: peer_id.to_string(),
                    offer,
                })
                .await;
            ShareResponse { ok: true, ..base }
        }
        "list" => {
            let offers = match crate::shares::list_shares() {
                Ok(shares) => shares
                    .into_iter()
                    .filter(|share| crate::shares::can_read(&share.share_id, peer_id))
                    .filter_map(|share| {
                        crate::shares::offer_for_share(&share.share_id)
                            .ok()
                            .flatten()
                    })
                    .collect(),
                Err(error) => {
                    p2plog_error(format!("failed to list shares: {error}"));
                    return share_error(base, "failed to list shares");
                }
            };
            ShareResponse {
                ok: true,
                offers,
                ..base
            }
        }
        "manifest" => {
            if !crate::shares::can_read(&request.share_id, peer_id) {
                return share_error(base, "access denied");
            }
            match crate::shares::load_manifest(&request.share_id) {
                Ok(manifest) => ShareResponse {
                    ok: true,
                    revision: manifest.revision,
                    manifest: Some(manifest),
                    ..base
                },
                Err(error) => share_error(base, &format!("no manifest: {error}")),
            }
        }
        "publish" => {
            let Some(manifest) = request.manifest.clone() else {
                return share_error(base, "missing manifest");
            };
            match crate::shares::accept_submitted_manifest(
                &request.share_id,
                peer_id,
                request.base_revision,
                &manifest,
            ) {
                Ok(stats) => {
                    let _ = event_tx
                        .send(SwarmEvent::ShareUpdated {
                            peer_id: peer_id.to_string(),
                            share_id: request.share_id.clone(),
                            revision: request.base_revision.saturating_add(1),
                            written: stats.written,
                            removed: stats.removed,
                        })
                        .await;
                    ShareResponse {
                        ok: true,
                        revision: request.base_revision.saturating_add(1),
                        ..base
                    }
                }
                Err(error) => share_error(base, &error.to_string()),
            }
        }
        _ => share_error(
            base,
            &format!("unknown share request kind: {}", request.kind),
        ),
    }
}

/// Consume a share-lane response: emit offers, persist a manifest and pull its
/// blobs, or ask again for the fresh manifest after our write was accepted.
async fn handle_share_response(
    response: &crate::behavior::ShareResponse,
    peer_id: &str,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    use crate::behavior::ShareRequest;
    if !response.ok {
        let _ = event_tx
            .send(SwarmEvent::ShareRejected {
                peer_id: peer_id.to_string(),
                share_id: response.share_id.clone(),
                reason: response
                    .reason
                    .clone()
                    .unwrap_or_else(|| "share request rejected".to_string()),
            })
            .await;
        return;
    }
    match response.kind.as_str() {
        "list" => {
            for offer in &response.offers {
                if offer.share_id.is_empty()
                    || crate::shares::validate_share_name(&offer.name).is_err()
                {
                    continue;
                }
                let _ = event_tx
                    .send(SwarmEvent::ShareOffered {
                        peer_id: peer_id.to_string(),
                        offer: offer.clone(),
                    })
                    .await;
            }
        }
        "manifest" => {
            handle_share_manifest_response(response, peer_id, event_tx, swarm).await;
        }
        "publish" => {
            // Our submission landed: pull the owner's fresh manifest so all
            // devices converge on the new revision.
            send_share_request(
                peer_id,
                ShareRequest {
                    kind: "manifest".to_string(),
                    share_id: response.share_id.clone(),
                    ..Default::default()
                },
                swarm,
            );
        }
        _ => {}
    }
}

/// Persist a received manifest (materialising the share row if its offer was
/// never seen), apply it when all its blobs are already local, and start pulls
/// for the rest.
async fn handle_share_manifest_response(
    response: &crate::behavior::ShareResponse,
    peer_id: &str,
    event_tx: &mpsc::Sender<SwarmEvent>,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let Some(manifest) = response.manifest.as_ref() else {
        return;
    };
    if manifest.share_id != response.share_id || crate::shares::validate_manifest(manifest).is_err()
    {
        return;
    }
    let existing = if let Some(share) = crate::shares::get_share(&manifest.share_id).ok().flatten()
    {
        share
    } else {
        // A manifest can arrive before the UI materialised its offer. Record a
        // minimal local row so the snapshot has a home.
        let owner = crate::identity::find_user_id_for_peer(peer_id)
            .ok()
            .flatten()
            .unwrap_or_else(|| crate::shares::unknown_owner_id(&manifest.share_id));
        let offer = crate::shares::ShareOffer {
            share_id: manifest.share_id.clone(),
            name: manifest.name.clone(),
            kind: crate::shares::ShareKind::Private,
            revision: manifest.revision,
            public_write: false,
            owner_user_id: Some(owner),
        };
        let Some(share) = crate::shares::accept_offer(&offer, None).ok() else {
            return;
        };
        share
    };
    let previous = existing.manifest;
    if crate::shares::save_manifest(&manifest.share_id, manifest).is_err() {
        return;
    }
    let queued = crate::shares::queue_apply(peer_id.to_string(), previous, manifest.clone());
    for applied in queued.applied {
        let _ = event_tx
            .send(SwarmEvent::ShareUpdated {
                peer_id: applied.peer_id,
                share_id: applied.share_id,
                revision: applied.revision,
                written: applied.stats.written,
                removed: applied.stats.removed,
            })
            .await;
    }
    let Ok(peer) = peer_id.parse::<libp2p::PeerId>() else {
        return;
    };
    for file_id in queued.missing {
        let (name, size, mime) = manifest
            .entries
            .iter()
            .find(|entry| entry.file_id == file_id)
            .map_or_else(
                || (file_id.clone(), 0_u64, None),
                |entry| (entry.path.clone(), entry.size, entry.mime.clone()),
            );
        maybe_start_fetch(
            &peer,
            &crate::files::MessageAttachment {
                file_id,
                name,
                size,
                mime,
            },
            swarm,
        );
    }
}

/// Send a share-lane request, skipping blocked peers like every other lane.
fn send_share_request(
    peer_id: &str,
    request: crate::behavior::ShareRequest,
    swarm: &mut Swarm<AppBehaviour>,
) {
    let Ok(peer) = peer_id.parse::<libp2p::PeerId>() else {
        return;
    };
    if crate::blocklist::is_blocked(&peer.to_string()) {
        p2plog_debug(format!(
            "Refusing to send share request to blocked peer {peer_id}"
        ));
        return;
    }
    let _ = swarm.behaviour_mut().shares.send_request(&peer, request);
}

fn unsubscribe_from_group(group_id: &str, swarm: &mut Swarm<AppBehaviour>) {
    // `subscribe_to_group` joins both topics (§11.5), so leaving must drop
    // both: leaving only the main topic kept stale mesh membership for a
    // group whose row is already gone.
    let [topic, backfill_topic] = group_topics(group_id);
    if swarm.behaviour_mut().gossipsub.unsubscribe(&topic) {
        p2plog_debug(format!("Unsubscribed from group {group_id}"));
    } else {
        p2plog_debug(format!("Was not subscribed to group {group_id}"));
    }
    if swarm.behaviour_mut().gossipsub.unsubscribe(&backfill_topic) {
        p2plog_debug(format!(
            "Unsubscribed from backfill topic for group {group_id}"
        ));
    }
}

/// Public IPFS bootstrap nodes, used only when the operator opts in with
/// `P2P_PUBLIC_DHT` (or lists their own peers).
#[cfg(feature = "kad")]
const PUBLIC_BOOTSTRAP_PEERS: &str = concat!(
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmNnooDu7bfu53Fn2A9xSfDMGcR6fYSdrNj4HgjCZNnTE5,",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmQCU2EcMqAqQPR2i9bChDtGNJ9GD89Nm3HLQQpXePmCRp,",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb,",
    "/dnsaddr/bootstrap.libp2p.io/p2p/QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt,"
);

/// Bootstrap multiaddrs to dial at node start.
///
/// Precedence, highest first:
///
/// 1. `P2P_BOOTSTRAP` — when the variable is **set** it wins outright, even
///    when empty (that is the documented way to disable bootstrapping and
///    ignore the persisted list).
/// 2. The persisted `network_peers` list of kind `bootstrap`, editable from
///    Settings in both frontends.
/// 3. The public IPFS bootstrap nodes, only with `P2P_PUBLIC_DHT=1` — a
///    default install stays LAN-only (mDNS) and generates no global DHT traffic.
#[cfg(feature = "kad")]
fn bootstrap_addrs() -> Vec<libp2p::Multiaddr> {
    // LAN-only mode (US-051): no internet bootstrap, regardless of env or
    // persisted entries. mDNS and manual direct dials still work.
    if crate::settings::lan_only() {
        return Vec::new();
    }
    if let Ok(value) = std::env::var("P2P_BOOTSTRAP") {
        return parse_bootstrap_addrs(&value);
    }
    let stored = crate::network_config::stored_bootstrap_addrs();
    if !stored.is_empty() {
        return stored;
    }
    let public_enabled = std::env::var("P2P_PUBLIC_DHT")
        .is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"));
    if public_enabled {
        return parse_bootstrap_addrs(PUBLIC_BOOTSTRAP_PEERS);
    }
    Vec::new()
}

/// Parse a comma-separated bootstrap list, dropping entries that do not parse.
#[cfg(feature = "kad")]
fn parse_bootstrap_addrs(list: &str) -> Vec<libp2p::Multiaddr> {
    let mut addrs: Vec<libp2p::Multiaddr> = Vec::new();
    for addr in list.split(',') {
        let addr = addr.trim();
        if addr.is_empty() {
            continue;
        }
        if let Ok(ma) = addr.parse::<libp2p::Multiaddr>() {
            if !addrs.contains(&ma) {
                addrs.push(ma);
            }
        } else {
            p2plog_debug(format!("Invalid bootstrap multiaddr: {addr}"));
        }
    }
    addrs
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
        // instead of staying LAN-only. See `bootstrap_addrs` for the source
        // precedence (env var → persisted list → opt-in public IPFS nodes).
        // The Kademlia behaviour is forced into client mode (see behavior.rs),
        // so we route through the public DHT without storing/serving its
        // records.
        #[cfg(feature = "kad")]
        {
            for addr in bootstrap_addrs() {
                let _ = swarm.dial(addr);
            }
        }
        // Circuit-relay reservations (US-050 follow-up 2): listen on the
        // relayed address of every relay configured in `P2P_RELAY`. That both
        // dials the relay (the relay transport dials it if not connected) and
        // requests a reservation; the listener renews it until it closes.
        let mut relays = RelayState::new();
        relays.configure(&mut swarm);

        // Store-and-forward: periodically retry sealed DMs that never got an
        // ack and prune expired rows. The first tick fires immediately (useful
        // after a restart); `Skip` keeps a busy node from stacking catch-ups.
        let mut flush_ticker = tokio::time::interval(std::time::Duration::from_secs(30));
        flush_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut announced = AnnouncedConnections::default();

        // One media session at a time, owned by the swarm loop like the call
        // signaling state: frontends only send commands and receive decoded
        // PCM events, they never hold a session of their own.
        let mut media: Option<MediaSession> = None;
        loop {
            let event = tokio::select! {
                swarm_event = swarm.select_next_some() => Some(Event::Swarm(Box::new(swarm_event))),
                Some(cmd) = cmd_rx.recv() => Some(Event::Command(Box::new(cmd))),
                _ = flush_ticker.tick() => Some(Event::Flush),
                else => None,
            };

            match event {
                Some(Event::Swarm(swarm_event)) => {
                    handle_swarm_event(
                        *swarm_event,
                        &event_tx,
                        &mut swarm,
                        &mut relays,
                        &mut announced,
                        &mut media,
                    )
                    .await;
                }
                Some(Event::Command(cmd)) => {
                    handle_command(*cmd, &mut swarm, &topic, &mut media);
                }
                Some(Event::Flush) => {
                    flush_pending_outbox(&mut swarm);
                    // Backstop for pulls whose request failed without a
                    // disconnect (an outbound timeout/protocol error produces
                    // no `"error"` response in the files lane).
                    abandon_idle_transfers(FILE_TRANSFER_IDLE_TIMEOUT);
                    // Dispatch markers only bridge the async-persist window;
                    // older ones are covered by the DB dedup and can be freed.
                    dispatched_dms()
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .prune(Instant::now(), DM_DISPATCH_MARKER_TTL);
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
