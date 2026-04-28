//! Message persistence and retrieval functions

use crate::{
    generated::models_insertable::{NewMessage, NewMessageReceipt},
    generated::models_queryable::Message,
    generated::schema::messages::dsl::messages,
};
use color_eyre::eyre::Context;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _};

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
pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
) -> color_eyre::Result<Message> {
    save_message_with_meta(content, peer_id, topic, is_direct, target_peer, None, None)
}

pub fn save_message_with_meta(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
    msg_id: Option<&str>,
    sent_at: Option<f64>,
) -> color_eyre::Result<Message> {
    let conn = &mut crate::sqlite_connect()?;
    let new_msg = NewMessage {
        content: content.to_string(),
        peer_id: peer_id.map(|s| s.to_string()),
        topic: topic.to_string(),
        sent: 0,
        is_direct: is_direct as i32,
        target_peer: target_peer.map(|s| s.to_string()),
        msg_id: msg_id.map(|s| s.to_string()),
        sent_at,
    };
    diesel::insert_into(crate::generated::schema::messages::table)
        .values(&new_msg)
        .returning(Message::as_returning())
        .get_result(conn)
        .wrap_err_with(|| {
            format!(
                "Failed to save message: content='{}', topic='{}', is_direct={}",
                content, topic, is_direct
            )
        })
        .inspect(|msg| {
            #[cfg(feature = "tracing")]
            tracing::debug!(message_id = msg.id, "Message saved to database");
        })
}

/// Get all unsent broadcast messages for a topic, ordered by creation time.
pub fn get_unsent_messages(topic: &str) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    messages
        .filter(crate::generated::schema::messages::topic.eq(topic))
        .filter(crate::generated::schema::messages::sent.eq(0))
        .filter(crate::generated::schema::messages::is_direct.eq(0))
        .order(crate::generated::schema::messages::created_at.asc())
        .select(Message::as_select())
        .load(conn)
        .wrap_err_with(|| format!("Failed to load unsent messages for topic: {}", topic))
}

/// Mark a message as sent by ID.
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

/// Load broadcast messages for a topic, newest first, limited to count.
pub fn load_messages(topic: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    let msgs = messages
        .filter(crate::generated::schema::messages::topic.eq(topic))
        .filter(crate::generated::schema::messages::is_direct.eq(0))
        .order(crate::generated::schema::messages::created_at.desc())
        .limit(limit as i64)
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Load direct messages with a peer, oldest first, limited to count.
pub fn load_direct_messages(target_peer: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    let msgs = messages
        .filter(crate::generated::schema::messages::target_peer.eq(target_peer))
        .filter(crate::generated::schema::messages::is_direct.eq(1))
        .order(crate::generated::schema::messages::created_at.asc())
        .limit(limit as i64)
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Get all unsent direct messages to a specific peer, ordered by creation time.
pub fn get_unsent_direct_messages(target_peer: &str) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut crate::sqlite_connect()?;
    messages
        .filter(crate::generated::schema::messages::target_peer.eq(target_peer))
        .filter(crate::generated::schema::messages::sent.eq(0))
        .filter(crate::generated::schema::messages::is_direct.eq(1))
        .order(crate::generated::schema::messages::created_at.asc())
        .select(Message::as_select())
        .load(conn)
        .wrap_err_with(|| {
            format!(
                "Failed to load unsent direct messages for peer: {}",
                target_peer
            )
        })
}

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
    Ok(())
}

pub fn load_receipts() -> color_eyre::Result<Vec<crate::generated::models_queryable::MessageReceipt>>
{
    let conn = &mut crate::sqlite_connect()?;
    use crate::generated::models_queryable::MessageReceipt;
    use crate::generated::schema::message_receipts::dsl::message_receipts;
    use diesel::SelectableHelper;
    let receipts = message_receipts
        .select(MessageReceipt::as_select())
        .load(conn)?;
    Ok(receipts)
}
