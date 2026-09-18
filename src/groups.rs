//! Group-chat persistence and identity helpers.
//!
//! Groups travel over per-group gossipsub topics (`group:<group_id>`), so a
//! node only receives messages for groups it has joined (subscribed). Public
//! v1 groups identify by their **canonical name**: [`stable_group_id`] derives
//! a deterministic `group_id` from it, so any two nodes that join the same name
//! resolve to the same id, topic, and membership. The `is_private` column
//! reserves the private-group switch (random ids + invite/membership lists);
//! the message-storage and send paths here are shared by both kinds.

use crate::generated::models_insertable::{NewGroup, NewGroupMember, NewGroupMessage};
use crate::generated::models_queryable::{Group, GroupMessage};
use crate::generated::schema::{group_members, group_messages, groups};
use color_eyre::eyre::Context;
use diesel::dsl::count;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};

/// Optional metadata for a group message (mirrors [`crate::messages::MessageMeta`]).
#[derive(Default, Clone)]
pub struct GroupMessageMeta {
    /// Optional nickname of the message sender
    pub sender_nickname: Option<String>,
    /// Optional unique identifier for the message
    pub msg_id: Option<String>,
    /// Optional timestamp of when the message was sent
    pub sent_at: Option<f64>,
}

// ── Topic helpers ──────────────────────────────────────────────────────────

/// Prefix of every per-group gossipsub topic (`group:<group_id>`).
pub const GROUP_TOPIC_PREFIX: &str = "group:";

/// Build the per-group gossipsub topic string for a `group_id`.
#[must_use]
pub fn group_topic(group_id: &str) -> String {
    format!("{GROUP_TOPIC_PREFIX}{group_id}")
}

/// True if `topic` is a per-group topic.
#[must_use]
pub fn is_group_topic(topic: &str) -> bool {
    topic.starts_with(GROUP_TOPIC_PREFIX)
}

/// Extract the group id from a per-group topic string.
#[must_use]
pub fn group_id_from_topic(topic: &str) -> Option<&str> {
    topic.strip_prefix(GROUP_TOPIC_PREFIX)
}

// ── Group identity ─────────────────────────────────────────────────────────

/// Canonical name of a public group: whitespace-trimmed, internal whitespace
/// collapsed, lowercased. Same canonical name == same group everywhere.
#[must_use]
pub fn canonical_group_name(name: &str) -> String {
    name.split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Deterministic 64-bit FNV-1-style hash of the canonical group name as hex.
///
/// Stable across platforms and processes (unlike `std`'s random hashers) and
/// hex-only, so the derived `group_id` is safe in a gossipsub topic name. The
/// hash uses only `wrapping_*` method calls (no operator arithmetic, which the
/// crate's clippy config denies).
#[must_use]
pub fn stable_group_id(name: &str) -> String {
    // FNV offset basis (well-mixed 64-bit starting state) and prime. The hash
    // uses only `wrapping_*` method calls (no operator arithmetic, which the
    // crate's clippy config denies).
    const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let canonical = canonical_group_name(name);
    let mut hash: u64 = FNV_OFFSET_BASIS;
    for byte in canonical.bytes() {
        hash = hash.wrapping_mul(FNV_PRIME);
        hash = hash.wrapping_add(u64::from(byte));
    }
    format!("{hash:016x}")
}

// ── Group CRUD ─────────────────────────────────────────────────────────────

/// A group with its current member count (for list views).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupSummary {
    /// The underlying group record.
    pub group: Group,
    /// Peers seen participating in the group.
    pub member_count: i64,
}

/// Find a group by its stable `group_id`.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn find_group(group_id: &str) -> color_eyre::Result<Option<Group>> {
    let conn = &mut crate::sqlite_connect()?;
    groups::table
        .filter(groups::group_id.eq(group_id))
        .select(Group::as_select())
        .first(conn)
        .optional()
        .wrap_err("failed to look up group")
}

/// Create a public group from `name`, or return the existing group if one with
/// the same canonical name is already known (idempotent — `create == join` in
/// the UI).
///
/// The stored `display_name` keeps the caller's original spelling; only the
/// identity (`group_id`) is derived from the canonical form.
///
/// # Errors
/// Returns an error if the group cannot be inserted.
pub fn create_public_group(name: &str) -> color_eyre::Result<Group> {
    let group_id = stable_group_id(name);
    if let Some(existing) = find_group(&group_id)? {
        return Ok(existing);
    }
    let conn = &mut crate::sqlite_connect()?;
    let row = diesel::insert_into(groups::table)
        .values(NewGroup {
            group_id,
            display_name: name.trim().to_string(),
            is_private: 0,
        })
        .returning(Group::as_returning())
        .get_result(conn)
        .wrap_err("failed to create group")?;
    Ok(row)
}

/// List all known groups (oldest first).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_groups() -> color_eyre::Result<Vec<Group>> {
    let conn = &mut crate::sqlite_connect()?;
    groups::table
        .order_by(groups::created_at.asc())
        .then_order_by(groups::id.asc())
        .select(Group::as_select())
        .load(conn)
        .wrap_err("failed to list groups")
}

/// List every known group with its current member count.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn list_groups_with_member_counts() -> color_eyre::Result<Vec<GroupSummary>> {
    let rows = list_groups()?;
    let counts = get_all_group_member_counts()?;
    Ok(rows
        .into_iter()
        .map(|group| GroupSummary {
            member_count: counts.get(&group.group_id).copied().unwrap_or(0),
            group,
        })
        .collect())
}

