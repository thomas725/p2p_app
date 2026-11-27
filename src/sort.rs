//! Canonical five-column sort shared by every frontend.
//!
//! The TUI Contacts table and both Flutter tables order the same columns with
//! the same rule: lowercase display name, two aggregate counts, two canonical
//! timestamps, and a stable-id tie-break. Keeping that rule here prevents the
//! three call sites from drifting (a previous copy used a different fallback
//! column for an unknown sort key).
//!
//! A descending sort reverses the **whole** ordering, including the tie-break —
//! never just the primary column.

use crate::fmt::parse_last_seen_ms;

/// The columns a Contacts/peers table can sort on.
///
/// Implementors only expose the values the comparator reads, so each frontend
/// row type stays a thin adapter and the ordering rule has one owner.
pub trait SortableRow {
    /// Display name, compared case-insensitively by column 0.
    fn display_name(&self) -> &str;

    /// Stable tie-break key (`contact_id` or `peer_id`).
    fn stable_id(&self) -> &str;

    /// Most recent activity, parsed by [`parse_last_seen_ms`] for column 3.
    fn last_seen(&self) -> &str;

    /// First activity, parsed by [`parse_last_seen_ms`] for column 4.
    fn first_seen(&self) -> &str;

    /// Direct messages exchanged, used by column 1.
    fn dm_count(&self) -> i64;

    /// Broadcasts sent to this row, used by column 2.
    fn broadcasts_sent(&self) -> i64;
}

/// Sort owned rows by column/order and return them in the requested order.
///
/// Column `0` is the lowercase display name, `1` DM count, `2` broadcasts sent,
/// `3` last seen, `4` first seen. Any other column falls back to the name.
/// Seen columns use [`parse_last_seen_ms`] (0 on unparseable input); every column
/// ties on [`SortableRow::stable_id`], and descending reverses that tie-break
/// too.
#[must_use]
pub fn sort_rows_by_column<T: SortableRow>(
    rows: Vec<T>,
    sort_column: u32,
    ascending: bool,
) -> Vec<T> {
    let mut keyed: Vec<(T, String)> = rows
        .into_iter()
        .map(|row| {
            let name_key = row.display_name().to_lowercase();
            (row, name_key)
        })
        .collect();
    keyed.sort_by(|(a, an), (b, bn)| {
        let ord = match sort_column {
            1 => a.dm_count().cmp(&b.dm_count()),
            2 => a.broadcasts_sent().cmp(&b.broadcasts_sent()),
            3 => parse_last_seen_ms(a.last_seen()).cmp(&parse_last_seen_ms(b.last_seen())),
            4 => parse_last_seen_ms(a.first_seen()).cmp(&parse_last_seen_ms(b.first_seen())),
            _ => an.cmp(bn),
        }
        .then_with(|| a.stable_id().cmp(b.stable_id()));
        if ascending { ord } else { ord.reverse() }
    });
    keyed.into_iter().map(|(row, _)| row).collect()
}

#[cfg(test)]
#[path = "../tests/unit/unit_sort.rs"]
mod tests;
