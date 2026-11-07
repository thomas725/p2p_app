//! Live network diagnostics: interfaces, reachability, relay and DHT state.
//!
//! Both frontends (TUI Settings tab and Flutter "Node & Network") render a
//! snapshot produced by [`snapshot`]. The swarm event loop feeds the live
//! fields through the `record_*` functions while it translates libp2p events;
//! keeping the state here means no frontend has to duplicate libp2p
//! bookkeeping, and the polled snapshot is always the single source of truth.
//!
//! Network interfaces (name, MAC, IPv4/IPv6) come from the OS via
//! [`network_interface`]; they are cached for a few seconds because both
//! frontends may render the snapshot on every frame.

use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use serde::Serialize;
use std::sync::{Mutex, OnceLock, PoisonError};

/// How long the OS interface list is cached, in seconds.
const INTERFACE_CACHE_TTL_SECS: f64 = 5.0;

/// Minimum interval between DHT announcements, in seconds.
const ANNOUNCE_INTERVAL_SECS: f64 = 60.0;

/// One OS network interface with its MAC and IP addresses.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceInfo {
    /// Interface name (`eth0`, `wlan0`, `lo`, …).
    pub name: String,
    /// MAC address when the OS reports one.
    pub mac: Option<String>,
    /// IPv4 addresses assigned to the interface.
    pub ipv4: Vec<String>,
    /// IPv6 addresses assigned to the interface.
    pub ipv6: Vec<String>,
    /// Whether this is a loopback (or otherwise not remotely reachable) device.
    pub internal: bool,
}

/// Everything the "network details" views show.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDetails {
    /// OS interfaces with MAC/IPv4/IPv6.
    pub interfaces: Vec<NetworkInterfaceInfo>,
    /// Addresses the swarm is listening on.
    pub listen_addrs: Vec<String>,
    /// Addresses confirmed reachable from outside (`AutoNAT`/identify).
    pub external_addrs: Vec<String>,
    /// Addresses observed but not yet confirmed.
    pub external_candidates: Vec<String>,
    /// Human-readable reachability class (public / relayed / private / unknown).
    pub reachability: String,
    /// `AutoNAT`'s verdict (`Public`, `Private`, `Unknown`).
    pub nat_status: String,
    /// Address `AutoNAT` confirmed as publicly reachable, when public.
    pub nat_public_addr: Option<String>,
    /// Successful `AutoNAT` dial-back probes this session.
    pub nat_probe_successes: u32,
    /// Failed `AutoNAT` dial-back probes this session.
    pub nat_probe_failures: u32,
    /// Hole-punch (`DCUtR`) attempts this session.
    pub hole_punch_attempts: u32,
    /// Successful hole punches this session.
    pub hole_punch_successes: u32,
    /// Relay peers we hold a reservation with.
    pub relay_reservations: Vec<String>,
    /// Last relay reservation status line.
    pub relay_status: String,
    /// Kademlia mode (`Client` / `Server`) — `Server` means routable.
    pub dht_mode: String,
    /// Whether the DHT routing table has at least one peer.
    pub dht_connected: bool,
    /// Number of peers in the DHT routing table.
    pub dht_routing_peers: u32,
    /// Successful bootstrap queries this session.
    pub dht_bootstrap_successes: u32,
    /// Failed bootstrap queries this session.
    pub dht_bootstrap_failures: u32,
    /// Whether our peer record was published to the DHT.
    pub dht_announced: bool,
    /// Last DHT announcement status line.
    pub dht_announce_status: String,
    /// Unix seconds of the last successful announcement.
    pub dht_announce_at: Option<f64>,
}

impl NetworkDetails {
    /// Addresses a remote peer could use to reach us, best first.
    #[must_use]
    pub fn reachable_addresses(&self) -> Vec<String> {
        let mut addrs: Vec<String> = self
            .external_addrs
            .iter()
            .filter(|addr| !is_loopback_addr(addr))
            .cloned()
            .collect();
        for addr in &self.listen_addrs {
            if !is_loopback_addr(addr) && !addrs.contains(addr) {
                addrs.push(addr.clone());
            }
        }
        addrs
    }

