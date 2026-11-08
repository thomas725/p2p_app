//! Persisted network configuration (US-050 follow-up 1).
//!
//! The node's internet-facing entry points used to be env-var only:
//! `P2P_BOOTSTRAP` (multiaddrs dialed at startup so the node can join the DHT)
//! and `P2P_RELAY` (circuit-relay targets it requests a reservation from).
//! This module stores the same two lists in the `network_peers` table so both
//! frontends can show and edit them, and every entry point (TUI, mobile/Flutter,
//! `p2p_chat_server`) picks them up without an env var.
//!
//! Precedence is deliberate:
//!
//! - bootstrap: `P2P_BOOTSTRAP` (when set, even to an empty string) wins over
//!   the persisted list, so an operator can override or disable it; the public
//!   IPFS list is only a fallback when neither yields anything.
//! - relay: env and persisted entries are **unioned** — relays are explicit
//!   opt-ins, so adding one in the UI must not silently drop a configured one.
//!
//! Everything is validated by parsing back into a [`Multiaddr`], so no
//! unparseable string ever reaches the swarm, and relay entries must carry
//! `/p2p/<relay-id>` (a reservation needs the peer id).

use crate::generated::models_insertable::NewNetworkPeer;
use crate::generated::models_queryable::NetworkPeerRow;
use crate::generated::schema::network_peers;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _};
use libp2p::Multiaddr;
use serde::Serialize;

/// Kind of a persisted network peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkPeerKind {
    /// Dialed at node start so the node can join the DHT.
    Bootstrap,
    /// Circuit-relay v2 reservation target (needs `/p2p/<relay-id>`).
    Relay,
}

impl NetworkPeerKind {
    /// The value stored in `network_peers.kind`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap",
            Self::Relay => "relay",
        }
    }

    /// Parse the stored `kind` value (case-insensitive).
    ///
    /// # Errors
    /// Returns an error for any other value.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "bootstrap" => Ok(Self::Bootstrap),
            "relay" => Ok(Self::Relay),
            other => Err(format!("unknown network peer kind: {other}")),
        }
    }
}

/// One persisted network peer, as both frontends render it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPeer {
    /// Row id, used to remove the entry.
    pub id: i32,
    /// `bootstrap` or `relay`.
    pub kind: String,
    /// Canonical multiaddr.
    pub multiaddr: String,
    /// When the entry was added ("YYYY-MM-DD HH:MM:SS").
    pub created_at: String,
}

impl From<NetworkPeerRow> for NetworkPeer {
    fn from(row: NetworkPeerRow) -> Self {
        Self {
            id: row.id,
            kind: row.kind,
            multiaddr: row.multiaddr,
            created_at: row.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

/// Validate one multiaddr for `kind` and return its canonical text form.
///
/// Bootstrap entries only need to parse; relay entries must additionally carry
/// `/p2p/<relay-id>`, because the reservation is requested from a specific peer.
///
/// # Errors
/// Returns a human-readable error for an empty, unparseable or relay-invalid
/// address — the frontends show it verbatim.
pub fn validate_network_peer(kind: NetworkPeerKind, multiaddr: &str) -> Result<String, String> {
    let trimmed = multiaddr.trim();
    if trimmed.is_empty() {
        return Err("address must not be empty".to_string());
    }
    let addr = trimmed
        .parse::<Multiaddr>()
        .map_err(|e| format!("invalid multiaddr: {e}"))?;
    if kind == NetworkPeerKind::Relay && peer_id_in(&addr).is_none() {
        return Err("relay address needs a /p2p/<peer-id> suffix".to_string());
    }
    Ok(addr.to_string())
}

/// Peer id embedded in a multiaddr, if any.
///
/// Local copy of `relay::relay_peer_id` so this module compiles without the
/// `relay` feature (the validation rule applies whenever a relay entry exists).
fn peer_id_in(addr: &Multiaddr) -> Option<libp2p::PeerId> {
    use libp2p::multiaddr::Protocol;
    addr.iter().find_map(|protocol| match protocol {
        Protocol::P2p(peer_id) => Some(peer_id),
        _ => None,
    })
}

/// All persisted network peers, oldest first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_network_peers() -> color_eyre::Result<Vec<NetworkPeer>> {
    use color_eyre::eyre::WrapErr as _;
    let conn = &mut crate::sqlite_connect()?;
    let rows = network_peers::table
        .order(network_peers::id.asc())
        .select(NetworkPeerRow::as_select())
        .load(conn)
        .wrap_err("failed to list network peers")?;
    Ok(rows.into_iter().map(NetworkPeer::from).collect())
}

/// Add one network peer and return the stored row.
///
/// Adding an address that is already stored under the same kind returns the
/// existing row instead of erroring — the UI can stay a plain "add" button.
///
/// # Errors
/// Returns a human-readable error when the address does not validate or the
/// database write fails.
pub fn add_network_peer(kind: &str, multiaddr: &str) -> Result<NetworkPeer, String> {
    let kind = NetworkPeerKind::parse(kind)?;
    let multiaddr = validate_network_peer(kind, multiaddr)?;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let row = NewNetworkPeer {
        kind: kind.as_str().to_string(),
        multiaddr: multiaddr.clone(),
    };
    diesel::insert_into(network_peers::table)
        .values(&row)
        .on_conflict_do_nothing()
        .execute(conn)
        .map_err(|e| format!("failed to save network peer: {e}"))?;
    network_peers::table
        .filter(network_peers::kind.eq(kind.as_str()))
        .filter(network_peers::multiaddr.eq(&multiaddr))
        .select(NetworkPeerRow::as_select())
        .first(conn)
        .map(NetworkPeer::from)
        .map_err(|e| format!("failed to read back network peer: {e}"))
}

/// Remove the persisted network peer with `id`; true when a row was deleted.
///
/// # Errors
/// Returns an error if the database delete fails.
pub fn remove_network_peer(id: i32) -> color_eyre::Result<bool> {
    let conn = &mut crate::sqlite_connect()?;
    let deleted =
        diesel::delete(network_peers::table.filter(network_peers::id.eq(id))).execute(conn)?;
    Ok(deleted > 0)
}

/// Persisted bootstrap multiaddrs, in insertion order.
///
/// Best-effort: a database error yields an empty list (the node still starts
/// with the env-var / public-DHT fallbacks) instead of failing startup.
#[must_use]
pub fn stored_bootstrap_addrs() -> Vec<Multiaddr> {
    stored_addrs(NetworkPeerKind::Bootstrap)
}

/// Persisted relay multiaddrs, in insertion order.
///
/// Best-effort, like [`stored_bootstrap_addrs`].
#[must_use]
pub fn stored_relay_addrs() -> Vec<Multiaddr> {
    stored_addrs(NetworkPeerKind::Relay)
}

/// Stored multiaddrs of one kind, parsed and deduplicated.
fn stored_addrs(kind: NetworkPeerKind) -> Vec<Multiaddr> {
    let Ok(peers) = list_network_peers() else {
        return Vec::new();
    };
    let mut addrs: Vec<Multiaddr> = Vec::new();
    for peer in peers {
        if peer.kind != kind.as_str() {
            continue;
        }
        if let Ok(addr) = peer.multiaddr.parse::<Multiaddr>()
            && !addrs.contains(&addr)
        {
            addrs.push(addr);
        }
    }
    addrs
}

#[cfg(test)]
#[path = "../tests/unit/unit_network_config.rs"]
mod tests;
