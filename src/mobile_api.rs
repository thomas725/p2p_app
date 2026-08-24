//! Thin API facade for mobile hosts.
//!
//! Keep this module smaller and more stable than the internal Rust API. Flutter,
//! Android services, and future iOS bindings should depend on this facade rather
//! than reaching into TUI/desktop-oriented modules directly.

/// Format an ISO timestamp string ("YYYY-MM-DD HH:MM:SS") to "HH:MM".
///
/// Used by Dart/Flutter UI to display message timestamps.
#[must_use]
#[flutter_rust_bridge::frb(sync)]
pub fn format_time_hhmm(dt: &str) -> String {
    if dt.len() >= 16 {
        dt[11..16].to_string()
    } else {
        dt.to_string()
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
    let end = (start + visible_count).min(total_messages);
    (start, end)
}

/// Validate a nickname: alphanumeric and dash only, max 20 chars.
///
/// Delegates to the canonical [`crate::nickname::validate_nickname`].
#[must_use]
pub fn validate_nickname(nick: &str) -> bool {
    crate::nickname::validate_nickname(nick)
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
    crate::db::set_cached_db_url(&db_path);
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
        crate::reset_db_url_cache();

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
        crate::reset_db_url_cache();

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
        crate::reset_db_url_cache();
        crate::db::set_cached_db_url(&db_url);

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
        let _ = format!("{:?}", s);
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
        let _ = format!("{:?}", s);
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
}
