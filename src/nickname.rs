//! Nickname management for peers and local identity

use crate::sqlite_connect;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};

/// Generate a random two-word nickname (e.g. `"brave-otter"`).
#[must_use]
pub fn generate_self_nickname() -> String {
    petname::petname(2, "-").unwrap_or_else(|| "anonymous-peer".to_string())
}

/// Read this node's own nickname from the database, if one is set.
pub fn get_self_nickname() -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let identity = crate::generated::schema::identities::table
        .select(crate::generated::models_queryable::Identity::as_select())
        .first(conn)
        .optional()?;
    Ok(identity.and_then(|i| i.self_nickname))
}

/// Persist this node's own nickname to the database.
pub fn set_self_nickname(nickname: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::update(crate::generated::schema::identities::table)
        .set(crate::generated::schema::identities::self_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

/// Return this node's nickname, generating and storing a random one if none exists yet.
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

/// Defines a setter that updates one nickname column for a peer, creating
/// the peer row first if it doesn't exist yet.
macro_rules! impl_set_peer_field {
    ($func_name:ident, $column:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $func_name(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
            let _ = crate::save_peer(peer_id, &[]);
            let conn = &mut sqlite_connect()?;
            diesel::update(
                crate::generated::schema::peers::table
                    .filter(crate::generated::schema::peers::peer_id.eq(peer_id)),
            )
            .set(crate::generated::schema::peers::$column.eq(nickname))
            .execute(conn)?;
            Ok(())
        }
    };
}

impl_set_peer_field!(
    set_peer_local_nickname,
    peer_local_nickname,
    "Set the local (user-chosen) nickname for a peer."
);
impl_set_peer_field!(
    set_peer_received_nickname,
    received_nickname,
    "Set the nickname this peer announced about themselves."
);
impl_set_peer_field!(
    set_peer_self_nickname_for_peer,
    self_nickname_for_peer,
    "Set the nickname we last sent to this peer for ourselves."
);

/// Get the local (user-chosen) nickname for a peer, if set.
pub fn get_peer_local_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.peer_local_nickname)
}

/// Get the nickname we last sent to this peer for ourselves, if any.
pub fn get_peer_self_nickname_for_peer(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.self_nickname_for_peer)
}

/// Get the nickname this peer announced about themselves, if any.
pub fn get_peer_received_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.received_nickname)
}

/// Get a human-friendly display name for a peer: their nickname (local
/// preferred over received) followed by a short ID suffix, or just the
/// short ID if no nickname is known.
pub fn get_peer_display_name(peer_id: &str) -> color_eyre::Result<String> {
    let short_id = crate::fmt::short_peer_id(peer_id);
    let suffix = &short_id[..3.min(short_id.len())];
    if let Some(local_nick) = get_peer_local_nickname(peer_id)? {
        return Ok(format!("{local_nick} ({suffix})"));
    }
    if let Some(received_nick) = get_peer_received_nickname(peer_id)? {
        return Ok(format!("{received_nick} ({suffix})"));
    }
    Ok(short_id)
}

#[cfg(test)]
#[path = "../tests/unit/unit_nickname.rs"]
mod tests;
