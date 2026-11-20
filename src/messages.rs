//! Message persistence and retrieval functions

use crate::{
    generated::models_insertable::{NewMessage, NewMessageReceipt},
    generated::models_queryable::Message,
    generated::schema::messages::dsl::messages,
};
use color_eyre::eyre::Context;
use diesel::{
    BoolExpressionMethods, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _,
    SelectableHelper as _,
};
use std::collections::HashMap;

/// Optional metadata for message creation
#[derive(Default)]
pub struct MessageMeta {
    /// Optional nickname of the message sender
    pub sender_nickname: Option<String>,
    /// Optional unique identifier for the message
    pub msg_id: Option<String>,
    /// Optional timestamp of when the message was sent
    pub sent_at: Option<f64>,
    /// Optional attachment descriptor (files lane §12)
    pub attachment: Option<crate::files::MessageAttachment>,
    /// Immutable reply snapshot (US-006), stored as `reply_to_json`.
    pub reply_to: Option<crate::ReplyRef>,
    /// Whether the message body was end-to-end encrypted (US-042); persisted
    /// so the marker survives a reload.
    pub encrypted: bool,
}

/// Save a message to the database.
///
/// Inserts a new message with the given content and metadata.
/// Messages are marked as unsent initially; use `mark_message_sent()` after transmission.
///
/// # Arguments
/// * `content` - Message text (can be empty string)
/// * `peer_id` - Sender's peer ID (None for local user)
/// * `topic` - Topic for broadcast messages (e.g., "test-net")
/// * `is_direct` - True for direct peer-to-peer, false for broadcast
/// * `target_peer` - Recipient peer ID (required if `is_direct` is true)
///
/// # Returns
/// The inserted Message with auto-generated id and timestamps
///
/// # Errors
/// Returns an error if the message cannot be inserted into the database.
pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
) -> color_eyre::Result<Message> {
    save_message_with_meta(
        content,
        peer_id,
        topic,
        is_direct,
        target_peer,
        MessageMeta::default(),
    )
}

/// Save a message with optional metadata.
///
/// Inserts a new message with the given content and optional metadata including
/// sender nickname, message ID, and send timestamp.
///
/// # Arguments
/// * `content` - Message text
/// * `peer_id` - Optional peer ID of the sender
/// * `topic` - Topic/channel where message was sent
/// * `is_direct` - Whether this is a direct message (true) or broadcast (false)
/// * `target_peer` - Optional recipient peer ID for direct messages
/// * `meta` - Optional metadata (nickname, message ID, timestamp)
///
/// # Returns
/// The saved message with database-assigned ID
///
/// # Errors
/// Returns an error if the message cannot be inserted into the database.
pub fn save_message_with_meta(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
    meta: MessageMeta,
) -> color_eyre::Result<Message> {
    let conn = &mut crate::sqlite_connect()?;
    let reply_to_json = meta.reply_to.as_ref().and_then(crate::reply::encode);
    let new_msg = NewMessage {
        content: content.to_string(),
        peer_id: peer_id.map(std::string::ToString::to_string),
        topic: topic.to_string(),
        sent: 0,
        is_direct: i32::from(is_direct),
        target_peer: target_peer.map(std::string::ToString::to_string),
        sender_nickname: meta.sender_nickname,
        msg_id: meta.msg_id,
        reply_to_json,
        sent_at: meta.sent_at,
        attachment_json: meta
            .attachment
            .as_ref()
            .and_then(crate::files::attachment_to_json),
        encrypted: i32::from(meta.encrypted),
    };
    diesel::insert_into(crate::generated::schema::messages::table)
        .values(&new_msg)
        .returning(Message::as_returning())
        .get_result(conn)
        .wrap_err_with(|| {
            format!(
                "Failed to save message: content='{content}', topic='{topic}', is_direct={is_direct}"
            )
        })
        .inspect(|msg| {
            #[cfg(feature = "tracing")]
            tracing::debug!(message_id = msg.id, "Message saved to database");
            #[cfg(not(feature = "tracing"))]
            let _ = msg;
        })
}

/// Mark a message as sent by ID.
///
/// # Errors
/// Returns an error if the database update fails.
pub fn mark_message_sent(id: i32) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(
        crate::generated::schema::messages::table
            .filter(crate::generated::schema::messages::id.eq(id)),
    )
    .set(crate::generated::schema::messages::sent.eq(1))
    .execute(conn)?;
    Ok(())
}

