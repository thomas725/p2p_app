//! Persistent per-message reactions (US-008).
//!
//! A reaction is `(msg_id, peer_id, emoji)`: the stable wire id of the message,
//! the reactor's transport peer id, and a short emoji token. Keying on `msg_id`
//! (rather than the local row id) means the same table serves broadcast, DM and
//! group messages, and a reaction can be correlated across peers. The unique
//! index makes both a re-applied toggle and a replayed network signal idempotent.
//!
//! Reactions are *additive* social data: removing one deletes a row, never a
//! message. Peers exchange them on a dedicated request-response lane
//! ([`crate::behavior::REACTION_PROTOCOL_NAME`]); the reactor applies its own
//! change locally and sends the same signal to the peers that need to see it.

use crate::generated::models_insertable::NewMessageReaction;
use crate::generated::models_queryable::MessageReactionRow;
use crate::generated::schema::message_reactions::dsl as reactions;
use crate::generated::schema::{group_members, group_messages, messages};
use color_eyre::eyre::WrapErr as _;
use diesel::{
    ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
};
use std::collections::HashMap;
use std::hash::BuildHasher;

/// Maximum length of an emoji token accepted from a frontend or the network.
///
/// Long enough for a compound emoji with variation selectors and a ZWJ sequence,
/// short enough that a hostile peer cannot stuff arbitrary text into a chip.
pub const MAX_EMOJI_LEN: usize = 32;

/// Default tokens offered when a user adds a reaction.
///
/// The TUI's `1`–`6` shortcuts and Flutter's palette menu both read this list,
/// so the affordance cannot drift between frontends. Tokens are still passed
/// through [`validate_emoji`] before they reach the database.
pub const QUICK_REACTIONS: [&str; 6] = ["👍", "❤️", "😂", "🎉", "😮", "😢"];

/// Return the palette token at a zero-based position.
#[must_use]
pub fn quick_reaction(index: usize) -> Option<&'static str> {
    QUICK_REACTIONS.get(index).copied()
}

/// Owned palette for the FRB/web boundaries, which cannot expose a static slice.
#[must_use]
pub fn quick_reactions() -> Vec<String> {
    QUICK_REACTIONS
        .iter()
        .map(|token| (*token).to_string())
        .collect()
}

/// Compact `1 token  2 token …` hint for the TUI reaction editor title.
#[must_use]
pub fn quick_reaction_hint() -> String {
    QUICK_REACTIONS
        .iter()
        .enumerate()
        .map(|(index, token)| format!("{} {token}", index.saturating_add(1)))
        .collect::<Vec<_>>()
        .join("  ")
}

/// Aggregated reactions on one message, ready for either frontend to render.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionSummary {
    /// Emoji token (glyph or shortcode) shared by all reactors in this group.
    pub emoji: String,
    /// Number of distinct peers using this emoji.
    pub count: i32,
    /// Whether this install's own peer is among the reactors.
    pub mine: bool,
}

/// Validate an inbound emoji token, returning a trimmed owned copy.
///
/// Rejects empty and overlong tokens so neither a frontend nor a remote peer can
/// persist unbounded strings. Whitespace around a pasted emoji is tolerated.
///
/// # Errors
/// Returns a description when the token is empty or too long.
pub fn validate_emoji(emoji: &str) -> Result<String, String> {
    let trimmed = emoji.trim();
    if trimmed.is_empty() {
        return Err("emoji must not be empty".into());
    }
    if trimmed.chars().count() > MAX_EMOJI_LEN {
        return Err(format!("emoji is longer than {MAX_EMOJI_LEN} characters"));
    }
    Ok(trimmed.to_string())
}

