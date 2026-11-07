//! Circuit-relay client helpers.
//!
//! A relay-client reservation is requested by **listening** on a relayed
//! address (`/ip4/…/tcp/…/p2p/<relay>/p2p-circuit`); libp2p-relay 0.22 has no
//! separate `reserve()` call. `swarm_handler` calls [`relayed_listen_addr`]
//! for every configured relay, and the running listener keeps the reservation
//! alive (the client behaviour renews it automatically and confirms the
//! resulting `/p2p/<relay>/p2p-circuit/p2p/<self>` address as external).
//!
//! Relays are configured with the `P2P_RELAY` env var: comma-separated
//! multiaddrs that each carry a `/p2p/<relay-peer-id>` suffix. We never request
//! a slot from an unconfigured peer — our own nodes deny reservations unless
//! `P2P_RELAY_SERVER=1`, and the public DHT is full of peers that would.

use libp2p::Multiaddr;
use libp2p::multiaddr::Protocol;

use crate::p2plog_debug;

/// Parse a `P2P_RELAY` value into relay multiaddrs.
///
/// Empty entries are ignored. Entries that do not parse as a [`Multiaddr`], or
/// that lack the `/p2p/<peer-id>` suffix a reservation needs, are logged and
/// skipped. Duplicates are dropped.
#[must_use]
pub fn parse_relay_addrs(input: &str) -> Vec<Multiaddr> {
    let mut relays: Vec<Multiaddr> = Vec::new();
    for entry in input.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        match entry.parse::<Multiaddr>() {
            Ok(addr) if relay_peer_id(&addr).is_some() => {
                if !relays.contains(&addr) {
                    relays.push(addr);
                }
            }
            Ok(_) => p2plog_debug(format!(
                "Ignoring relay address without /p2p/<peer-id>: {entry}"
            )),
            Err(e) => p2plog_debug(format!("Invalid relay multiaddr {entry}: {e}")),
        }
    }
    relays
}

/// Relay multiaddrs from the `P2P_RELAY` env var (none when unset/empty).
#[must_use]
pub fn configured_relay_addrs() -> Vec<Multiaddr> {
    std::env::var("P2P_RELAY").map_or_else(|_| Vec::new(), |value| parse_relay_addrs(&value))
}

/// Peer id embedded in a multiaddr, if any.
#[must_use]
pub fn relay_peer_id(addr: &Multiaddr) -> Option<libp2p::PeerId> {
    addr.iter().find_map(|protocol| match protocol {
        Protocol::P2p(peer_id) => Some(peer_id),
        _ => None,
    })
}

/// The relayed listen address that requests a reservation with the relay in
/// `relay`.
///
/// Returns `None` when the address carries no `/p2p/<peer-id>` (the relay
/// transport rejects it) and leaves an address that already names a circuit
/// untouched.
#[must_use]
pub fn relayed_listen_addr(relay: &Multiaddr) -> Option<Multiaddr> {
    relay_peer_id(relay)?;
    if relay.iter().any(|p| matches!(p, Protocol::P2pCircuit)) {
        return Some(relay.clone());
    }
    Some(relay.clone().with(Protocol::P2pCircuit))
}

#[cfg(test)]
#[path = "../tests/unit/unit_relay.rs"]
mod tests;