/// Locally hide a message we sent (US-005).
///
/// Only an outgoing (`sent = 1`) row that is not already a tombstone is
/// stamped with the current Unix timestamp; incoming rows are never deletable.
/// Returns `true` iff a row actually changed. This is local-only: no wire
/// message is sent and peers keep their own copy of the row.
///
/// # Errors
/// Returns an error string when the database write fails.
pub fn mark_message_deleted(id: i32) -> Result<bool, String> {
    use crate::generated::schema::messages as messages_table;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let changed = diesel::update(
        messages_table::table
            .filter(messages_table::id.eq(id))
            .filter(messages_table::sent.eq(1))
            .filter(messages_table::deleted_at.is_null()),
    )
    .set(messages_table::deleted_at.eq(crate::fmt::current_timestamp()))
    .execute(conn)
    .map_err(|e| format!("failed to delete message {id}: {e}"))?;
    Ok(changed > 0)
}

/// Locally edit the body of a message we sent (US-005).
///
/// Only an outgoing (`sent = 1`), non-tombstoned row is changed; incoming rows
/// are never editable. The new body replaces the stored one and `edited_at`
/// records the LOCAL edit time, so both the content and the marker survive a
/// reload. Repeat edits are allowed and refresh the timestamp. Nothing leaves
/// this device: peers keep their original copy. Returns `true` iff a row changed.
///
/// # Errors
/// Returns an error string when the database write fails.
pub fn edit_message(id: i32, content: &str) -> Result<bool, String> {
    use crate::generated::schema::messages as messages_table;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let changed = diesel::update(
        messages_table::table
            .filter(messages_table::id.eq(id))
            .filter(messages_table::sent.eq(1))
            .filter(messages_table::deleted_at.is_null()),
    )
    .set((
        messages_table::content.eq(content),
        messages_table::edited_at.eq(crate::fmt::current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| format!("failed to edit message {id}: {e}"))?;
    Ok(changed > 0)
}

/// Raw stored body of an editable sent row, for the TUI edit pre-fill.
///
/// The transcript line contains display-only decorations (timestamp, sender,
/// quote header, file chip, edit marker), so the editor must start from the
/// persisted `content` instead. `None` means the row is missing, incoming, or
/// tombstoned and therefore not editable.
///
/// # Errors
/// Returns an error string when the database read fails.
pub fn message_body(id: i32) -> Result<Option<String>, String> {
    use crate::generated::schema::messages as messages_table;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    messages_table::table
        .filter(messages_table::id.eq(id))
        .filter(messages_table::sent.eq(1))
        .filter(messages_table::deleted_at.is_null())
        .select(messages_table::content)
        .first::<String>(conn)
        .optional()
        .map_err(|e| format!("failed to read message {id}: {e}"))
}

/// Fully rendered body of an edited sent row (attachment chip + edit marker).
///
/// Used after an edit so the in-memory transcript agrees with the next reload:
/// the chip is regenerated from `attachment_json` exactly once and the marker
/// comes from the persisted `edited_at`, never from the editor's text.
///
/// # Errors
/// Returns an error string when the database read fails.
pub fn message_display_body(id: i32) -> Result<Option<String>, String> {
    use crate::generated::schema::messages as messages_table;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let row = messages_table::table
        .filter(messages_table::id.eq(id))
        .filter(messages_table::sent.eq(1))
        .filter(messages_table::deleted_at.is_null())
        .select((
            messages_table::content,
            messages_table::attachment_json,
            messages_table::edited_at,
        ))
        .first::<(String, Option<String>, Option<f64>)>(conn)
        .optional()
        .map_err(|e| format!("failed to read message {id}: {e}"))?;
    Ok(row.map(|(content, attachment_json, edited_at)| {
        let body = crate::files::render_stored_body(&content, attachment_json.as_deref());
        crate::fmt::with_edited_marker_if(edited_at.is_some(), &body)
    }))
}

/// Load broadcast messages for a topic, newest first, limited to count.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_messages(topic: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    let msgs = messages
        .filter(crate::generated::schema::messages::topic.eq(topic))
        .filter(crate::generated::schema::messages::is_direct.eq(0))
        .filter(crate::generated::schema::messages::deleted_at.is_null())
        .order_by(crate::generated::schema::messages::created_at.desc())
        .then_order_by(crate::generated::schema::messages::id.desc())
        .limit(crate::usize_limit_to_i64(limit))
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Load direct messages with a peer, oldest first, limited to count.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_direct_messages(target_peer: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    let msgs = messages
        .filter(crate::generated::schema::messages::target_peer.eq(target_peer))
        .filter(crate::generated::schema::messages::is_direct.eq(1))
        .filter(crate::generated::schema::messages::deleted_at.is_null())
        .order_by(crate::generated::schema::messages::created_at.asc())
        .then_order_by(crate::generated::schema::messages::id.asc())
        .limit(crate::usize_limit_to_i64(limit))
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Save a message receipt (delivery/read confirmation).
///
/// Records when a message was delivered to or read by a peer.
/// Receipts track message acknowledgments and read status.
///
/// # Arguments
/// * `msg_id` - ID of the message being acknowledged
/// * `peer_id` - ID of the peer sending the receipt
/// * `kind` - Receipt type: `0` = broadcast ack, `1` = direct-message ack
///   (mirrors `i32::from(is_dm)` in the TUI command processor; a kind-0 receipt
///   is precisely the "peer acknowledged our broadcast" signal).
/// * `confirmed_at` - Timestamp when the receipt was confirmed
///
/// # Errors
/// Returns an error if the database insert fails.
pub fn save_receipt(
    msg_id: &str,
    peer_id: &str,
    kind: i32,
    confirmed_at: f64,
) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let receipt = NewMessageReceipt {
        msg_id: msg_id.to_string(),
        peer_id: peer_id.to_string(),
        kind,
        confirmed_at,
    };
    diesel::insert_into(crate::generated::schema::message_receipts::table)
        .values(&receipt)
        .on_conflict((
            crate::generated::schema::message_receipts::msg_id,
            crate::generated::schema::message_receipts::peer_id,
            crate::generated::schema::message_receipts::kind,
        ))
        .do_update()
        .set(crate::generated::schema::message_receipts::confirmed_at.eq(confirmed_at))
        .execute(conn)?;

    // A kind-0 (delivered/ack) receipt for a broadcast we sent confirms that the
    // giver of the receipt received it — back-fill `broadcast_recipients.confirmed_at`
    // so we can attribute confirmed (not just transmitted) broadcasts to the peer.
    if kind == 0 {
        use crate::generated::schema::broadcast_recipients;
        diesel::update(
            crate::generated::schema::broadcast_recipients::table.filter(
                broadcast_recipients::msg_id
                    .eq(msg_id)
                    .and(broadcast_recipients::peer_id.eq(peer_id)),
            ),
        )
        .set(broadcast_recipients::confirmed_at.eq(confirmed_at))
        .execute(conn)?;
    }
    Ok(())
}

/// The subset of `msg_ids` that at least one recipient has acknowledged: a
/// `message_receipts` row exists for the id (`kind` 0 = broadcast ack, `1` = DM
/// ack — both mean "the peer received our message").
///
/// Batched into a single query so a history load stays a constant number of
/// round trips; the frontends render a delivery tick for the ids returned here.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn confirmed_message_ids(
    msg_ids: &[String],
) -> color_eyre::Result<std::collections::HashSet<String>> {
    use crate::generated::schema::message_receipts;
    if msg_ids.is_empty() {
        return Ok(std::collections::HashSet::new());
    }
    let conn = &mut crate::sqlite_connect()?;
    let ids: Vec<String> = message_receipts::table
        .filter(message_receipts::msg_id.eq_any(msg_ids))
        .select(message_receipts::msg_id)
        .distinct()
        .load(conn)
        .wrap_err("failed to load confirmed message ids")?;
    Ok(ids.into_iter().collect())
}

/// Resolve whether the message identified by `msg_id` is a direct message.
/// Returns `None` when the message id is unknown (nothing was persisted for it).
///
/// Used by the mobile receipt handler to pick the receipt `kind` (`i32::from`) —
/// mirroring the TUI's `is_dm_receipt`, which answers the same question from
/// in-memory state.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn message_is_dm(msg_id: &str) -> color_eyre::Result<Option<bool>> {
    use diesel::OptionalExtension as _;

    let conn = &mut crate::sqlite_connect()?;
    let is_direct = crate::generated::schema::messages::table
        .filter(crate::generated::schema::messages::msg_id.eq(msg_id))
        .filter(crate::generated::schema::messages::sent.eq(1))
        .select(crate::generated::schema::messages::is_direct)
        .first::<i32>(conn)
        .optional()?;
    Ok(is_direct.map(|d| d == 1))
}

