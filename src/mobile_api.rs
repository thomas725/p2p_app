//! Thin API facade for mobile hosts.
//!
//! Keep this module smaller and more stable than the internal Rust API. Flutter,
//! Android services, and future iOS bindings should depend on this facade rather
//! than reaching into TUI/desktop-oriented modules directly.

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

/// Calculate first visible message index accounting for scroll
#[must_use]
pub fn calculate_visible_range(
    total_messages: usize,
    scroll_offset: usize,
    visible_count: usize,
) -> (usize, usize) {
    let start = scroll_offset.min(total_messages.saturating_sub(1));
    let end = start.saturating_add(visible_count).min(total_messages);
    (start, end)
}

/// Validate a nickname: alphanumeric and dash only, max 20 chars.
///
/// Delegates to the canonical [`crate::nickname::validate_nickname`].
#[must_use]
pub fn validate_nickname(nick: &str) -> bool {
    crate::nickname::validate_nickname(nick)
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
    pub dm_count: u32,
    pub broadcast_count: u32,
}

/// A known peer joined with its message counts, fetched in a single call so
/// Flutter can populate both the peer list and the peer table's count columns
/// without an N+1 round trip over `get_peer_stats`.
#[derive(Debug, Clone)]
pub struct PeerWithStats {
    pub peer_id: String,
    pub display_name: String,
    pub first_seen: String,
    pub last_seen: String,
    pub nickname: Option<String>,
    pub local_nickname: Option<String>,
    pub dm_count: i64,
    pub broadcast_received: i64,
    pub broadcast_sent: i64,
}

/// Load all known peers and their message statistics in a single round trip.
///
/// Combines the peer list (`get_known_peers`) with per-peer stats
/// (`get_peer_stats`), mirroring what Flutter's `_refreshPeers` does with
/// N+1 calls, so the UI needs only one FFI call.
///
/// # Errors
/// Returns an error if the peers cannot be loaded.
pub fn get_peers_with_stats() -> Result<Vec<PeerWithStats>, String> {
    let peers = crate::mobile_node::get_known_peers()?;
    let mut rows = Vec::with_capacity(peers.len());
    for p in peers {
        let stats = crate::messages::get_peer_stats(&p.peer_id).map_err(|e| e.to_string())?;
        rows.push(PeerWithStats {
            peer_id: p.peer_id,
            display_name: p.display_name,
            first_seen: p.first_seen,
            last_seen: p.last_seen,
            nickname: p.nickname,
            local_nickname: p.local_nickname,
            dm_count: stats.dm_count,
            broadcast_received: stats.broadcast_received,
            broadcast_sent: stats.broadcast_sent,
        });
    }
    Ok(rows)
}

/// Sort known peers by the given column and order, mirroring the TUI peers
/// table: column `0` name, `1` DM count, `2` broadcast count, `3` last seen
/// (any other value falls back to name). Ordering always ties on `peer_id`,
/// so the result is total and stable for a given dataset.
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
            _ => an.cmp(bn).then_with(|| a.peer_id.cmp(&b.peer_id)),
        };
        if ascending { ord } else { ord.reverse() }
    });
    keyed.into_iter().map(|(p, _)| p).collect()
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

    #[test]
    fn test_calculate_visible_range() {
        // Basic case: 10 messages, offset 3, visible 5 => items 3-7 (4 items shown)
        let (start, end) = calculate_visible_range(10, 3, 5);
        assert_eq!(start, 3);
        assert_eq!(end, 8); // exclusive end, so items 3,4,5,6,7

        // Offset beyond total => clamp to total-1
        let (start, end) = calculate_visible_range(10, 15, 5);
        assert_eq!(start, 9);
        assert_eq!(end, 10);

        // Offset 0, visible all
        let (start, end) = calculate_visible_range(10, 0, 100);
        assert_eq!(start, 0);
        assert_eq!(end, 10);

        // Empty case
        let (start, end) = calculate_visible_range(0, 0, 10);
        assert_eq!(start, 0);
        assert_eq!(end, 0);
    }

    #[test]
    fn test_validate_nickname() {
        // Valid nicknames
        assert!(validate_nickname("valid-nick"));
        assert!(validate_nickname("abc123"));
        assert!(validate_nickname("a"));

        // Invalid nicknames
        assert!(!validate_nickname(""));
        assert!(!validate_nickname(
            "this-nickname-is-way-too-long-exceeds-twenty-chars"
        ));
        assert!(!validate_nickname("nick with spaces"));
        assert!(!validate_nickname("nick@special"));
        assert!(!validate_nickname("nick%dollar"));
    }

    fn sample_peer(id: &str, name: &str, last_seen: &str, dm: u32, br: u32) -> PeerSortInput {
        PeerSortInput {
            peer_id: id.to_string(),
            display_name: name.to_string(),
            last_seen: last_seen.to_string(),
            dm_count: dm,
            broadcast_count: br,
        }
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
            sample_peer("aaa", "alpha", "2024-01-01 09:00:00", 1, 2),
            sample_peer("bbb", "beta", "2024-01-01 10:00:00", 1, 1),
            sample_peer("ccc", "gamma", "2024-01-01 09:00:00", 3, 0),
        ];
        let sorted = sort_peers(peers, 3, false);
        assert_eq!(ids(&sorted), vec!["bbb", "ccc", "aaa"]);
    }

    #[test]
    fn sort_peers_name_asc_is_case_insensitive() {
        let peers = vec![
            sample_peer("bbb", "Zulu", "2024-01-01 09:00:00", 0, 0),
            sample_peer("aaa", "alpha", "2024-01-01 09:00:00", 0, 0),
            sample_peer("ccc", "Bravo", "2024-01-01 09:00:00", 0, 0),
        ];
        let sorted = sort_peers(peers, 0, true);
        assert_eq!(ids(&sorted), vec!["aaa", "ccc", "bbb"]);
    }

    #[test]
    fn sort_peers_count_columns_not_affected_by_columns() {
        let peers = vec![
            sample_peer("aaa", "A", "2024-01-01 09:00:00", 3, 5),
            sample_peer("bbb", "B", "2024-01-01 09:00:00", 1, 9),
            sample_peer("ccc", "C", "2024-01-01 09:00:00", 2, 1),
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
        crate::save_message("hello", Some(peer_id), crate::CHAT_TOPIC, false, None)
            .expect("save broadcast");
        crate::save_message("dm", Some(peer_id), "dm-topic", true, Some("me")).expect("save dm");

        let rows = get_peers_with_stats().expect("peers with stats");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, peer_id);
        assert_eq!(rows[0].broadcast_received, 1);
        assert_eq!(rows[0].dm_count, 1);
        assert_eq!(rows[0].nickname.as_deref(), Some("Announced"));
        assert_eq!(rows[0].local_nickname.as_deref(), Some("Local"));
        assert!(!rows[0].display_name.is_empty());
    }
}
