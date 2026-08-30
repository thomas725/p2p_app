//! Nickname management for peers and local identity

use crate::generated::models_insertable::NewPeer;
use crate::logging::p2plog_debug;
use crate::sqlite_connect;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, MutexGuard};

diesel::table! {
    peer_name_history (id) {
        id -> Integer,
        peer_id -> Text,
        name -> Text,
        name_kind -> Text,
        set_at -> Timestamp,
    }
}

/// Generate a random two-word nickname (e.g. `"brave-otter"`).
#[must_use]
pub fn generate_self_nickname() -> String {
    petname::petname(2, "-").unwrap_or_else(|| "anonymous-peer".to_string())
}

/// Cache of fully formatted display names (`"name (abc)"`) keyed by peer id.
///
/// Resolving a display name opens a fresh `SQLite` connection and runs up to
/// three queries, so the Peers tab would otherwise pay one round-trip per peer
/// on every full-list sort. The cache is populated lazily on first resolution;
/// any code path that changes a peer's nicknames (the `impl_set_peer_field!`
/// setters) or upserts or creates a peer row
/// ([`crate::peers::save_peer`]) invalidates the affected entry so the display
/// never goes stale.
static DISPLAY_NAME_CACHE: LazyLock<Mutex<HashMap<String, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn display_cache() -> MutexGuard<'static, HashMap<String, String>> {
    match DISPLAY_NAME_CACHE.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Forget a single peer's cached display name after its nicknames changed.
pub(crate) fn invalidate_display_name(peer_id: &str) {
    display_cache().remove(peer_id);
}

/// Forget every cached display name (e.g. when the database URL changes).
pub(crate) fn clear_display_names() {
    display_cache().clear();
}

/// Read this node's own nickname from the database, if one is set.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_self_nickname() -> color_eyre::Result<Option<String>> {
    let conn = &mut sqlite_connect()?;
    let identity = crate::generated::schema::identities::table
        .select(crate::generated::models_queryable::Identity::as_select())
        .first(conn)
        .optional()?;
    Ok(identity.and_then(|i| i.self_nickname))
}

/// Persist this node's own nickname to the database.
///
/// # Errors
/// Returns an error if the database update fails.
pub fn set_self_nickname(nickname: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::update(crate::generated::schema::identities::table)
        .set(crate::generated::schema::identities::self_nickname.eq(nickname))
        .execute(conn)?;
    Ok(())
}

/// Return this node's nickname, generating and storing a random one if none exists yet.
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn ensure_self_nickname() -> color_eyre::Result<String> {
    if let Some(nick) = get_self_nickname()? {
        p2plog_debug(format!("[Nickname] loaded self nickname from db: {nick}"));
        return Ok(nick);
    }
    let nickname = generate_self_nickname();
    p2plog_debug(format!(
        "[Nickname] generated and stored self nickname: {nickname}"
    ));
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
            crate::nickname::invalidate_display_name(peer_id);
            Ok(())
        }
    };
}

impl_set_peer_field!(
    set_peer_local_nickname,
    peer_local_nickname,
    "Set the local (user-chosen) nickname for a peer.\n\n# Errors\nReturns an error if the database update fails."
);
impl_set_peer_field!(
    set_peer_received_nickname,
    received_nickname,
    "Set the nickname this peer announced about themselves.\n\n# Errors\nReturns an error if the database update fails."
);
impl_set_peer_field!(
    set_peer_self_nickname_for_peer,
    self_nickname_for_peer,
    "Set the nickname we last sent to this peer for ourselves.\n\n# Errors\nReturns an error if the database update fails."
);

/// Get the local (user-chosen) nickname for a peer, if set.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_peer_local_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.peer_local_nickname)
}

/// Get the nickname we last sent to this peer for ourselves, if any.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_peer_self_nickname_for_peer(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.self_nickname_for_peer)
}

/// Get the nickname this peer announced about themselves, if any.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_peer_received_nickname(peer_id: &str) -> color_eyre::Result<Option<String>> {
    get_peer_field(peer_id, |p| p.received_nickname)
}

/// An archived previous nickname for a peer, kept so we retain a timeline of
/// the names we knew them by before the current one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerNameHistoryEntry {
    pub name: String,
    pub name_kind: String,
    pub set_at: chrono::NaiveDateTime,
}

/// Persist a superseded nickname into the `peer_name_history` table.
fn archive_peer_name(peer_id: &str, name: &str, kind: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    let now = chrono::Utc::now().naive_utc();
    diesel::insert_into(peer_name_history::table)
        .values((
            peer_name_history::peer_id.eq(peer_id),
            peer_name_history::name.eq(name),
            peer_name_history::name_kind.eq(kind),
            peer_name_history::set_at.eq(now),
        ))
        .execute(conn)?;
    Ok(())
}

