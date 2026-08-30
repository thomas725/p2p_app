//! Peer management, session tracking, and port persistence

use crate::{
    generated::models_insertable::{NewPeer, NewPeerSession},
    generated::models_queryable::Peer,
    generated::schema::peers::dsl::peers,
    logging::p2plog_debug,
};
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
    sql_query,
};
use diesel::sql_types::Text;

/// Peer row returned by `load_known_peers()`.
#[derive(Debug, Clone, diesel::QueryableByName)]
pub struct KnownPeer {
    #[diesel(sql_type = diesel::sql_types::Text)]
    /// Unique libp2p peer identifier
    pub peer_id: String,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    /// Timestamp when this peer was first observed
    pub first_seen: chrono::NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    /// Timestamp when this peer was most recently observed
    pub last_seen: chrono::NaiveDateTime,
}

/// Save or update a peer in the database.
///
/// If peer already exists (by `peer_id`), updates addresses and `last_seen` timestamp.
/// Otherwise inserts a new peer record with current timestamp.
///
/// # Arguments
/// * `peer_id` - Unique peer identifier
/// * `addresses` - List of multiaddrs where this peer can be reached
///
/// # Returns
/// The saved or updated Peer record
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn save_peer(peer_id: &str, addresses: &[String]) -> color_eyre::Result<Peer> {
    let conn = &mut crate::sqlite_connect()?;
    let addresses_str = addresses.join(",");
    let now = chrono::Utc::now().naive_utc();

    // Generate a petname only for a brand-new peer. Re-seen peers keep the name
    // already stored in the database. Silent peers that predate this feature
    // (NULL generated_nickname) get a petname lazily via get_peer_display_name.
    let generated_nickname = {
        let exists = peers
            .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
            .count()
            .get_result::<i64>(conn)?
            > 0;
        if exists {
            None
        } else {
            let name = crate::nickname::generate_self_nickname();
            p2plog_debug(format!(
                "[Nickname] generated petname '{name}' for new silent peer {peer_id}"
            ));
            Some(name)
        }
    };

    let new_peer = NewPeer {
        peer_id: peer_id.to_string(),
        addresses: addresses_str.clone(),
        first_seen: now,
        last_seen: now,
        peer_local_nickname: None,
        self_nickname_for_peer: None,
        received_nickname: None,
        generated_nickname,
    };

    let peer = diesel::insert_into(crate::generated::schema::peers::table)
        .values(&new_peer)
        .on_conflict(crate::generated::schema::peers::peer_id)
        .do_update()
        .set((
            crate::generated::schema::peers::addresses.eq(&addresses_str),
            crate::generated::schema::peers::last_seen.eq(now),
        ))
        .returning(Peer::as_returning())
        .get_result(conn)?;
    // A save may assign a new petname (brand-new peers) or touch rows that feed
    // the display-name cache, so drop any memoized name for this peer.
    crate::nickname::invalidate_display_name(peer_id);
    Ok(peer)
}

/// Load all known peers, ordered by most recently seen first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_peers() -> color_eyre::Result<Vec<Peer>> {
    let conn = &mut crate::sqlite_connect()?;
    let peers_list = peers
        .order(crate::generated::schema::peers::last_seen.desc())
        .select(Peer::as_select())
        .load(conn)?;
    Ok(peers_list)
}

/// Load all known peers, combining both the `peers` table and any peer IDs present in `messages`.
///
/// This fixes older databases where messages may exist but the `peers` table is empty.
/// Ordering is by most-recently-seen first (max of `peer.last_seen` and latest message timestamp).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_known_peers() -> color_eyre::Result<Vec<KnownPeer>> {
    use diesel::sql_query;

    let conn = &mut crate::sqlite_connect()?;
    let sql = r"
WITH msg_peers AS (
    SELECT
        peer_id AS peer_id,
        MIN(created_at) AS first_seen,
        MAX(created_at) AS last_seen
    FROM messages
    WHERE peer_id IS NOT NULL
    GROUP BY peer_id
),
peer_peers AS (
    SELECT
        peer_id AS peer_id,
        first_seen AS first_seen,
        last_seen AS last_seen
    FROM peers
),
merged AS (
    SELECT peer_id, first_seen, last_seen FROM peer_peers
    UNION ALL
    SELECT peer_id, first_seen, last_seen FROM msg_peers
)
SELECT
    peer_id,
    MIN(first_seen) AS first_seen,
    MAX(last_seen) AS last_seen