/// Whether an incoming DM from `peer_id` with `msg_id` is already persisted.
///
/// Store-and-forward re-delivers the same sealed envelope after a reconnect;
/// the swarm handler uses this to drop a replay before it reaches a frontend,
/// so the retry cannot create a second message row. Fail-open: a broken DB
/// means "not seen", never a globally muted DM lane.
#[must_use]
pub fn incoming_dm_exists(peer_id: &str, msg_id: &str) -> bool {
    use crate::generated::schema::messages as messages_table;
    let Ok(conn) = &mut crate::sqlite_connect() else {
        return false;
    };
    let count: i64 = messages_table::table
        .filter(messages_table::peer_id.eq(peer_id))
        .filter(messages_table::msg_id.eq(msg_id))
        .count()
        .get_result(conn)
        .unwrap_or(0);
    count > 0
}

/// Load all message receipts from the database.
///
/// Retrieves all stored message receipts (delivery/read confirmations).
///
/// # Returns
/// Vector of all message receipts
///
/// # Errors
/// Returns an error if the database query fails.
pub fn load_receipts()
-> color_eyre::eyre::Result<Vec<crate::generated::models_queryable::MessageReceipt>> {
    use crate::generated::models_queryable::MessageReceipt;
    use crate::generated::schema::message_receipts::dsl::message_receipts;
    use diesel::SelectableHelper;

    let conn = &mut crate::sqlite_connect()?;
    let receipts = message_receipts
        .select(MessageReceipt::as_select())
        .load(conn)?;
    Ok(receipts)
}