/// Record a nickname received from a remote peer.
///
/// If it differs from the name we had stored, the old one is archived into
/// `peer_name_history` with a timestamp so we keep a record of the names we
/// knew this peer by before.
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn record_peer_received_name_change(peer_id: &str, new_name: &str) -> color_eyre::Result<()> {
    let current = get_peer_received_nickname(peer_id)?;
    match current {
        Some(old) if old == new_name => Ok(()),
        Some(old) => {
            archive_peer_name(peer_id, &old, "received")?;
            set_peer_received_nickname(peer_id, new_name)
        }
        None => set_peer_received_nickname(peer_id, new_name),
    }
}

/// Read the archived nickname history for a peer (most recent first).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_peer_name_history(peer_id: &str) -> color_eyre::Result<Vec<PeerNameHistoryEntry>> {
    let conn = &mut sqlite_connect()?;
    let rows = peer_name_history::table
        .filter(peer_name_history::peer_id.eq(peer_id))
        .order(peer_name_history::set_at.desc())
        .select((
            peer_name_history::name,
            peer_name_history::name_kind,
            peer_name_history::set_at,
        ))
        .load::<(String, String, chrono::NaiveDateTime)>(conn)?;
    Ok(rows
        .into_iter()
        .map(|(name, name_kind, set_at)| PeerNameHistoryEntry {
            name,
            name_kind,
            set_at,
        })
        .collect())
}

/// Validate a nickname: alphanumeric and dash only, max 20 chars.
#[must_use]
pub fn validate_nickname(nick: &str) -> bool {
    !nick.is_empty() && nick.len() <= 20 && nick.chars().all(|c| c.is_alphanumeric() || c == '-')
}

/// Ensure a silent peer has a stable generated petname, assigning and storing
/// one on first use. Returns the petname. This is what makes silent peers show
/// a name instead of a raw ID even for peers discovered before this feature or
/// loaded from history before they reconnect this session.
fn ensure_generated_nickname(peer_id: &str) -> color_eyre::Result<String> {
    if let Some(existing) = get_peer_field(peer_id, |p| p.generated_nickname)? {
        return Ok(existing);
    }
    let name = generate_self_nickname();
    let conn = &mut sqlite_connect()?;
    // Make sure a row exists so the UPDATE has a row to match.
    let exists = crate::generated::schema::peers::table
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .count()
        .get_result::<i64>(conn)?
        > 0;
    if !exists {
        let now = chrono::Utc::now().naive_utc();
        let _ = diesel::insert_into(crate::generated::schema::peers::table)
            .values(NewPeer {
                peer_id: peer_id.to_string(),
                addresses: String::new(),
                first_seen: now,
                last_seen: now,
                peer_local_nickname: None,
                self_nickname_for_peer: None,
                received_nickname: None,
                generated_nickname: None,
            })
            .on_conflict(crate::generated::schema::peers::peer_id)
            .do_nothing()
            .execute(conn);
    }
    diesel::update(crate::generated::schema::peers::table)
        .filter(crate::generated::schema::peers::peer_id.eq(peer_id))
        .set(crate::generated::schema::peers::generated_nickname.eq(&name))
        .execute(conn)?;
    p2plog_debug(format!(
        "[Nickname] assigned petname '{name}' for silent peer {peer_id}"
    ));
    Ok(name)
}

/// Get a human-friendly display name for a peer: their nickname (local
/// preferred over received, then an auto-generated name) followed by a short
/// ID suffix, or just the short ID if nothing is known.
///
/// # Errors
/// Returns an error if any database query fails.
pub fn get_peer_display_name(peer_id: &str) -> color_eyre::Result<String> {
    let cached = display_cache().get(peer_id).cloned();
    if let Some(cached) = cached {
        return Ok(cached);
    }
    let suffix = crate::fmt::peer_id_suffix(peer_id);
    let display = if let Some(local_nick) = get_peer_local_nickname(peer_id)? {
        format!("{local_nick} ({suffix})")
    } else if let Some(received_nick) = get_peer_received_nickname(peer_id)? {
        format!("{received_nick} ({suffix})")
    } else {
        // Silent peer: assign a stable petname on demand (and persist it) so it
        // shows a name instead of the raw ID across sessions and reloading.
        let generated = ensure_generated_nickname(peer_id)?;
        format!("{generated} ({suffix})")
    };
    display_cache().insert(peer_id.to_string(), display.clone());
    Ok(display)
}

#[cfg(test)]
#[path = "../tests/unit/unit_nickname.rs"]
mod tests;