/// Delete a group and all of its messages and memberships.
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn delete_group(group_id: &str) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::delete(group_messages::table.filter(group_messages::group_id.eq(group_id)))
        .execute(conn)?;
    diesel::delete(group_members::table.filter(group_members::group_id.eq(group_id)))
        .execute(conn)?;
    diesel::delete(groups::table.filter(groups::group_id.eq(group_id))).execute(conn)?;
    Ok(())
}

// ── Group members ──────────────────────────────────────────────────────────

/// Record `peer_id` as a participant in a group (idempotent; public groups
/// discover members from traffic).
///
/// # Errors
/// Returns an error if the database operation fails.
pub fn record_group_member(group_id: &str, peer_id: &str) -> color_eyre::Result<()> {
    use diesel::insert_into;
    let conn = &mut crate::sqlite_connect()?;
    insert_into(group_members::table)
        .values(NewGroupMember {
            group_id: group_id.to_string(),
            peer_id: peer_id.to_string(),
        })
        .on_conflict((group_members::group_id, group_members::peer_id))
        .do_nothing()
        .execute(conn)?;
    Ok(())
}

/// Number of peers seen in a group.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_group_member_count(group_id: &str) -> color_eyre::Result<i64> {
    let conn = &mut crate::sqlite_connect()?;
    group_members::table
        .filter(group_members::group_id.eq(group_id))
        .count()
        .get_result(conn)
        .wrap_err("failed to count group members")
}

/// Whether the local peer is a member of the given group.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn is_group_member(group_id: &str, peer_id: &str) -> color_eyre::Result<bool> {
    let conn = &mut crate::sqlite_connect()?;
    let count: i64 = group_members::table
        .filter(group_members::group_id.eq(group_id))
        .filter(group_members::peer_id.eq(peer_id))
        .count()
        .get_result(conn)
        .wrap_err("failed to check group membership")?;
    Ok(count > 0)
}

/// Member counts for every group (for list views).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_all_group_member_counts() -> color_eyre::Result<std::collections::HashMap<String, i64>> {
    let conn = &mut crate::sqlite_connect()?;
    let total: i64 = group_members::table.count().get_result(conn)?;
    if total == 0 {
        return Ok(std::collections::HashMap::new());
    }
    let rows: Vec<(String, i64)> = group_members::table
        .group_by(group_members::group_id)
        .select((group_members::group_id, count(group_members::group_id)))
        .load(conn)?;
    Ok(rows.into_iter().collect())
}

// ── Group messages ─────────────────────────────────────────────────────────

/// Save an outgoing group message (inserted as already-sent: the row only
/// exists because the message was transmitted).
///
/// # Errors
/// Returns an error if the message cannot be inserted.
pub fn save_outgoing_group_message(
    group_id: &str,
    content: &str,
    meta: GroupMessageMeta,
) -> color_eyre::Result<GroupMessage> {
    let conn = &mut crate::sqlite_connect()?;
    let new_msg = NewGroupMessage {
        group_id: group_id.to_string(),
        content: content.to_string(),
        peer_id: None,
        sent: 1,
        msg_id: meta.msg_id,
        sent_at: meta.sent_at,
        sender_nickname: meta.sender_nickname,
    };
    diesel::insert_into(group_messages::table)
        .values(&new_msg)
        .returning(GroupMessage::as_returning())
        .get_result(conn)
        .wrap_err("failed to save group message")
}

/// Save an incoming group message, deduplicating mesh re-deliveries on
/// `(group_id, msg_id)`. Returns [`Option::None`] when a duplicate is skipped.
///
/// # Errors
/// Returns an error if the message cannot be inserted.
pub fn save_incoming_group_message(
    group_id: &str,
    peer_id: &str,
    content: &str,
    meta: GroupMessageMeta,
) -> color_eyre::Result<Option<GroupMessage>> {
    let conn = &mut crate::sqlite_connect()?;
    let new_msg = NewGroupMessage {
        group_id: group_id.to_string(),
        content: content.to_string(),
        peer_id: Some(peer_id.to_string()),
        sent: 0,
        msg_id: meta.msg_id,
        sent_at: meta.sent_at,
        sender_nickname: meta.sender_nickname,
    };
    diesel::insert_into(group_messages::table)
        .values(&new_msg)
        .on_conflict((group_messages::group_id, group_messages::msg_id))
        .do_nothing()
        .returning(GroupMessage::as_returning())
        .get_result(conn)
        .optional()
        .wrap_err("failed to save group message")
}

/// Load a group's messages, newest first, limited to `limit`.
///
/// # Errors
/// Returns an error if the database query fails.
#[allow(clippy::as_conversions, clippy::cast_possible_wrap)]
pub fn load_group_messages(group_id: &str, limit: usize) -> color_eyre::Result<Vec<GroupMessage>> {
    let conn = &mut crate::sqlite_connect()?;
    group_messages::table
        .filter(group_messages::group_id.eq(group_id))
        .order_by(group_messages::created_at.desc())
        .then_order_by(group_messages::id.desc())
        .limit(limit as i64)
        .select(GroupMessage::as_select())
        .load(conn)
        .wrap_err("failed to load group messages")
}

#[cfg(test)]
#[path = "../tests/unit/unit_groups.rs"]
mod tests;