    /// Addresses to advertise in the DHT peer record.
    ///
    /// Behind NAT the only dialable addresses are the relayed circuits, so
    /// when `AutoNAT` says `Private` **and** we hold a relay reservation this
    /// narrows to those (announcing `LAN` addresses would produce a record
    /// remote peers cannot dial). Otherwise the normal external/listen list is
    /// used, with any circuit addresses appended — they are reachable for
    /// peers able to use the relay.
    #[must_use]
    pub fn announce_addresses(&self, self_peer_id: &str) -> Vec<String> {
        let circuits = self.relay_circuit_addresses(self_peer_id);
        if self.nat_status == "Private" && !self.relay_reservations.is_empty() {
            return circuits;
        }
        let mut addrs: Vec<String> = self
            .reachable_addresses()
            .into_iter()
            .filter(|addr| !addr.contains("/p2p-circuit"))
            .collect();
        for circuit in circuits {
            if !addrs.contains(&circuit) {
                addrs.push(circuit);
            }
        }
        addrs
    }

    /// Relay circuit addresses we can be reached on, completed with
    /// `/p2p/<self>`.
    ///
    /// The relay client confirms a reservation with the full
    /// `/…/p2p/<relay>/p2p-circuit/p2p/<self>` address (as an external
    /// address), while a raw circuit listener address can omit the trailing
    /// self id — every entry is completed here so the advertised form is
    /// always dialable.
    #[must_use]
    pub fn relay_circuit_addresses(&self, self_peer_id: &str) -> Vec<String> {
        let suffix = format!("/p2p/{self_peer_id}");
        let mut circuits: Vec<String> = Vec::new();
        for addr in self.external_addrs.iter().chain(&self.listen_addrs) {
            if !addr.contains("/p2p-circuit") {
                continue;
            }
            let full = if addr.ends_with(&suffix) {
                addr.clone()
            } else {
                format!("{addr}{suffix}")
            };
            if !circuits.contains(&full) {
                circuits.push(full);
            }
        }
        circuits
    }
}

/// Return the live network diagnostics snapshot.
///
/// Enumerating OS interfaces is cached for [`INTERFACE_CACHE_TTL_SECS`] so it
/// can be called from a render loop.
#[must_use]
pub fn snapshot() -> NetworkDetails {
    let live = live_state().lock().unwrap_or_else(PoisonError::into_inner);
    NetworkDetails {
        interfaces: cached_interfaces(),
        listen_addrs: live.listen_addrs.clone(),
        external_addrs: live.external_addrs.clone(),
        external_candidates: live.external_candidates.clone(),
        reachability: reachability_label(&live),
        nat_status: live.nat_status.clone(),
        nat_public_addr: live.nat_public_addr.clone(),
        nat_probe_successes: live.nat_probe_successes,
        nat_probe_failures: live.nat_probe_failures,
        hole_punch_attempts: live.hole_punch_attempts,
        hole_punch_successes: live.hole_punch_successes,
        relay_reservations: live.relay_reservations.clone(),
        relay_status: live.relay_status.clone(),
        dht_mode: live.dht_mode.clone(),
        dht_connected: live.dht_connected,
        dht_routing_peers: live.dht_routing_peers,
        dht_bootstrap_successes: live.dht_bootstrap_successes,
        dht_bootstrap_failures: live.dht_bootstrap_failures,
        dht_announced: live.dht_announced,
        dht_announce_status: live.dht_announce_status.clone(),
        dht_announce_at: live.dht_announce_at,
    }
}

/// Clear the live state (node stop / test isolation). Interfaces keep their
/// cache (they are OS state, not node state).
pub fn reset() {
    *live_state().lock().unwrap_or_else(PoisonError::into_inner) = LiveState::default();
}

// --- Live state -------------------------------------------------------------

#[derive(Debug)]
struct LiveState {
    listen_addrs: Vec<String>,
    external_addrs: Vec<String>,
    external_candidates: Vec<String>,
    nat_status: String,
    nat_public_addr: Option<String>,
    nat_probe_successes: u32,
    nat_probe_failures: u32,
    hole_punch_attempts: u32,
    hole_punch_successes: u32,
    relay_reservations: Vec<String>,
    relay_status: String,
    dht_mode: String,
    dht_connected: bool,
    dht_routing_peers: u32,
    dht_bootstrap_successes: u32,
    dht_bootstrap_failures: u32,
    dht_announced: bool,
    dht_announce_status: String,
    dht_announce_at: Option<f64>,
    last_announce_attempt: Option<f64>,
}

impl Default for LiveState {
    fn default() -> Self {
        Self {
            listen_addrs: Vec::new(),
            external_addrs: Vec::new(),
            external_candidates: Vec::new(),
            nat_status: "Unknown".to_string(),
            nat_public_addr: None,
            nat_probe_successes: 0,
            nat_probe_failures: 0,
            hole_punch_attempts: 0,
            hole_punch_successes: 0,
            relay_reservations: Vec::new(),
            relay_status: "No relay reservation".to_string(),
            dht_mode: "Unknown".to_string(),
            dht_connected: false,
            dht_routing_peers: 0,
            dht_bootstrap_successes: 0,
            dht_bootstrap_failures: 0,
            dht_announced: false,
            dht_announce_status: "Not announced yet".to_string(),
            dht_announce_at: None,
            last_announce_attempt: None,
        }
    }
}

