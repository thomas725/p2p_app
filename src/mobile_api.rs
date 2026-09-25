//! Thin API facade for mobile hosts.
//!
//! Keep this module smaller and more stable than the internal Rust API. Flutter,
//! Android services, and future iOS bindings should depend on this facade rather
//! than reaching into TUI/desktop-oriented modules directly.

use chrono::TimeZone;

/// Format an ISO timestamp string ("YYYY-MM-DD HH:MM:SS") to "HH:MM".
///
/// Used by Dart/Flutter UI to display message timestamps. Uses a checked
/// character slice so a string shorter than expected never panics.
#[must_use]
#[flutter_rust_bridge::frb(sync)]
pub fn format_time_hhmm(dt: &str) -> String {
    match dt.get(11..16) {
        Some(hhmm) => hhmm.to_string(),
        None => dt.to_string(),
    }
}

/// Check if scroll position indicates at bottom
///
/// Returns true if the scroll offset is at or past the point where the bottom
/// of the content becomes visible given the visible area size.
#[must_use]
#[flutter_rust_bridge::frb(sync)]
#[flutter_rust_bridge::frb(type_64bit_int)]
pub fn is_at_bottom(scroll_offset: usize, total: usize, visible: usize) -> bool {
    scroll_offset >= total.saturating_sub(visible)
}

/// Parse a `YYYY-MM-DD HH:MM:SS` (or `...T...`) timestamp into milliseconds
/// since epoch; 0 for any unparseable input.
///
/// Delegates to the canonical [`crate::fmt::parse_last_seen_ms`].
#[must_use]
#[flutter_rust_bridge::frb(sync, type_64bit_int)]
pub fn parse_last_seen_ms(last_seen: &str) -> u64 {
    crate::fmt::parse_last_seen_ms(last_seen)
}

/// One row of the peers table, as fed to [`sort_peers`].
#[derive(Debug, Clone)]
pub struct PeerSortInput {
    pub peer_id: String,
    pub display_name: String,
    pub last_seen: String,
    pub first_seen: String,
    pub dm_count: u32,
    pub broadcast_count: u32,
}

/// A known peer joined with its message counts, fetched in a single call.
///
/// Flutter populates both the peer list and the peer table's count columns
/// from this one response, avoiding the N+1 round trip over
/// [`crate::messages::get_all_peer_stats`].
#[derive(Debug, Clone)]
pub struct PeerWithStats {
    pub peer_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub nickname: Option<String>,
    pub local_nickname: Option<String>,
    pub dm_count: i64,
    pub broadcast_sent_to_peer: i64,
}

/// Load all known peers and their message statistics in a single round trip.
///
/// Combines the peer list (`get_known_peers`) with per-peer stats via the
/// bulk [`crate::messages::get_all_peer_stats`] (one grouped query), mirroring
/// what Flutter's `_refreshPeers` does with N+1 calls, so the UI needs only
/// one FFI call regardless of peer count.
///
/// # Errors
/// Returns an error if the peers or their stats cannot be loaded.
pub fn get_peers_with_stats() -> Result<Vec<PeerWithStats>, String> {
    let peers = crate::mobile_node::get_known_peers()?;
    let stats = crate::messages::get_all_peer_stats().map_err(|e| e.to_string())?;
    let mut rows = Vec::with_capacity(peers.len());
    for p in peers {
        let stats = stats.get(&p.peer_id);
        rows.push(PeerWithStats {
            peer_id: p.peer_id,
            display_name: p.display_name,
            first_seen: p.first_seen,
            last_seen: p.last_seen,
            nickname: p.nickname,
            local_nickname: p.local_nickname,
            dm_count: stats.map_or(0, |s| s.dm_count),
            broadcast_sent_to_peer: stats.map_or(0, |s| s.broadcast_sent_to_peer),
        });
    }
    Ok(rows)
}

/// One device row inside a contact's roster entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileContactDevice {
    pub peer_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub received_nickname: Option<String>,
    pub local_nickname: Option<String>,
}

