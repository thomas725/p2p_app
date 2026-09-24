//! Canonical "Contacts" roster model (backlog §11; design doc 2026-09-21,
//! "Contacts tab" section).
//!
//! Both frontends (TUI + Flutter) render against [`ContactRow`]s. The DB-backed
//! path is [`load_contacts`]; the TUI renders its live in-memory peer list
//! through the same pure grouping builder [`build_contact_rows`]. A row is
//! either:
//!
//! - a **user contact** (`is_user == true`) — a `users` row that at least one
//!   known device is bound into via a live `device_bindings` row, collapsing
//!   all of that user's bound devices into a single row; or
//! - a **single unbound device** (`is_user == false`) — a known peer with no
//!   verified binding yet.
//!
//! Stats are aggregated across a user's devices in the same two-pass bulk query
//! used by the peer table, and display names resolve with the two-sided
//! nickname rule (owner side `contacts.nickname_override`/`peer_local_nickname`
//! first, then receiver side `users.self_nickname`/`received_nickname`).

use crate::generated::models_queryable::User;
use crate::messages::{PeerMessageStats, get_all_peer_stats};
use std::collections::HashMap;

/// A single device (peer) inside a contact row.
#[derive(Debug, Clone)]
pub struct ContactDevice {
    /// The device's libp2p peer ID.
    pub peer_id: String,
    /// Resolved device-level display name (owner → received → petname).
    pub display_name: String,
    /// First time this device was observed ("YYYY-MM-DD HH:MM:SS").
    pub first_seen: String,
    /// Most recent time this device was observed ("YYYY-MM-DD HH:MM:SS").
    pub last_seen: String,
    /// Nickname this device announced to us (receiver side of the pair).
    pub received_nickname: Option<String>,
    /// Nickname we assigned to this device locally (owner side of the pair).
    pub local_nickname: Option<String>,
}

/// A device-level row fed into [`build_contact_rows`].
///
/// Both load paths produce this shape: the DB loader (`load_contacts`) from
/// `peers` rows, and the TUI from its live in-memory peer list. Timestamps are
/// pre-formatted "YYYY-MM-DD HH:MM:SS" strings so the builder never needs a DB
/// connection or a clock.
#[derive(Debug, Clone)]
pub struct ContactDeviceInput {
    pub peer_id: String,
    pub first_seen: String,
    pub last_seen: String,
    pub received_nickname: Option<String>,
    pub local_nickname: Option<String>,
}

/// A peer group: the device inputs collapsed under one contact id, plus whether
/// that contact is a user row.
struct PeerGroup<'a> {
    is_user: bool,
    devices: Vec<&'a ContactDeviceInput>,
}

/// One row of the Contacts tab. A user's multiple bound devices are collapsed
/// into one row; the per-device detail lives in [`ContactRow::device_rows`].
#[derive(Debug, Clone)]
pub struct ContactRow {
    /// Stable key: `user_id` when [`ContactRow::is_user`], else the device
    /// `peer_id`.
    pub contact_id: String,
    /// `true` when this row is a user with ≥1 bound device.
    pub is_user: bool,
    /// Resolved row-level display name (user override → user self → best device).
    pub display_name: String,
    /// Number of devices shown for this contact.
    pub device_count: usize,
    /// The bound devices (a single element when the row is an unbound device).
    pub device_rows: Vec<ContactDevice>,
    /// Aggregate first-seen across the group ("YYYY-MM-DD HH:MM:SS").
    pub first_seen: String,
    /// Aggregate most-recent-seen across the group ("YYYY-MM-DD HH:MM:SS").
    pub last_seen: String,
    /// Sum of DMs exchanged with any of the contact's devices.
    pub dm_count: i64,
    /// Sum of broadcasts we sent that reached any of the contact's devices.
    pub broadcast_sent_to_peer: i64,
}

impl ContactRow {
    /// Peer ID of the row's most-recently-seen device.
    ///
    /// Single-device contacts (the common case) return that one device. For
    /// multi-device contacts this is the DM/PeerInfo target until explicit
    /// per-device expand lands.
    #[must_use]
    pub fn best_device_peer_id(&self) -> Option<&str> {
        self.device_rows.first().map(|d| d.peer_id.as_str())
    }
}

