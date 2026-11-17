//! Peer blocklist / mute (US-054).
//!
//! A blocked peer is ignored at the **swarm boundary**, not merely hidden in a
//! list: `swarm_handler` drops its gossipsub traffic, DM/call/device-binding/
//! file-transfer requests, refuses to dial it, and disconnects it if it manages
//! to connect. Nothing it sends is persisted.
//!
//! The blocklist is a `blocked_peers` table keyed by the canonical base58 peer
//! id, so blocking is idempotent and unblocking is one delete. Every check is
//! **fail-open**: if the database cannot be read, `is_blocked` returns `false`
//! and the swarm keeps working — a broken database must never look like a
//! global mute.
//!
//! Scope is one peer id: blocking a contact's device does not block its sibling
//! devices (the account layer knows that grouping, the blocklist deliberately
//! does not).

use crate::generated::models_insertable::NewBlockedPeer;
use crate::generated::models_queryable::BlockedPeerRow;
use crate::generated::schema::blocked_peers;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _};

/// Canonicalize a peer id (parses it as a libp2p peer id).
///
/// # Errors
/// Returns a human-readable error for an empty or unparseable peer id.
fn canonical_peer_id(peer_id: &str) -> Result<String, String> {
    let trimmed = peer_id.trim();
    if trimmed.is_empty() {
        return Err("peer id must not be empty".to_string());
    }
    trimmed
        .parse::<libp2p::PeerId>()
        .map(|p| p.to_string())
        .map_err(|e| format!("invalid peer id: {e}"))
}

/// Block one peer; `true` when the row was newly created.
///
/// Blocking an already-blocked peer is a no-op, so a frontend toggle never has
/// to read first.
///
/// # Errors
/// Returns a human-readable error when the peer id does not parse or the
/// database write fails.
pub fn block_peer(peer_id: &str) -> Result<bool, String> {
    let peer_id = canonical_peer_id(peer_id)?;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let inserted = diesel::insert_into(blocked_peers::table)
        .values(&NewBlockedPeer { peer_id })
        .on_conflict_do_nothing()
        .execute(conn)
        .map_err(|e| format!("failed to block peer: {e}"))?;
    Ok(inserted > 0)
}

/// Unblock one peer; `true` when a row was deleted.
///
/// # Errors
/// Returns a human-readable error when the peer id does not parse or the
/// database delete fails.
pub fn unblock_peer(peer_id: &str) -> Result<bool, String> {
    let peer_id = canonical_peer_id(peer_id)?;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let deleted = diesel::delete(blocked_peers::table.filter(blocked_peers::peer_id.eq(&peer_id)))
        .execute(conn)
        .map_err(|e| format!("failed to unblock peer: {e}"))?;
    Ok(deleted > 0)
}

/// Whether `peer_id` is blocked.
///
/// Best-effort by design: an unreadable database yields `false`, so a broken
/// database can never act as a global mute.
#[must_use]
pub fn is_blocked(peer_id: &str) -> bool {
    let Ok(conn) = &mut crate::sqlite_connect() else {
        return false;
    };
    blocked_peers::table
        .filter(blocked_peers::peer_id.eq(peer_id))
        .select(BlockedPeerRow::as_select())
        .first(conn)
        .is_ok()
}

/// Blocked peer ids, oldest first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_blocked_peers() -> color_eyre::Result<Vec<String>> {
    use color_eyre::eyre::WrapErr as _;
    let conn = &mut crate::sqlite_connect()?;
    let rows = blocked_peers::table
        .order(blocked_peers::blocked_at.asc())
        .select(BlockedPeerRow::as_select())
        .load(conn)
        .wrap_err("failed to list blocked peers")?;
    Ok(rows.into_iter().map(|row| row.peer_id).collect())
}

#[cfg(test)]
#[path = "../tests/unit/unit_blocklist.rs"]
mod tests;
