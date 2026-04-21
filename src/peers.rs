//! Peer management and peer session tracking

use crate::{
    models_insertable::{NewPeer, NewPeerSession}, models_queryable::Peer,
    schema::peers::dsl::peers,
};
use color_eyre::eyre::Context;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _};

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

    let peer = diesel::insert_into(crate::schema::peers::table)
        .values(&new_peer)
        .on_conflict(crate::schema::peers::peer_id)
        .do_update()
        .set((
            crate::schema::peers::addresses.eq(&addresses_str),
            crate::schema::peers::last_seen.eq(now),
        ))
        .returning(Peer::as_returning())
        .get_result(conn)?;
    Ok(peer)
}

/// Load all known peers, ordered by most recently seen first.
pub fn load_peers() -> color_eyre::Result<Vec<Peer>> {
    let conn = &mut crate::sqlite_connect()?;
    let peers_list = peers
        .order(crate::schema::peers::last_seen.desc())
        .select(Peer::as_select())
        .load(conn)?;
    Ok(peers_list)
}

/// Record a peer session snapshot with the concurrent peer count.
pub fn save_peer_session(concurrent_peers: i32) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let new_session = NewPeerSession {
        concurrent_peers,
        recorded_at: chrono::Utc::now().naive_utc(),
    };
    diesel::insert_into(crate::schema::peer_sessions::table)
        .values(&new_session)
        .execute(conn)?;
    Ok(())
}

/// Save the last used TCP and QUIC ports to the database.
pub fn save_listen_ports(tcp_port: Option<i32>, quic_port: Option<i32>) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(crate::schema::identities::table)
        .set((
            crate::schema::identities::last_tcp_port.eq(tcp_port),
            crate::schema::identities::last_quic_port.eq(quic_port),
        ))
        .execute(conn)?;
    Ok(())
}

/// Load the last used TCP and QUIC ports from the database.
pub fn load_listen_ports() -> color_eyre::Result<(Option<i32>, Option<i32>)> {
    let conn = &mut crate::sqlite_connect()?;
    let result = crate::schema::identities::table
        .select((
            crate::schema::identities::last_tcp_port,
            crate::schema::identities::last_quic_port,
        ))
        .first::<(Option<i32>, Option<i32>)>(conn)
        .optional()?;
    Ok(result.unwrap_or((None, None)))
}

/// Calculate the average peer count across all recorded sessions.
pub fn get_average_peer_count() -> color_eyre::Result<f64> {
    let conn = &mut crate::sqlite_connect()?;
    let sessions = crate::schema::peer_sessions::table
        .select(crate::schema::peer_sessions::concurrent_peers)
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
    let last = crate::schema::peer_sessions::table
        .select(crate::schema::peer_sessions::concurrent_peers)
        .order(crate::schema::peer_sessions::recorded_at.desc())
        .first::<i32>(conn)
        .optional()?;
    Ok(last.unwrap_or(0))
}