/// One row of the Contacts tab, as exposed to Flutter.
///
/// A user's bound devices are collapsed into a single row
/// ([`MobileContactRow::device_rows`]); unbound peers appear as single-device
/// rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileContactRow {
    /// Stable key: `user_id` when [`MobileContactRow::is_user`], else `peer_id`.
    pub contact_id: String,
    pub is_user: bool,
    pub display_name: String,
    pub device_count: usize,
    pub device_rows: Vec<MobileContactDevice>,
    pub first_seen: String,
    pub last_seen: String,
    pub dm_count: i64,
    pub broadcast_sent_to_peer: i64,
}

fn contact_device_from(device: crate::contacts::ContactDevice) -> MobileContactDevice {
    MobileContactDevice {
        peer_id: device.peer_id,
        display_name: device.display_name,
        first_seen: device.first_seen,
        last_seen: device.last_seen,
        received_nickname: device.received_nickname,
        local_nickname: device.local_nickname,
    }
}

fn contact_row_from(row: crate::contacts::ContactRow) -> MobileContactRow {
    MobileContactRow {
        contact_id: row.contact_id,
        is_user: row.is_user,
        display_name: row.display_name,
        device_count: row.device_count,
        device_rows: row
            .device_rows
            .into_iter()
            .map(contact_device_from)
            .collect(),
        first_seen: row.first_seen,
        last_seen: row.last_seen,
        dm_count: row.dm_count,
        broadcast_sent_to_peer: row.broadcast_sent_to_peer,
    }
}

/// Load the canonical Contacts-tab roster in a single round trip.
///
/// Mirrors [`crate::contacts::load_contacts`] for Flutter: bound devices
/// collapse into user rows, unbound peers stay single-device rows, and stats
/// are aggregated per contact.
///
/// # Errors
/// Returns an error if the roster cannot be loaded.
#[flutter_rust_bridge::frb(ignore)]
pub fn get_contacts_with_stats() -> Result<Vec<MobileContactRow>, String> {
    crate::contacts::load_contacts()
        .map(|rows| rows.into_iter().map(contact_row_from).collect())
        .map_err(|e| e.to_string())
}

/// Sort known peers by the given column and order, mirroring the TUI peers
/// table: column `0` name, `1` DM count, `2` broadcast count, `3` last seen,
/// `4` first seen (any other value falls back to name). Ordering always ties
/// on `peer_id`, so the result is total and stable for a given dataset.
#[must_use]
#[flutter_rust_bridge::frb(sync)]
pub fn sort_peers(
    peers: Vec<PeerSortInput>,
    sort_column: u32,
    ascending: bool,
) -> Vec<PeerSortInput> {
    let mut keyed: Vec<(PeerSortInput, String)> = peers
        .into_iter()
        .map(|p| {
            let name_key = p.display_name.to_lowercase();
            (p, name_key)
        })
        .collect();
    keyed.sort_by(|(a, an), (b, bn)| {
        let ord = match sort_column {
            1 => a
                .dm_count
                .cmp(&b.dm_count)
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
            2 => a
                .broadcast_count
                .cmp(&b.broadcast_count)
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
            3 => crate::fmt::parse_last_seen_ms(&a.last_seen)
                .cmp(&crate::fmt::parse_last_seen_ms(&b.last_seen))
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
            4 => crate::fmt::parse_last_seen_ms(&a.first_seen)
                .cmp(&crate::fmt::parse_last_seen_ms(&b.first_seen))
                .then_with(|| a.peer_id.cmp(&b.peer_id)),
            _ => an.cmp(bn).then_with(|| a.peer_id.cmp(&b.peer_id)),
        };
        if ascending { ord } else { ord.reverse() }
    });
    keyed.into_iter().map(|(p, _)| p).collect()
}

/// One contact row of the Contacts tab, as fed to [`sort_contacts`].
///
/// Flutter rebuilds this from `MobileContactRow`s; the sort only needs the
/// columns it orders on, so the FFI round trip stays as light as `sort_peers`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSortInput {
    /// Stable key: `user_id` when the contact is a bound user, else `peer_id`.
    pub contact_id: String,
    pub display_name: String,
    pub last_seen: String,
    pub first_seen: String,
    pub dm_count: i64,
    pub broadcast_sent_to_peer: i64,
}