/// Build the canonical Contacts-tab roster from device-level inputs.
///
/// This is the grouping/aggregation heart shared by both frontends. Given the
/// device rows (`devices`), each device's resolved display name
/// (`device_display_names`), the reverse `device_bindings` lookup
/// (`peer_user` mapping device `peer_id` to `user_id`), the bulk `users` rows, this
/// install's nickname overrides, and the per-peer message stats, it collapses
/// bound devices under their user and returns rows ordered most-recently-seen
/// first.
///
/// Ties are handled by the caller (no sorting key of its own — see
/// [`sort_canonical_rows`]).
#[allow(clippy::too_many_arguments, clippy::implicit_hasher)]
#[must_use]
pub fn build_contact_rows(
    devices: &[ContactDeviceInput],
    device_display_names: &HashMap<String, String>,
    peer_user: &HashMap<String, String>,
    users: &HashMap<String, User>,
    overrides: &HashMap<String, String>,
    stats: &HashMap<String, PeerMessageStats>,
) -> Vec<ContactRow> {
    // Group devices by contact: bound devices collapse under their user id,
    // unbound devices each keep their own device id.
    let mut groups: HashMap<String, PeerGroup<'_>> = HashMap::new();
    for device in devices {
        let (key, is_user) = peer_user.get(&device.peer_id).map_or_else(
            || (device.peer_id.clone(), false),
            |uid| (uid.clone(), true),
        );
        groups
            .entry(key)
            .and_modify(|g| g.devices.push(device))
            .or_insert_with(|| PeerGroup {
                is_user,
                devices: vec![device],
            });
    }

    let mut rows: Vec<ContactRow> = Vec::with_capacity(groups.len());
    for (contact_id, group) in groups {
        rows.push(build_row(
            contact_id,
            group.is_user,
            &group.devices,
            device_display_names,
            users,
            overrides,
            stats,
        ));
    }
    rows.sort_by(|a, b| {
        let a_seen = crate::fmt::parse_last_seen_ms(&a.last_seen);
        let b_seen = crate::fmt::parse_last_seen_ms(&b.last_seen);
        b_seen
            .cmp(&a_seen)
            .then_with(|| a.contact_id.cmp(&b.contact_id))
    });
    rows
}

#[allow(clippy::too_many_arguments)]
fn build_row(
    contact_id: String,
    is_user: bool,
    group: &[&ContactDeviceInput],
    device_display_names: &HashMap<String, String>,
    users: &HashMap<String, User>,
    overrides: &HashMap<String, String>,
    stats: &HashMap<String, PeerMessageStats>,
) -> ContactRow {
    let mut devices: Vec<ContactDevice> = group
        .iter()
        .map(|d| {
            let display_name = device_display_names
                .get(&d.peer_id)
                .cloned()
                .unwrap_or_else(|| d.peer_id.clone());
            ContactDevice {
                peer_id: d.peer_id.clone(),
                display_name,
                first_seen: d.first_seen.clone(),
                last_seen: d.last_seen.clone(),
                received_nickname: d.received_nickname.clone(),
                local_nickname: d.local_nickname.clone(),
            }
        })
        .collect();
    devices.sort_by(|a, b| {
        crate::fmt::parse_last_seen_ms(&b.last_seen)
            .cmp(&crate::fmt::parse_last_seen_ms(&a.last_seen))
    });

    // A peer group always has ≥1 device (groups are built from devices). Fold
    // over the group to take the min first-seen / max last-seen directly; the
    // zero-padded "YYYY-MM-DD HH:MM:SS" strings order lexicographically like
    // chronologically.
    let mut first_seen: Option<&str> = None;
    let mut last_seen: Option<&str> = None;
    for d in group {
        first_seen = match first_seen {
            Some(cur) if cur <= d.first_seen.as_str() => Some(cur),
            _ => Some(d.first_seen.as_str()),
        };
        last_seen = match last_seen {
            Some(cur) if cur >= d.last_seen.as_str() => Some(cur),
            _ => Some(d.last_seen.as_str()),
        };
    }
    let first_seen = first_seen.unwrap_or("").to_string();
    let last_seen = last_seen.unwrap_or("").to_string();

    let dm_count = group
        .iter()
        .map(|d| stats.get(&d.peer_id).map_or(0, |s| s.dm_count))
        .sum();
    let broadcast_sent_to_peer = group
        .iter()
        .map(|d| {
            stats
                .get(&d.peer_id)
                .map_or(0, |s| s.broadcast_sent_to_peer)
        })
        .sum();

    let display_name = resolve_row_display_name(&contact_id, is_user, &devices, users, overrides);

    ContactRow {
        contact_id,
        is_user,
        display_name,
        device_count: group.len(),
        device_rows: devices,
        first_seen,
        last_seen,
        dm_count,
        broadcast_sent_to_peer,
    }
}

/// Resolve the row-level display name with the two-sided nickname rule.
///
/// User rows: our `contacts.nickname_override` for that user wins; otherwise
/// the user's own announced `users.self_nickname`; otherwise the
/// most-recently-seen device's display name. Unbound device rows keep their
/// single device's display name.
fn resolve_row_display_name(
    contact_id: &str,
    is_user: bool,
    devices: &[ContactDevice],
    users: &HashMap<String, User>,
    overrides: &HashMap<String, String>,
) -> String {
    if is_user {
        let owner_side = overrides.get(contact_id);
        let receiver_side = users.get(contact_id).and_then(|u| u.self_nickname.as_ref());
        if let Some(name) = owner_side {
            return format_contact_name(name, contact_id);
        }
        if let Some(name) = receiver_side {
            return format_contact_name(name, contact_id);
        }
    }
    devices
        .first()
        .map_or_else(|| contact_id.to_string(), |d| d.display_name.clone())
}

