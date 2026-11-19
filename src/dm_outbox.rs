//! Store-and-forward DM outbox (backlog item).
//!
//! A DM is sealed with the recipient's identity-derived X25519 key before it
//! leaves the node (US-042). When the recipient is offline the sealed envelope
//! is parked here so it can be retried on reconnect and by a periodic sweep.
//! Only the *sealed* wire message is stored: the envelope's plaintext fields
//! (`content`, `nickname`, `attachment`, `reply_to`) are deliberately empty, so
//! a dump of this table reveals routing fields and ciphertext, never the body.
//!
//! Lifecycle:
//!
//! - [`queue`] is called from the `send_dm` path after sealing; the unique
//!   `(recipient, msg_id)` index makes a resend/replay a no-op.
//! - [`remove`] is called when the recipient's ack (`ack_for = msg_id`) arrives.
//! - [`pending`] / [`pending_for`] feed the reconnect and periodic retries.
//! - [`prune_expired`] bounds retention so undeliverable envelopes do not live
//!   forever.

use crate::generated::models_insertable::NewDmOutbox;
use crate::generated::models_queryable::DmOutboxRow;
use crate::generated::schema::dm_outbox::dsl as dm_dsl;
use color_eyre::eyre::WrapErr as _;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _};

/// Default retention window for a queued sealed DM (7 days).
pub const DEFAULT_TTL_SECS: i64 = 7 * 24 * 60 * 60;

/// Queue a sealed DM envelope for later delivery.
///
/// Returns `true` if a new row was inserted, `false` if `(recipient, msg_id)`
/// was already queued — the unique index keeps a resend/replay idempotent.
///
/// # Errors
/// Returns an error if the database write fails.
pub fn queue(
    recipient_peer_id: &str,
    msg_id: &str,
    envelope_json: &str,
    ttl_secs: i64,
) -> color_eyre::Result<bool> {
    let conn = &mut crate::sqlite_connect()?;
    let now = chrono::Utc::now().naive_utc();
    let expires_at = now
        .checked_add_signed(chrono::Duration::seconds(ttl_secs))
        .unwrap_or(now);
    let row = NewDmOutbox {
        recipient_peer_id: recipient_peer_id.to_string(),
        msg_id: msg_id.to_string(),
        envelope_json: envelope_json.to_string(),
        expires_at,
    };
    let inserted = diesel::insert_into(dm_dsl::dm_outbox)
        .values(&row)
        .on_conflict_do_nothing()
        .execute(conn)
        .wrap_err("failed to queue sealed DM")?;
    Ok(inserted == 1)
}

/// Load all non-expired rows waiting for delivery, oldest first.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn pending() -> color_eyre::Result<Vec<DmOutboxRow>> {
    let conn = &mut crate::sqlite_connect()?;
    let now = chrono::Utc::now().naive_utc();
    let rows = dm_dsl::dm_outbox
        .filter(dm_dsl::expires_at.gt(now))
        .order(dm_dsl::id.asc())
        .select(DmOutboxRow::as_select())
        .load(conn)
        .wrap_err("failed to load pending DMs")?;
    Ok(rows)
}

/// Load non-expired rows for one recipient, oldest first (reconnect flush).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn pending_for(recipient_peer_id: &str) -> color_eyre::Result<Vec<DmOutboxRow>> {
    let conn = &mut crate::sqlite_connect()?;
    let now = chrono::Utc::now().naive_utc();
    let rows = dm_dsl::dm_outbox
        .filter(dm_dsl::recipient_peer_id.eq(recipient_peer_id))
        .filter(dm_dsl::expires_at.gt(now))
        .order(dm_dsl::id.asc())
        .select(DmOutboxRow::as_select())
        .load(conn)
        .wrap_err("failed to load pending DMs for peer")?;
    Ok(rows)
}

/// Delete a delivered row on ack. Returns the number of rows removed (0 or 1).
///
/// # Errors
/// Returns an error if the database write fails.
pub fn remove(recipient_peer_id: &str, msg_id: &str) -> color_eyre::Result<usize> {
    let conn = &mut crate::sqlite_connect()?;
    let removed = diesel::delete(
        dm_dsl::dm_outbox
            .filter(dm_dsl::recipient_peer_id.eq(recipient_peer_id))
            .filter(dm_dsl::msg_id.eq(msg_id)),
    )
    .execute(conn)
    .wrap_err("failed to clear delivered DM from outbox")?;
    Ok(removed)
}

/// Bump the attempt counter for a row after a delivery try.
///
/// # Errors
/// Returns an error if the database write fails.
pub fn record_attempt(id: i32) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let current: i32 = dm_dsl::dm_outbox
        .filter(dm_dsl::id.eq(id))
        .select(dm_dsl::attempts)
        .first(conn)
        .wrap_err("failed to read DM attempt count")?;
    diesel::update(dm_dsl::dm_outbox.filter(dm_dsl::id.eq(id)))
        .set(dm_dsl::attempts.eq(current.saturating_add(1)))
        .execute(conn)
        .wrap_err("failed to record DM delivery attempt")?;
    Ok(())
}

/// Delete expired rows (retention). Returns the number of rows removed.
///
/// # Errors
/// Returns an error if the database write fails.
pub fn prune_expired() -> color_eyre::Result<usize> {
    let conn = &mut crate::sqlite_connect()?;
    let now = chrono::Utc::now().naive_utc();
    let removed = diesel::delete(dm_dsl::dm_outbox.filter(dm_dsl::expires_at.le(now)))
        .execute(conn)
        .wrap_err("failed to prune expired DMs")?;
    Ok(removed)
}

/// Total number of queued rows (used by tests and diagnostics).
///
/// # Errors
/// Returns an error if the database query fails.
pub fn count() -> color_eyre::Result<i64> {
    let conn = &mut crate::sqlite_connect()?;
    let total = dm_dsl::dm_outbox
        .count()
        .get_result(conn)
        .wrap_err("failed to count queued DMs")?;
    Ok(total)
}

#[cfg(test)]
#[path = "../tests/unit/unit_dm_outbox.rs"]
mod tests;