/// Sort a Contacts-tab roster by the given column and order, mirroring the
/// canonical `contacts::sort_canonical_rows` used by the TUI and the DB
/// loader: column `0` name, `1` DM count, `2` broadcast count, `3` last seen,
/// `4` first seen (any other value falls back to name). Every column ties on
/// the contact's stable key (`contact_id`), so the result is total and
/// deterministic; a descending sort reverses the WHOLE ordering (tie-break
/// included — Rust `ord.reverse()`). Seen columns use the canonical
/// [`crate::fmt::parse_last_seen_ms`] (0 on unparseable input).
#[must_use]
#[flutter_rust_bridge::frb(sync)]
pub fn sort_contacts(
    contacts: Vec<ContactSortInput>,
    sort_column: u32,
    ascending: bool,
) -> Vec<ContactSortInput> {
    let mut keyed: Vec<(ContactSortInput, String)> = contacts
        .into_iter()
        .map(|c| {
            let name_key = c.display_name.to_lowercase();
            (c, name_key)
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
    keyed.into_iter().map(|(c, _)| c).collect()
}

/// A device bound into a user's device-set, as exposed to Flutter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileDeviceInfo {
    /// Unique identifier of the signed device→user claim
    pub claim_id: String,
    /// User this device is bound into
    pub user_id: String,
    /// Peer ID of the bound device
    pub peer_id: String,
    /// Device pubkey (install key) being bound
    pub device_pubkey: Vec<u8>,
    /// Timestamp when the claim was bound, formatted for display
    pub bound_at: String,
}

/// A user-arranged contact pair, as exposed to Flutter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileContactInfo {
    /// User ID of the owner of this contact row
    pub owner_user_id: String,
    /// User ID of the contact (foreign user)
    pub contact_user_id: String,
    /// Nickname override this owner has assigned to the contact
    pub nickname_override: Option<String>,
    /// Timestamp when the contact was first added, formatted for display
    pub added_at: String,
}

fn device_info_from(
    binding: crate::generated::models_queryable::DeviceBinding,
) -> MobileDeviceInfo {
    MobileDeviceInfo {
        claim_id: binding.claim_id,
        user_id: binding.user_id,
        peer_id: binding.peer_id,
        device_pubkey: binding.device_pubkey,
        bound_at: chrono::Utc
            .from_utc_datetime(&binding.bound_at)
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    }
}

fn contact_info_from(contact: crate::generated::models_queryable::Contact) -> MobileContactInfo {
    MobileContactInfo {
        owner_user_id: contact.owner_user_id,
        contact_user_id: contact.contact_user_id,
        nickname_override: contact.nickname_override,
        added_at: chrono::Utc
            .from_utc_datetime(&contact.added_at)
            .with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M:%S")
            .to_string(),
    }
}

/// Resolve this install's user ID from its stored identity and binding.
///
/// A user is "this install's user" when a live [`crate::identity::DeviceBinding`]
/// row binds our local peer ID into that user's device-set. Returns `None`
/// before the install has been bound to any user.
///
/// # Errors
/// Returns an error if the local peer ID cannot be resolved or the DB lookup
/// fails.
#[flutter_rust_bridge::frb(ignore)]
pub fn get_my_user_id() -> Result<Option<String>, String> {
    let local_peer = crate::get_local_peer_id().map_err(|e| e.to_string())?;
    crate::identity::find_user_id_for_peer(&local_peer.to_string()).map_err(|e| e.to_string())
}

/// Provision this install as a user: mint an account keypair, persist the
/// account with the given display seams, store the account key, and sign this
/// install's own first device-binding claim. Idempotent — returns the existing
/// user ID if the install is already bound.
///
/// # Errors
/// Returns an error if provisioning/persistence fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn provision_my_user(
    self_username: Option<String>,
    self_nickname: Option<String>,
) -> Result<String, String> {
    crate::identity::provision_my_user(self_username.as_deref(), self_nickname.as_deref())
        .map_err(|e| e.to_string())
}