/// Per-peer message statistics shown in the peer-list table.
#[derive(Debug, Clone)]
pub struct PeerMessageStats {
    /// Direct messages exchanged with this peer (sent + received).
    pub dm_count: i64,
    /// Broadcast messages we sent that this peer was a recipient of.
    pub broadcast_sent_to_peer: i64,
}

/// One grouped row from [`get_all_peer_stats`]: the DM count for a single
/// peer, produced by a single `GROUP BY` query over `messages`.
#[derive(Debug, Clone, PartialEq, Eq, diesel::QueryableByName)]
struct PeerMessageAggregate {
    #[diesel(sql_type = diesel::sql_types::Text)]
    peer_id: String,
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    dm_count: i64,
}

/// Compute message statistics for **all** peers in two grouped queries.
///
/// Each DM is stored exactly once with the peer it is exchanged with in either
/// `peer_id` or `target_peer` (inbound DMs set both to the sender), so
/// `COALESCE(peer_id, target_peer)` yields one unattributed row per DM — no
/// double counting. Local tombstones are excluded. A second grouped query
/// over `broadcast_recipients` counts the broadcasts *we* sent that reached
/// each peer (unaffected by message deletion).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn get_all_peer_stats() -> color_eyre::eyre::Result<HashMap<String, PeerMessageStats>> {
    use crate::generated::schema::broadcast_recipients::dsl as br;
    use diesel::{dsl::count, sql_query};
    let conn = &mut crate::sqlite_connect()?;

    let dm_sql = "
        SELECT peer_id, COALESCE(SUM(dm), 0) AS dm_count
        FROM (
            SELECT COALESCE(peer_id, target_peer) AS peer_id, 1 AS dm FROM messages
                WHERE is_direct = 1 AND deleted_at IS NULL
                  AND (peer_id IS NOT NULL OR target_peer IS NOT NULL)
        )
        GROUP BY peer_id";
    let dm_aggregates = sql_query(dm_sql).load::<PeerMessageAggregate>(conn)?;

    let recip_vec: Vec<(String, i64)> = br::broadcast_recipients
        .group_by(br::peer_id)
        .select((br::peer_id, count(br::peer_id)))
        .load::<(String, i64)>(conn)?;

    let mut result: HashMap<String, PeerMessageStats> = HashMap::with_capacity(dm_aggregates.len());
    for a in dm_aggregates {
        result.insert(
            a.peer_id,
            PeerMessageStats {
                dm_count: a.dm_count,
                broadcast_sent_to_peer: 0,
            },
        );
    }
    for (pid, count) in recip_vec {
        result
            .entry(pid)
            .or_insert_with(|| PeerMessageStats {
                dm_count: 0,
                broadcast_sent_to_peer: 0,
            })
            .broadcast_sent_to_peer = count;
    }

    Ok(result)
}

#[cfg(test)]
#[path = "../tests/unit/unit_messages.rs"]
mod tests;