FROM merged
GROUP BY peer_id
ORDER BY last_seen DESC
    ";

    let rows = sql_query(sql).load::<KnownPeer>(conn)?;
    Ok(rows)
}

/// Record a peer session snapshot with the concurrent peer count.
///
/// # Errors
/// Returns an error if the database insert fails.
pub fn save_peer_session(concurrent_peers: i32) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let new_session = NewPeerSession {
        concurrent_peers,
        recorded_at: chrono::Utc::now().naive_utc(),
    };
    diesel::insert_into(crate::generated::schema::peer_sessions::table)
        .values(&new_session)
        .execute(conn)?;
    Ok(())
}

/// Save the last used TCP and QUIC ports to the database.
///
/// # Errors
/// Returns an error if the database update fails.
pub fn save_listen_ports(tcp_port: Option<i32>, quic_port: Option<i32>) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(crate::generated::schema::identities::table)
        .set((
            crate::generated::schema::identities::last_tcp_port.eq(tcp_port),
            crate::generated::schema::identities::last_quic_port.eq(quic_port),
        ))
        .execute(conn)?;
    Ok(())
}

/// Load the last used TCP and QUIC ports from the database.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_listen_ports() -> color_eyre::Result<(Option<i32>, Option<i32>)> {
    let conn = &mut crate::sqlite_connect()?;
    let result = crate::generated::schema::identities::table
        .select((
            crate::generated::schema::identities::last_tcp_port,
            crate::generated::schema::identities::last_quic_port,
        ))
        .first::<(Option<i32>, Option<i32>)>(conn)
        .optional()?;
    Ok(result.unwrap_or((None, None)))
}

/// Calculate the average peer count across all recorded sessions.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_average_peer_count() -> color_eyre::Result<f64> {
    let conn = &mut crate::sqlite_connect()?;
    let sessions = crate::generated::schema::peer_sessions::table
        .select(crate::generated::schema::peer_sessions::concurrent_peers)
        .load::<i32>(conn)?;
    if sessions.is_empty() {
        return Ok(0.0);
    }
    let sum: i64 = sessions.iter().map(|&c| i64::from(c)).sum();
    #[allow(clippy::as_conversions, clippy::cast_precision_loss)]
    let avg = sum as f64 / sessions.len() as f64;
    Ok(avg)
}

/// Get the most recently recorded peer count.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_recent_peer_count() -> color_eyre::Result<i32> {
    let conn = &mut crate::sqlite_connect()?;
    let last = crate::generated::schema::peer_sessions::table
        .select(crate::generated::schema::peer_sessions::concurrent_peers)
        .order(crate::generated::schema::peer_sessions::recorded_at.desc())
        .first::<i32>(conn)
        .optional()?;
    Ok(last.unwrap_or(0))
}

/// Increment the `broadcasts_sent` counter for each peer that was connected
/// when a broadcast was transmitted. This records, per peer, how many
/// broadcasts we sent while that peer was online.
///
/// Peers that connected are always saved (via `save_peer`) before this is
/// called, but guard with `INSERT OR IGNORE` in case a peer is only known from
/// message history.
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn record_broadcasts_sent(peer_ids: &[String]) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    for pid in peer_ids {
        let updated = sql_query(
            "UPDATE peers SET broadcasts_sent = COALESCE(broadcasts_sent, 0) + 1 \
             WHERE peer_id = ?",
        )
        .bind::<Text, _>(pid)
        .execute(conn)?;
        if updated == 0 {
            let _ = sql_query(
                "INSERT OR IGNORE INTO peers \
                 (peer_id, addresses, first_seen, last_seen, broadcasts_sent) \
                 VALUES (?, '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, 1)",
            )
            .bind::<Text, _>(pid)
            .execute(conn);
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/unit_peers.rs"]
mod tests;