/// Verify a raw device-binding claim against the user's stored account key.
///
/// The claim is the raw signature over `user_id ‖ device_pubkey ‖ claim_id`;
/// it succeeds only when it checks out against the account pubkey stored on
/// that user's row.
///
/// # Errors
/// Returns an error if the DB lookup fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn verify_device_binding_claim(
    user_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
) -> Result<bool, String> {
    crate::identity::verify_device_binding_claim(&user_id, &device_pubkey, &claim_id, &claim)
        .map_err(|e| e.to_string())
}

/// Verify a claim against the user's account key, then bind the device.
///
/// # Errors
/// Returns an error if the claim fails verification or the insert fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn add_device(
    user_id: String,
    peer_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
) -> Result<MobileDeviceInfo, String> {
    crate::identity::save_device_binding(&user_id, &peer_id, &device_pubkey, &claim_id, &claim)
        .map(device_info_from)
        .map_err(|e| e.to_string())
}

/// List the live devices bound to a user, most recent first.
///
/// # Errors
/// Returns an error if the DB query fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn get_user_devices(user_id: String) -> Result<Vec<MobileDeviceInfo>, String> {
    crate::identity::list_user_devices(&user_id)
        .map(|devices| devices.into_iter().map(device_info_from).collect())
        .map_err(|e| e.to_string())
}

/// List this install's user's contacts (owner = this install's user).
///
/// Returns an empty list before the install is bound to a user.
///
/// # Errors
/// Returns an error if the DB query fails.
#[flutter_rust_bridge::frb(ignore)]
pub fn get_contacts() -> Result<Vec<MobileContactInfo>, String> {
    let Some(owner_user_id) = get_my_user_id()? else {
        return Ok(Vec::new());
    };
    crate::identity::list_contacts(&owner_user_id)
        .map(|contacts| contacts.into_iter().map(contact_info_from).collect())
        .map_err(|e| e.to_string())
}

/// Add a contact for this install's user (owner = this install's user).
///
/// # Errors
/// Returns an error if the install has no bound user or the insert fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn add_contact(
    contact_user_id: String,
    nickname_override: Option<String>,
) -> Result<(), String> {
    let owner_user_id =
        get_my_user_id()?.ok_or_else(|| "this install is not bound to a user yet".to_string())?;
    crate::identity::add_contact(
        &owner_user_id,
        &contact_user_id,
        nickname_override.as_deref(),
    )
    .map_err(|e| e.to_string())
}