/// Format a contact display name as `"name (suffix)"`, mirroring the device
/// display-name convention.
fn format_contact_name(name: &str, contact_id: &str) -> String {
    format!("{name} ({})", crate::fmt::peer_id_suffix(contact_id))
}

/// Produce the canonical Contacts-tab roster from the database.
///
/// Loads every known peer once, resolves each to its user via the bulk
/// `device_bindings` reverse lookup, groups by user, and aggregates the
/// per-peer message statistics into per-contact sums. Unbound peers each stay
/// a single-device contact. Rows are ordered most-recently-seen first (ties on
/// contact id).
///
/// The owner-side nickname override is resolved against this install's own
/// user (the `contacts` owner) when this install is itself bound to a user.
///
/// # Errors
/// Returns an error if any database query fails.
pub fn load_contacts() -> color_eyre::Result<Vec<ContactRow>> {
    let peers = crate::peers::load_peers()?;
    let stats = get_all_peer_stats()?;

    let peer_ids: Vec<String> = peers.iter().map(|p| p.peer_id.clone()).collect();
    let peer_user = crate::identity::find_user_ids_for_peers(&peer_ids)?;

    // Bulk-fetch every user that owns a bound device.
    let user_ids: Vec<String> = {
        let mut set: Vec<String> = peer_user.values().cloned().collect();
        set.sort();
        set.dedup();
        set
    };
    let users = crate::identity::find_users(&user_ids)?;

    // This install's own user id (the owner of any nickname override).
    let owner_id = crate::get_local_peer_id()
        .ok()
        .and_then(|id| crate::identity::find_user_id_for_peer(&id.to_string()).ok())
        .flatten();
    let overrides: HashMap<String, String> = owner_id
        .as_ref()
        .and_then(|owner| crate::identity::list_contacts(owner).ok())
        .map(|contacts| {
            contacts
                .into_iter()
                .filter_map(|c| c.nickname_override.map(|n| (c.contact_user_id, n)))
                .collect()
        })
        .unwrap_or_default();

    let devices: Vec<ContactDeviceInput> = peers
        .iter()
        .map(|p| ContactDeviceInput {
            peer_id: p.peer_id.clone(),
            first_seen: crate::fmt::format_peer_datetime(p.first_seen),
            last_seen: crate::fmt::format_peer_datetime(p.last_seen),
            received_nickname: p.received_nickname.clone(),
            local_nickname: p.peer_local_nickname.clone(),
        })
        .collect();
    let device_display_names: HashMap<String, String> = peers
        .iter()
        .map(|p| {
            let name =
                crate::get_peer_display_name(&p.peer_id).unwrap_or_else(|_| p.peer_id.clone());
            (p.peer_id.clone(), name)
        })
        .collect();

    Ok(build_contact_rows(
        &devices,
        &device_display_names,
        &peer_user,
        &users,
        &overrides,
        &stats,
    ))
}

/// Sort rows by column/order, following the caller's sort conventions.
///
/// Column `0` display name, `1` DM count, `2` broadcast count, `3` last seen,
/// `4` first seen (any other value falls back to name). Every column ties on
/// the contact's stable key (`contact_id`), so the result is total and
/// deterministic.
#[must_use]
pub fn sort_canonical_rows(
    rows: &[ContactRow],
    sort_column: usize,
    ascending: bool,
) -> Vec<ContactRow> {
    let mut keyed: Vec<(ContactRow, String)> = rows
        .iter()
        .map(|r| {
            let name_key = r.display_name.to_lowercase();
            ((*r).clone(), name_key)
        })
        .collect();
    keyed.sort_by(|(a, an), (b, bn)| {
        let ord = match sort_column {
            1 => a
                .dm_count
                .cmp(&b.dm_count)
                .then_with(|| a.contact_id.cmp(&b.contact_id)),
            2 => a
                .broadcast_sent_to_peer
                .cmp(&b.broadcast_sent_to_peer)
                .then_with(|| a.contact_id.cmp(&b.contact_id)),
            3 => crate::fmt::parse_last_seen_ms(&a.last_seen)
                .cmp(&crate::fmt::parse_last_seen_ms(&b.last_seen))
                .then_with(|| a.contact_id.cmp(&b.contact_id)),
            4 => crate::fmt::parse_last_seen_ms(&a.first_seen)
                .cmp(&crate::fmt::parse_last_seen_ms(&b.first_seen))
                .then_with(|| a.contact_id.cmp(&b.contact_id)),
            _ => an.cmp(bn).then_with(|| a.contact_id.cmp(&b.contact_id)),
        };
        if ascending { ord } else { ord.reverse() }
    });
    keyed.into_iter().map(|(r, _)| r).collect()
}

#[cfg(test)]
#[path = "../tests/unit/unit_contacts.rs"]
mod tests;
