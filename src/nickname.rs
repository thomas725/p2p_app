use crate::sqlite_connect;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};

/// Generate a random nickname for this peer using the petname library.
///
/// Creates a two-word random nickname (e.g., "happy-squirrel").
/// Falls back to "anonymous-peer" if generation fails.
pub fn generate_self_nickname() -> String {
    petname::petname(2, "-").unwrap_or_else(|| "anonymous-peer".to_string())
}

/// Retrieve the nickname that this peer has set for itself.
///
/// # Returns
/// The self-assigned nickname, or None if not yet set
pub fn get_self_nickname() -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let identity = crate::generated::schema::identities::table
        .select(crate::generated::models_queryable::Identity::as_select())
        .first(conn)
        .optional()?;
    Ok(identity.and_then(|i| i.self_nickname))
}

/// Set or update the nickname for this peer.
///
/// # Arguments
/// * `nickname` - The new nickname to set
pub fn set_self_nickname(nickname: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    // Use update instead of insert_or_ignore since the identity row already exists
    // (created during initial key generation)
    diesel::update(crate::generated::schema::identities::table)
        .set(crate::generated::schema::identities::self_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

/// Ensure this peer has a nickname, generating one if necessary.
///
/// Returns the existing nickname if set, or generates and saves a new one.
///
/// # Returns
/// The peer's nickname (newly generated or previously set)
pub fn ensure_self_nickname() -> color_eyre::Result<String> {
    if let Some(nick) = get_self_nickname()? {
        return Ok(nick);
    }
    let nickname = generate_self_nickname();
    set_self_nickname(&nickname)?;
    Ok(nickname)
}

fn get_peer_field(
    peer_id: &str,
    field: impl FnOnce(crate::generated::models_queryable::Peer) -> Option<String>,
) -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let peer = crate::generated::schema::peers::table
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .select(crate::generated::models_queryable::Peer::as_select())
        .first(conn)
        .optional()?;
    Ok(peer.and_then(field))
}

pub fn set_peer_local_nickname(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
    let _ = crate::save_peer(peer_id, &[]);
    let conn = &mut sqlite_connect()?;
    diesel::update(
        crate::generated::schema::peers::table
            .filter(crate::generated::schema::peers::peer_id.eq(peer_id)),
    )
    .set(crate::generated::schema::peers::peer_local_nickname.eq(nickname))
    .execute(conn)?;
    Ok(())
}

pub fn get_peer_local_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.peer_local_nickname)
}

pub fn set_peer_received_nickname(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
    let _ = crate::save_peer(peer_id, &[]);
    let conn = &mut sqlite_connect()?;
    diesel::update(
        crate::generated::schema::peers::table
            .filter(crate::generated::schema::peers::peer_id.eq(peer_id)),
    )
    .set(crate::generated::schema::peers::received_nickname.eq(nickname))
    .execute(conn)?;
    Ok(())
}

pub fn set_peer_self_nickname_for_peer(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
    let _ = crate::save_peer(peer_id, &[]);
    let conn = &mut sqlite_connect()?;
    diesel::update(
        crate::generated::schema::peers::table
            .filter(crate::generated::schema::peers::peer_id.eq(peer_id)),
    )
    .set(crate::generated::schema::peers::self_nickname_for_peer.eq(nickname))
    .execute(conn)?;
    Ok(())
}

pub fn get_peer_self_nickname_for_peer(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.self_nickname_for_peer)
}

pub fn get_peer_received_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.received_nickname)
}

/// Get a display name for a peer with fallback logic.
///
/// Returns a user-friendly display name for a peer, preferring local nickname,
/// then received nickname, then a shortened peer ID.
///
/// # Arguments
/// * `peer_id` - ID of the peer
///
/// # Returns
/// A display string suitable for showing in the UI (e.g., "happy-squirrel (001)")
pub fn get_peer_display_name(peer_id: &str) -> color_eyre::Result<String> {
    let short_id = crate::fmt::short_peer_id(peer_id);
    let suffix = &short_id[..3.min(short_id.len())];
    if let Some(local_nick) = get_peer_local_nickname(peer_id)? {
        return Ok(format!("{} ({})", local_nick, suffix));
    }
    if let Some(received_nick) = get_peer_received_nickname(peer_id)? {
        return Ok(format!("{} ({})", received_nick, suffix));
    }
    Ok(short_id)
}