/// Apply a reaction change for one peer, returning whether the reaction is now
/// present (`true` = added, `false` = removed).
///
/// Idempotent: inserting an existing `(msg, peer, emoji)` and deleting a missing
/// one are both no-ops, so replayed signals and double clicks cannot inflate a
/// count.
///
/// # Errors
/// Returns an error if the database write fails or the emoji is invalid.
pub fn apply(msg_id: &str, peer_id: &str, emoji: &str, remove: bool) -> Result<bool, String> {
    let emoji = validate_emoji(emoji)?;
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let existing = reactions::message_reactions
        .filter(reactions::msg_id.eq(msg_id))
        .filter(reactions::peer_id.eq(peer_id))
        .filter(reactions::emoji.eq(&emoji))
        .select(MessageReactionRow::as_select())
        .first(conn)
        .optional()
        .wrap_err("failed to read reaction")
        .map_err(|e| e.to_string())?;

    if remove {
        if existing.is_some() {
            diesel::delete(
                reactions::message_reactions
                    .filter(reactions::msg_id.eq(msg_id))
                    .filter(reactions::peer_id.eq(peer_id))
                    .filter(reactions::emoji.eq(&emoji)),
            )
            .execute(conn)
            .wrap_err("failed to remove reaction")
            .map_err(|e| e.to_string())?;
        }
        Ok(false)
    } else {
        if existing.is_none() {
            let row = NewMessageReaction {
                msg_id: msg_id.to_string(),
                peer_id: peer_id.to_string(),
                emoji,
            };
            diesel::insert_into(reactions::message_reactions)
                .values(&row)
                .on_conflict_do_nothing()
                .execute(conn)
                .wrap_err("failed to add reaction")
                .map_err(|e| e.to_string())?;
        }
        Ok(true)
    }
}

/// Whether a given peer currently uses an emoji on a message.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn has_reaction(msg_id: &str, peer_id: &str, emoji: &str) -> Result<bool, String> {
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let found = reactions::message_reactions
        .filter(reactions::msg_id.eq(msg_id))
        .filter(reactions::peer_id.eq(peer_id))
        .filter(reactions::emoji.eq(emoji))
        .select(reactions::id)
        .first::<i32>(conn)
        .optional()
        .wrap_err("failed to read reaction")
        .map_err(|e| e.to_string())?;
    Ok(found.is_some())
}

/// Peers that need to hear about a reaction on a given message (US-008).
///
/// A DM reaction goes only to the conversation partner; a broadcast reaction
/// fans out to every known peer; a group reaction goes to that group's members.
/// The local peer is always excluded. Unknown message ids yield an empty list
/// (there is nothing to sync), which is also the fail-open behaviour on a DB
/// error — a broken query must never block a local reaction.
#[must_use]
pub fn recipients_for_message(msg_id: &str) -> Vec<String> {
    let Ok(conn) = crate::sqlite_connect() else {
        return Vec::new();
    };
    let mut conn = conn;

    // A direct message has exactly one partner; a broadcast reaches everyone.
    let dm: Option<(i32, i32, Option<String>, Option<String>)> = messages::table
        .filter(messages::msg_id.eq(msg_id))
        .select((
            messages::sent,
            messages::is_direct,
            messages::peer_id,
            messages::target_peer,
        ))
        .first(&mut conn)
        .optional()
        .ok()
        .flatten();
    if let Some((sent, is_direct, peer_id, target_peer)) = dm {
        if is_direct == 1 {
            let partner = if sent == 1 { target_peer } else { peer_id };
            return partner.into_iter().collect();
        }
        return crate::peers::load_peers()
            .map_or_default(|peers| peers.into_iter().map(|p| p.peer_id).collect());
    }

    // Group messages sync to the group's members only.
    let group_id: Option<String> = group_messages::table
        .filter(group_messages::msg_id.eq(msg_id))
        .select(group_messages::group_id)
        .first(&mut conn)
        .optional()
        .ok()
        .flatten();
    if let Some(group_id) = group_id {
        // The local device is recorded as a group member when we accept an
        // invite, but reacting is a local action — echoing the signal back to
        // ourselves would just re-emit a redundant reaction event. Match the
        // broadcast branch, which excludes self via `load_peers()`.
        let local_peer = crate::peers::local_peer_id_string();
        return group_members::table
            .filter(group_members::group_id.eq(group_id))
            .select(group_members::peer_id)
            .load::<String>(&mut conn)
            .unwrap_or_default()
            .into_iter()
            .filter(|peer| local_peer.as_ref() != Some(peer))
            .collect();
    }

    Vec::new()
}