/// Remove a contact for this install's user (owner = this install's user).
///
/// # Errors
/// Returns an error if the install has no bound user or the delete fails.
#[allow(clippy::needless_pass_by_value)]
#[flutter_rust_bridge::frb(ignore)]
pub fn remove_contact(contact_user_id: String) -> Result<(), String> {
    let owner_user_id =
        get_my_user_id()?.ok_or_else(|| "this install is not bound to a user yet".to_string())?;
    crate::identity::remove_contact(&owner_user_id, &contact_user_id).map_err(|e| e.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobileInitStatus {
    pub database_url: String,
    pub local_peer_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MobilePeerStatus {
    pub database_url: String,
    pub local_peer_id: String,
    pub self_nickname: Option<String>,
}

#[flutter_rust_bridge::frb(ignore)]
pub fn init_mobile_database(db_path: String) -> Result<MobileInitStatus, String> {
    crate::db::set_db_url(&db_path);
    crate::init_database().map_err(|e| e.to_string())?;
    let local_peer_id = crate::get_local_peer_id().map_err(|e| e.to_string())?;

    Ok(MobileInitStatus {
        database_url: crate::get_database_url(),
        local_peer_id: local_peer_id.to_string(),
    })
}

#[flutter_rust_bridge::frb(ignore)]
pub fn get_mobile_peer_status() -> Result<MobilePeerStatus, String> {
    let local_peer_id = crate::get_local_peer_id().map_err(|e| e.to_string())?;
    let self_nickname = crate::get_self_nickname().map_err(|e| e.to_string())?;

    Ok(MobilePeerStatus {
        database_url: crate::get_database_url(),
        local_peer_id: local_peer_id.to_string(),
        self_nickname,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial(db)]
    fn init_mobile_database_uses_supplied_path() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("mobile.sqlite");
        crate::reset_db_url();

        let status =
            init_mobile_database(db_path.to_string_lossy().into_owned()).expect("mobile init");

        assert_eq!(status.database_url, db_path.to_string_lossy());
        assert!(!status.local_peer_id.is_empty());
        assert!(db_path.exists());
    }

    #[test]
    #[serial(db)]
    fn init_mobile_database_creates_missing_parent_dirs() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        // Simulate Android: nested `databases` folder does not exist yet.
        let db_path = dir.path().join("databases").join("p2p.db");
        assert!(!db_path.parent().unwrap().exists());
        crate::reset_db_url();

        init_mobile_database(db_path.to_string_lossy().into_owned()).expect("mobile init");

        assert!(db_path.exists());
    }

    #[test]
    #[serial(db)]
    fn get_mobile_peer_status_returns_current_state() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("status_test.sqlite");
        let db_url = db_path.to_string_lossy().into_owned();
        crate::reset_db_url();
        crate::db::set_db_url(&db_url);

        init_mobile_database(db_url).expect("mobile init");

        let status = get_mobile_peer_status().expect("peer status");
        assert_eq!(status.database_url, db_path.to_string_lossy());
        assert!(!status.local_peer_id.is_empty());
    }

    #[test]
    fn test_mobile_init_status_clone_debug() {
        let s = MobileInitStatus {
            database_url: "db.sqlite".into(),
            local_peer_id: "peer1".into(),
        };
        let cloned = s.clone();
        assert_eq!(cloned.database_url, "db.sqlite");
        assert_eq!(cloned.local_peer_id, "peer1");
        let _ = format!("{s:?}");
    }

    #[test]
    fn test_mobile_peer_status_clone_debug() {
        let s = MobilePeerStatus {
            database_url: "db.sqlite".into(),
            local_peer_id: "peer1".into(),
            self_nickname: Some("Alice".into()),
        };
        let cloned = s.clone();
        assert_eq!(cloned.self_nickname.as_deref(), Some("Alice"));
        let _ = format!("{s:?}");
    }

    #[test]
    fn test_format_time_hhmm() {
        // ISO format "YYYY-MM-DD HH:MM:SS"
        let result = format_time_hhmm("2024-06-15 10:30:00");
        assert_eq!(result, "10:30");

        // Short string fallback
        let result = format_time_hhmm("short");
        assert_eq!(result, "short");

        // Edge case: exactly 16 chars
        let result = format_time_hhmm("2024-06-15 09:05:00");
        assert_eq!(result, "09:05");
    }

    #[test]
    fn test_is_at_bottom() {
        // scroll_offset >= total - visible  => at bottom
        assert!(is_at_bottom(95, 100, 10)); // 95 >= 90 => true
        assert!(!is_at_bottom(89, 100, 10)); // 89 >= 90 => false
        assert!(is_at_bottom(100, 100, 10)); // 100 >= 90 => true
        assert!(!is_at_bottom(0, 100, 10)); // 0 >= 90 => false

        // Edge cases
        assert!(is_at_bottom(5, 5, 10)); // 5 >= 0 => true (total <= visible)
        assert!(is_at_bottom(0, 0, 0)); // 0 >= 0 => true (empty)
    }

    fn sample_peer(
        id: &str,
        name: &str,
        last_seen: &str,
        first_seen: &str,
        dm: u32,
        br: u32,
    ) -> PeerSortInput {
        PeerSortInput {
            peer_id: id.to_string(),
            display_name: name.to_string(),
            last_seen: last_seen.to_string(),
            first_seen: first_seen.to_string(),
            dm_count: dm,
            broadcast_count: br,
        }
    }

    fn sample_contact(
        id: &str,
        name: &str,
        last_seen: &str,
        first_seen: &str,
        dm: i64,
        br: i64,
    ) -> ContactSortInput {
        ContactSortInput {
            contact_id: id.to_string(),
            display_name: name.to_string(),
            last_seen: last_seen.to_string(),
            first_seen: first_seen.to_string(),
            dm_count: dm,
            broadcast_sent_to_peer: br,
        }
    }

    fn contact_ids(contacts: &[ContactSortInput]) -> Vec<&str> {
        contacts.iter().map(|c| c.contact_id.as_str()).collect()
    }

    fn ids(peers: &[PeerSortInput]) -> Vec<&str> {
        peers.iter().map(|p| p.peer_id.as_str()).collect()
    }

    #[test]
    fn parse_last_seen_ms_delegates_to_canonical() {
        let ms = parse_last_seen_ms("2024-01-01 00:00:01");
        let direct = parse_last_seen_ms("2024-01-01T00:00:01");
        assert_eq!(ms, direct);
        assert!(ms > 0);
        assert_eq!(parse_last_seen_ms("not-a-date"), 0);
    }

    #[test]
    fn sort_peers_last_seen_desc_with_id_tiebreak() {
        let peers = vec![
            sample_peer(
                "aaa",
                "alpha",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                1,
                2,
            ),
            sample_peer(
                "bbb",
                "beta",
                "2024-01-01 10:00:00",
                "2024-01-01 08:00:00",
                1,
                1,
            ),
            sample_peer(
                "ccc",
                "gamma",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                3,
                0,
            ),
        ];
        let sorted = sort_peers(peers, 3, false);
        assert_eq!(ids(&sorted), vec!["bbb", "ccc", "aaa"]);
    }

    #[test]
    fn sort_peers_first_seen_desc_with_id_tiebreak() {
        let peers = vec![
            sample_peer(
                "aaa",
                "alpha",
                "2024-01-01 08:00:00",
                "2024-01-01 09:00:00",
                1,
                2,
            ),
            sample_peer(
                "bbb",
                "beta",
                "2024-01-01 08:00:00",
                "2024-01-01 10:00:00",
                1,
                1,
            ),
            sample_peer(
                "ccc",
                "gamma",
                "2024-01-01 08:00:00",
                "2024-01-01 09:00:00",
                3,
                0,
            ),
        ];
        let sorted = sort_peers(peers, 4, false);
        assert_eq!(ids(&sorted), vec!["bbb", "ccc", "aaa"]);
    }

    #[test]
    fn sort_peers_name_asc_is_case_insensitive() {
        let peers = vec![
            sample_peer(
                "bbb",
                "Zulu",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
            sample_peer(
                "aaa",
                "alpha",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
            sample_peer(
                "ccc",
                "Bravo",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
        ];
        let sorted = sort_peers(peers, 0, true);
        assert_eq!(ids(&sorted), vec!["aaa", "ccc", "bbb"]);
    }

    #[test]
    fn sort_contacts_last_seen_desc_with_contact_id_tiebreak() {
        let contacts = vec![
            sample_contact(
                "aaa",
                "alpha",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                1,
                2,
            ),
            sample_contact(
                "bbb",
                "beta",
                "2024-01-01 10:00:00",
                "2024-01-01 08:00:00",
                1,
                1,
            ),
            sample_contact(
                "ccc",
                "gamma",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                3,
                0,
            ),
        ];
        let sorted = sort_contacts(contacts, 3, false);
        assert_eq!(contact_ids(&sorted), vec!["bbb", "ccc", "aaa"]);
    }

    #[test]
    fn sort_contacts_name_asc_is_case_insensitive() {
        let contacts = vec![
            sample_contact(
                "bbb",
                "Zulu",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
            sample_contact(
                "aaa",
                "alpha",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
            sample_contact(
                "ccc",
                "Bravo",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                0,
                0,
            ),
        ];
        let sorted = sort_contacts(contacts, 0, true);
        assert_eq!(contact_ids(&sorted), vec!["aaa", "ccc", "bbb"]);
    }

    #[test]
    fn sort_contacts_count_columns_unchanged_by_unrelated_columns() {
        let contacts = vec![
            sample_contact(
                "aaa",
                "A",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                3,
                5,
            ),
            sample_contact(
                "bbb",
                "B",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                1,
                9,
            ),
            sample_contact(
                "ccc",
                "C",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                2,
                1,
            ),
        ];
        let by_dm = sort_contacts(contacts.clone(), 1, true);
        assert_eq!(contact_ids(&by_dm), vec!["bbb", "ccc", "aaa"]);
        let by_br = sort_contacts(contacts.clone(), 2, false);
        assert_eq!(contact_ids(&by_br), vec!["bbb", "aaa", "ccc"]);
        let unknown = sort_contacts(contacts, 99, true);
        assert_eq!(contact_ids(&unknown), vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    fn sort_peers_count_columns_not_affected_by_columns() {
        let peers = vec![
            sample_peer(
                "aaa",
                "A",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                3,
                5,
            ),
            sample_peer(
                "bbb",
                "B",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                1,
                9,
            ),
            sample_peer(
                "ccc",
                "C",
                "2024-01-01 09:00:00",
                "2024-01-01 08:00:00",
                2,
                1,
            ),
        ];
        let by_dm = sort_peers(peers.clone(), 1, true);
        assert_eq!(ids(&by_dm), vec!["bbb", "ccc", "aaa"]);
        let by_br = sort_peers(peers.clone(), 2, false);
        assert_eq!(ids(&by_br), vec!["bbb", "aaa", "ccc"]);
        let unknown = sort_peers(peers, 99, true);
        assert_eq!(ids(&unknown), vec!["aaa", "bbb", "ccc"]);
    }

    #[test]
    #[serial(db)]
    fn get_peers_with_stats_returns_peers_and_counts() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("peers_stats.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        crate::init_database().expect("init db");

        let peer_id = "peerA";
        crate::save_peer(peer_id, &[]).expect("save peer");
        crate::set_peer_received_nickname(peer_id, "Announced").expect("set received nick");
        crate::set_peer_local_nickname(peer_id, "Local").expect("set local nick");
        crate::save_message("dm", Some(peer_id), "dm-topic", true, Some("me")).expect("save dm");
        crate::peers::record_broadcast_recipients("b1", &[peer_id.to_string()])
            .expect("record recipient");

        let rows = get_peers_with_stats().expect("peers with stats");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, peer_id);
        assert_eq!(rows[0].broadcast_sent_to_peer, 1);
        assert_eq!(rows[0].dm_count, 1);
        assert_eq!(rows[0].nickname.as_deref(), Some("Announced"));
        assert_eq!(rows[0].local_nickname.as_deref(), Some("Local"));
        assert!(!rows[0].display_name.is_empty());
    }

    #[test]
    #[serial(db)]
    fn get_contacts_with_stats_collapses_user_devices() {
        let _guard = crate::db::shared_db_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("contacts_stats.sqlite");
        crate::reset_db_url();
        crate::db::set_db_url(&db_path.to_string_lossy());
        crate::init_database().expect("init db");

        let account_key = libp2p_identity::Keypair::generate_ed25519();
        let user_id = "alice";
        crate::identity::save_user(
            user_id,
            &account_key.public().encode_protobuf(),
            Some(user_id),
            Some(user_id),
        )
        .expect("save user");
        for (peer_id, claim_id) in [("dev-A", "claim-A"), ("dev-B", "claim-B")] {
            let device_pubkey = account_key.public().encode_protobuf();
            let mut message = Vec::new();
            message.extend_from_slice(user_id.as_bytes());
            message.extend_from_slice(&device_pubkey);
            message.extend_from_slice(claim_id.as_bytes());
            let claim = account_key.sign(&message).expect("sign claim");
            crate::peers::save_peer(peer_id, &[]).expect("save peer");
            crate::identity::save_device_binding(
                user_id,
                peer_id,
                &device_pubkey,
                claim_id,
                &claim,
            )
            .expect("save binding");
        }
        crate::save_message("dm-1", None, "dm-topic", true, Some("dev-A")).expect("save dm");
        crate::peers::record_broadcast_recipients("b1", &["dev-B".to_string()])
            .expect("record recipient");

        let rows = get_contacts_with_stats().expect("contacts with stats");
        assert_eq!(rows.len(), 1, "both devices collapse into one user row");
        let row = &rows[0];
        assert!(row.is_user);
        assert_eq!(row.contact_id, user_id);
        assert_eq!(row.device_count, 2);
        assert_eq!(row.dm_count, 1);
        assert_eq!(row.broadcast_sent_to_peer, 1);
        assert_eq!(row.device_rows.len(), 2);
    }
}