fn live_state() -> &'static Mutex<LiveState> {
    static LIVE: OnceLock<Mutex<LiveState>> = OnceLock::new();
    LIVE.get_or_init(|| Mutex::new(LiveState::default()))
}

fn with_live(mutate: impl FnOnce(&mut LiveState)) {
    mutate(&mut live_state().lock().unwrap_or_else(PoisonError::into_inner));
}

/// Record a swarm listen address.
pub fn record_listen_addr(addr: &str) {
    with_live(|live| push_unique(&mut live.listen_addrs, addr));
}

/// Record an externally observed address candidate (unconfirmed).
pub fn record_external_candidate(addr: &str) {
    with_live(|live| push_unique(&mut live.external_candidates, addr));
}

/// Record a confirmed externally reachable address.
pub fn record_external_addr(addr: &str) {
    with_live(|live| push_unique(&mut live.external_addrs, addr));
}

/// Drop an externally reachable address that expired.
pub fn expire_external_addr(addr: &str) {
    with_live(|live| {
        live.external_addrs.retain(|a| a != addr);
        live.external_candidates.retain(|a| a != addr);
    });
}

/// Record `AutoNAT`'s current verdict.
pub fn record_nat_status(status: &str, public_addr: Option<&str>) {
    with_live(|live| {
        live.nat_status = status.to_string();
        live.nat_public_addr = public_addr.map(ToString::to_string);
    });
}

/// Record the outcome of one `AutoNAT` dial-back probe.
pub fn record_nat_probe(success: bool) {
    with_live(|live| {
        if success {
            live.nat_probe_successes = live.nat_probe_successes.saturating_add(1);
        } else {
            live.nat_probe_failures = live.nat_probe_failures.saturating_add(1);
        }
    });
}

/// Record the outcome of one `DCUtR` hole-punch attempt.
pub fn record_hole_punch(success: bool) {
    with_live(|live| {
        live.hole_punch_attempts = live.hole_punch_attempts.saturating_add(1);
        if success {
            live.hole_punch_successes = live.hole_punch_successes.saturating_add(1);
        }
    });
}

/// Record a relay reservation accepted by a relay peer.
pub fn record_relay_reservation(relay_peer_id: &str) {
    with_live(|live| {
        push_unique(&mut live.relay_reservations, relay_peer_id);
        live.relay_status = format!("Reserved via {relay_peer_id}");
    });
}

/// Drop a relay reservation that is no longer held (its listener closed).
pub fn expire_relay_reservation(relay_peer_id: &str) {
    with_live(|live| {
        live.relay_reservations.retain(|id| id != relay_peer_id);
        if live.relay_reservations.is_empty() {
            live.relay_status = "No relay reservation".to_string();
        }
    });
}

/// Record a relay status line (failures, renewals, circuit establishment).
pub fn record_relay_status(status: &str) {
    with_live(|live| live.relay_status = status.to_string());
}

/// Record Kademlia's mode and routing-table size.
pub fn record_dht_mode(mode: &str) {
    with_live(|live| {
        live.dht_mode = mode.to_string();
        live.dht_connected = mode == "Server" || live.dht_routing_peers > 0;
    });
}

/// Record the current DHT routing-table size.
pub fn record_dht_routing_peers(peers: u32) {
    with_live(|live| {
        live.dht_routing_peers = peers;
        live.dht_connected = live.dht_mode == "Server" || peers > 0;
    });
}

/// Record one bootstrap query outcome.
pub fn record_dht_bootstrap(success: bool) {
    with_live(|live| {
        if success {
            live.dht_bootstrap_successes = live.dht_bootstrap_successes.saturating_add(1);
        } else {
            live.dht_bootstrap_failures = live.dht_bootstrap_failures.saturating_add(1);
        }
    });
}

/// Whether an announcement should be attempted now (rate-limited).
#[must_use]
pub fn announce_due() -> bool {
    let now = crate::current_timestamp();
    let live = live_state().lock().unwrap_or_else(PoisonError::into_inner);
    live.last_announce_attempt
        .is_none_or(|last| now >= last + ANNOUNCE_INTERVAL_SECS)
}