/// Extend a UI reaction map with summaries for the given wire message ids.
///
/// History loaders call this once with their aligned id queue so a freshly
/// opened chat shows stored chips without one query per row. Entries for ids
/// with no reactions are simply absent, which renderers treat as no chips.
pub fn hydrate_into<S: BuildHasher>(
    map: &mut HashMap<String, Vec<ReactionSummary>, S>,
    msg_ids: impl IntoIterator<Item = Option<String>>,
) {
    let ids: Vec<String> = msg_ids.into_iter().flatten().collect();
    map.extend(summaries_for_many(&ids));
}

/// Aggregate raw rows into summaries, marking this install's own reactions.
fn summarize(rows: Vec<MessageReactionRow>, mine_peer: Option<&str>) -> Vec<ReactionSummary> {
    let mut out: Vec<ReactionSummary> = Vec::new();
    for row in rows {
        let mine = mine_peer.is_some_and(|peer| peer == row.peer_id);
        if let Some(summary) = out.iter_mut().find(|s| s.emoji == row.emoji) {
            summary.count = summary.count.saturating_add(1);
            summary.mine |= mine;
        } else {
            out.push(ReactionSummary {
                emoji: row.emoji,
                count: 1,
                mine,
            });
        }
    }
    out
}

/// Load the aggregated reactions for one message.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn summaries_for(msg_id: &str) -> Result<Vec<ReactionSummary>, String> {
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    let rows = reactions::message_reactions
        .filter(reactions::msg_id.eq(msg_id))
        .order(reactions::id.asc())
        .select(MessageReactionRow::as_select())
        .load(conn)
        .wrap_err("failed to load reactions")
        .map_err(|e| e.to_string())?;
    Ok(summarize(
        rows,
        crate::peers::local_peer_id_string().as_deref(),
    ))
}

