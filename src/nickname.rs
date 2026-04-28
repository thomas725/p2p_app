use crate::sqlite_connect;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};

pub fn generate_self_nickname() -> String {
    petname::petname(2, "-")
}

pub fn get_self_nickname() -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let identity = crate::generated::schema::identities::table
        .select(crate::generated::models_queryable::Identity::as_select())
        .first(conn)
        .optional()?;
    Ok(identity.and_then(|i| i.self_nickname))
}

pub fn set_self_nickname(nickname: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    // Use update instead of insert_or_ignore since the identity row already exists
    // (created during initial key generation)
    diesel::update(crate::generated::schema::identities::table)
        .set(crate::generated::schema::identities::self_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

pub fn ensure_self_nickname() -> color_eyre::Result<String> {
    if let Some(nick) = get_self_nickname()? {
        return Ok(nick);
    }
    let nickname = generate_self_nickname();
    set_self_nickname(&nickname)?;
    Ok(nickname)
}

pub fn set_peer_local_nickname(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
    // Ensure a peers row exists (older DBs may only have messages).
    let _ = crate::save_peer(peer_id, &[]);
    let conn = &mut sqlite_connect()?;
    diesel::insert_or_ignore_into(crate::generated::schema::peers::table)
        .values((
            crate::generated::schema::peers::peer_id.eq(peer_id),
            crate::generated::schema::peers::peer_local_nickname.eq(nickname),
        ))
        .on_conflict(crate::generated::schema::peers::peer_id)
        .do_update()
        .set(crate::generated::schema::peers::peer_local_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

pub fn get_peer_local_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let peer = crate::generated::schema::peers::table
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .select(crate::generated::models_queryable::Peer::as_select())
        .first(conn)
        .optional()?;
    Ok(peer.and_then(|p| p.peer_local_nickname))
}

pub fn set_peer_received_nickname(peer_id: &str, nickname: &str) -> color_eyre::Result<()> {
    // Ensure a peers row exists (older DBs may only have messages).
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
    // Ensure a peers row exists (older DBs may only have messages).
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
    let conn = &mut sqlite_connect()?;
    let peer = crate::generated::schema::peers::table
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .select(crate::generated::models_queryable::Peer::as_select())
        .first(conn)
        .optional()?;
    Ok(peer.and_then(|p| p.self_nickname_for_peer))
}

pub fn get_peer_display_name(peer_id: &str) -> color_eyre::Result<String> {
    let short_id: String = peer_id
        .chars()
        .rev()
        .take(8)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    if let Some(local_nick) = get_peer_local_nickname(peer_id)? {
        return Ok(format!(
            "{} ({})",
            local_nick,
            &short_id[..3.min(short_id.len())]
        ));
    }
    if let Some(received_nick) = get_peer_received_nickname(peer_id)? {
        return Ok(format!(
            "{} ({})",
            received_nick,
            &short_id[..3.min(short_id.len())]
        ));
    }
    Ok(short_id)
}

pub fn get_peer_received_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let peer = crate::generated::schema::peers::table
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .select(crate::generated::models_queryable::Peer::as_select())
        .first(conn)
        .optional()?;
    Ok(peer.and_then(|p| p.received_nickname))
}