/// Mark that an announcement is being attempted.
pub fn record_dht_announce_attempt() {
    with_live(|live| {
        live.last_announce_attempt = Some(crate::current_timestamp());
        live.dht_announce_status = "Announcing…".to_string();
    });
}

/// Record the outcome of the last announcement to the DHT.
pub fn record_dht_announce_result(ok: bool, message: &str) {
    with_live(|live| {
        live.dht_announced = ok;
        live.dht_announce_status = message.to_string();
        if ok {
            live.dht_announce_at = Some(crate::current_timestamp());
        }
    });
}

/// Mark the node as not announced (node stopped).
pub fn clear_announcement() {
    with_live(|live| {
        live.dht_announced = false;
        live.dht_announce_status = "Not announced yet".to_string();
        live.last_announce_attempt = None;
    });
}

/// Serialize the DHT announcement payload for our peer record.
#[must_use]
pub fn announce_record_json(peer_id: &str, addresses: &[String], public: bool) -> Option<Vec<u8>> {
    serde_json::to_vec(&serde_json::json!({
        "peerId": peer_id,
        "addresses": addresses,
        "public": public,
    }))
    .ok()
}

fn push_unique(list: &mut Vec<String>, value: &str) {
    if !list.iter().any(|entry| entry == value) {
        list.push(value.to_string());
    }
}

fn reachability_label(live: &LiveState) -> String {
    if live.nat_status == "Public" {
        return "Public (directly reachable)".to_string();
    }
    if !live.relay_reservations.is_empty() {
        return "Relayed (reachable via a relay)".to_string();
    }
    if !live.external_addrs.is_empty() {
        return "External address known".to_string();
    }
    if live.nat_status == "Private" {
        return "Private (behind NAT)".to_string();
    }
    "Unknown".to_string()
}

/// Whether a multiaddr string points at a loopback address.
#[must_use]
pub fn is_loopback_addr(addr: &str) -> bool {
    addr.starts_with("/ip4/127.")
        || addr.starts_with("/ip6/::1")
        || addr.starts_with("/ip6/0:0:0:0:0:0:0:1")
}

// --- Interfaces -------------------------------------------------------------

/// Cache for the OS interface list: `(fetched_at, interfaces)`.
fn interface_cache() -> &'static Mutex<(f64, Vec<NetworkInterfaceInfo>)> {
    static CACHE: OnceLock<Mutex<(f64, Vec<NetworkInterfaceInfo>)>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new((f64::NEG_INFINITY, Vec::new())))
}

fn cached_interfaces() -> Vec<NetworkInterfaceInfo> {
    {
        let cache = interface_cache()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if crate::current_timestamp() < cache.0 + INTERFACE_CACHE_TTL_SECS {
            return cache.1.clone();
        }
    }
    let fresh = interfaces();
    let mut cache = interface_cache()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    *cache = (crate::current_timestamp(), fresh.clone());
    fresh
}

/// Enumerate the OS network interfaces (uncached).
#[must_use]
pub fn interfaces() -> Vec<NetworkInterfaceInfo> {
    match NetworkInterface::show() {
        Ok(list) => group_interfaces(list),
        Err(e) => {
            crate::p2plog_debug(format!("Failed to enumerate network interfaces: {e}"));
            Vec::new()
        }
    }
}

/// Fold the crate's per-address interface entries into one row per interface.
#[must_use]
pub fn group_interfaces(list: Vec<NetworkInterface>) -> Vec<NetworkInterfaceInfo> {
    let mut rows: Vec<NetworkInterfaceInfo> = Vec::new();
    for iface in list {
        let index = if let Some(index) = rows.iter().position(|row| row.name == iface.name) {
            index
        } else {
            rows.push(NetworkInterfaceInfo {
                name: iface.name.clone(),
                mac: iface.mac_addr.clone().filter(|mac| !mac.is_empty()),
                internal: iface.internal,
                ..NetworkInterfaceInfo::default()
            });
            rows.len().saturating_sub(1)
        };
        let Some(row) = rows.get_mut(index) else {
            continue;
        };
        if row.mac.is_none() {
            row.mac = iface.mac_addr.filter(|mac| !mac.is_empty());
        }
        for addr in &iface.addr {
            match addr {
                network_interface::Addr::V4(v4) => push_unique(&mut row.ipv4, &v4.ip.to_string()),
                network_interface::Addr::V6(v6) => push_unique(&mut row.ipv6, &v6.ip.to_string()),
            }
        }
    }
    rows.sort_by(|a, b| a.name.cmp(&b.name));
    rows
}

#[cfg(test)]
#[path = "../tests/unit/unit_net_diag.rs"]
mod tests;