/// Load reactions for many messages in one query, keyed by message id.
///
/// Loaders call this once per history fetch so a screen with hundreds of
/// messages costs one round trip, not one per row.
#[must_use]
pub fn summaries_for_many(
    msg_ids: &[String],
) -> std::collections::HashMap<String, Vec<ReactionSummary>> {
    let mut result = std::collections::HashMap::new();
    if msg_ids.is_empty() {
        return result;
    }
    let Ok(conn) = crate::sqlite_connect() else {
        return result;
    };
    let mut conn = conn;
    let Ok(rows) = reactions::message_reactions
        .filter(reactions::msg_id.eq_any(msg_ids))
        .order(reactions::id.asc())
        .select(MessageReactionRow::as_select())
        .load::<MessageReactionRow>(&mut conn)
    else {
        return result;
    };
    let mine = crate::peers::local_peer_id_string();
    let mut grouped: std::collections::HashMap<String, Vec<MessageReactionRow>> =
        std::collections::HashMap::new();
    for row in rows {
        grouped.entry(row.msg_id.clone()).or_default().push(row);
    }
    for (msg_id, group) in grouped {
        result.insert(msg_id, summarize(group, mine.as_deref()));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn db_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn with_test_db(f: impl FnOnce()) {
        let _guard = db_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().expect("tempdir");
        let db_path = dir.path().join("test.db");
        crate::db::set_db_url(db_path.to_str().expect("db path"));
        crate::db::init_database().expect("init db");
        f();
        crate::db::release_db_lock();
        crate::db::reset_db_url();
    }

    #[test]
    fn validate_emoji_trims_and_rejects_bad_input() {
        assert_eq!(validate_emoji("  👍 ").expect("trim"), "👍");
        assert!(validate_emoji("   ").is_err());
        let too_long = "x".repeat(MAX_EMOJI_LEN + 1);
        assert!(validate_emoji(&too_long).is_err());
    }

    #[test]
    fn quick_reaction_palette_is_indexable_and_valid() {
        assert_eq!(quick_reaction(0), Some("👍"));
        assert_eq!(quick_reaction(QUICK_REACTIONS.len()), None);
        assert_eq!(quick_reactions(), vec!["👍", "❤️", "😂", "🎉", "😮", "😢"]);
        assert_eq!(quick_reaction_hint(), "1 👍  2 ❤️  3 😂  4 🎉  5 😮  6 😢");
        for token in QUICK_REACTIONS {
            assert_eq!(validate_emoji(token).expect("valid palette token"), token);
        }
    }

    #[test]
    fn summarize_counts_and_marks_mine() {
        let make = |id: i32, peer: &str, emoji: &str| MessageReactionRow {
            id,
            msg_id: "m1".into(),
            peer_id: peer.into(),
            emoji: emoji.into(),
            created_at: chrono::Utc::now().naive_utc(),
        };
        let rows = vec![
            make(1, "peer-a", "👍"),
            make(2, "peer-b", "👍"),
            make(3, "peer-a", "❤️"),
        ];
        let out = summarize(rows, Some("peer-b"));
        assert_eq!(out.len(), 2);
        let thumbs = out.iter().find(|s| s.emoji == "👍").expect("thumbs");
        assert_eq!(thumbs.count, 2);
        assert!(thumbs.mine);
        let heart = out.iter().find(|s| s.emoji == "❤️").expect("heart");
        assert_eq!(heart.count, 1);
        assert!(!heart.mine);
    }

    #[test]
    #[serial]
    fn apply_is_idempotent_and_toggle_removes() {
        with_test_db(|| {
            assert!(apply("m1", "peer-a", "👍", false).expect("add"));
            // Re-adding an existing row must not inflate the count.
            assert!(apply("m1", "peer-a", "👍", false).expect("re-add"));
            let rows = summaries_for("m1").expect("summaries");
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].count, 1);
            // Removing a missing reaction is a no-op, not an error.
            assert!(!apply("m1", "nobody", "🎉", true).expect("remove missing"));
            assert!(!apply("m1", "peer-a", "👍", true).expect("remove"));
            assert_eq!(summaries_for("m1").expect("summaries"), vec![]);
        });
    }

    #[test]
    #[serial]
    fn summaries_for_many_batches_and_hydrate_skips_empty() {
        with_test_db(|| {
            apply("m1", "peer-a", "👍", false).expect("add m1");
            apply("m1", "peer-b", "👍", false).expect("add m1 second");
            apply("m2", "peer-a", "❤️", false).expect("add m2");
            let map = summaries_for_many(&["m1".into(), "m2".into(), "m3".into()]);
            assert_eq!(map.len(), 2, "m3 has no reactions and must be absent");
            assert_eq!(map["m1"][0].count, 2);
            assert_eq!(map["m2"][0].emoji, "❤️");
            let mut hydrated = std::collections::HashMap::new();
            hydrate_into(&mut hydrated, [Some("m1".into()), None, Some("m3".into())]);
            assert_eq!(hydrated.len(), 1);
            assert_eq!(hydrated["m1"][0].count, 2);
        });
    }

    #[test]
    #[serial]
    fn long_emoji_is_rejected_by_apply() {
        with_test_db(|| {
            let too_long = "x".repeat(MAX_EMOJI_LEN + 1);
            assert!(apply("m1", "peer-a", &too_long, false).is_err());
        });
    }

    #[test]
    #[serial]
    fn group_reaction_fanout_excludes_local_peer() {
        with_test_db(|| {
            let local = crate::get_local_peer_id()
                .expect("local identity")
                .to_string();
            let remote = libp2p_identity::Keypair::generate_ed25519()
                .public()
                .to_peer_id()
                .to_string();

            crate::groups::ensure_group_for_invite("g-react").expect("group");
            crate::groups::record_group_member("g-react", &local).expect("local member");
            crate::groups::record_group_member("g-react", &remote).expect("remote member");
            crate::groups::save_outgoing_group_message(
                "g-react",
                "hello",
                crate::groups::GroupMessageMeta {
                    msg_id: Some("gm-1".into()),
                    ..Default::default()
                },
            )
            .expect("group message");

            let recipients = recipients_for_message("gm-1");
            assert!(
                recipients.contains(&remote),
                "the remote member must receive the reaction"
            );
            assert!(
                !recipients.contains(&local),
                "the local peer must never receive its own reaction"
            );
        });
    }
}
