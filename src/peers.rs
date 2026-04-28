//! Peer management and peer session tracking

use crate::{
    generated::models_insertable::{NewPeer, NewPeerSession},
    generated::models_queryable::Peer,
    generated::schema::peers::dsl::peers,
};
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};

/// Peer row returned by `load_known_peers()`.
#[derive(Debug, Clone, diesel::QueryableByName)]
pub struct KnownPeer {
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub peer_id: String,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub first_seen: chrono::NaiveDateTime,
    #[diesel(sql_type = diesel::sql_types::Timestamp)]
    pub last_seen: chrono::NaiveDateTime,
}

/// Save or update a peer in the database.
///
/// If peer already exists (by peer_id), updates addresses and last_seen timestamp.
/// Otherwise inserts a new peer record with current timestamp.
///
/// # Arguments
/// * `peer_id` - Unique peer identifier
/// * `addresses` - List of multiaddrs where this peer can be reached
///
/// # Returns
/// The saved or updated Peer record
pub fn save_peer(peer_id: &str, addresses: &[String]) -> color_eyre::Result<Peer> {
    let conn = &mut crate::sqlite_connect()?;
    let addresses_str = addresses.join(",");
    let now = chrono::Utc::now().naive_utc();

    let new_peer = NewPeer {
        peer_id: peer_id.to_string(),
        addresses: addresses_str.clone(),
        first_seen: now,
        last_seen: now,
        peer_local_nickname: None,
        received_nickname: None,
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
    Ok(peer)
}

/// Load all known peers, ordered by most recently seen first.
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
/// Ordering is by most-recently-seen first (max of peer.last_seen and latest message timestamp).
pub fn load_known_peers() -> color_eyre::Result<Vec<KnownPeer>> {
    use diesel::sql_query;
    use diesel::RunQueryDsl;

    let conn = &mut crate::sqlite_connect()?;
    let sql = r#"
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
    "#;

    let rows = sql_query(sql).load::<KnownPeer>(conn)?;
    Ok(rows)
}

/// Record a peer session snapshot with the concurrent peer count.
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
pub fn get_average_peer_count() -> color_eyre::Result<f64> {
    let conn = &mut crate::sqlite_connect()?;
    let sessions = crate::generated::schema::peer_sessions::table
        .select(crate::generated::schema::peer_sessions::concurrent_peers)
        .load::<i32>(conn)?;
    if sessions.is_empty() {
        return Ok(0.0);
    }
    let sum: i64 = sessions.iter().map(|&c| c as i64).sum();
    Ok(sum as f64 / sessions.len() as f64)
}

/// Get the most recently recorded peer count.
pub fn get_recent_peer_count() -> color_eyre::Result<i32> {
    let conn = &mut crate::sqlite_connect()?;
    let last = crate::generated::schema::peer_sessions::table
        .select(crate::generated::schema::peer_sessions::concurrent_peers)
        .order(crate::generated::schema::peer_sessions::recorded_at.desc())
        .first::<i32>(conn)
        .optional()?;
    Ok(last.unwrap_or(0))
}
