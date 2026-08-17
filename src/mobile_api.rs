//! Thin API facade for mobile hosts.
//!
//! Keep this module smaller and more stable than the internal Rust API. Flutter,
//! Android services, and future iOS bindings should depend on this facade rather
//! than reaching into TUI/desktop-oriented modules directly.

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

pub fn init_mobile_database(db_path: String) -> Result<MobileInitStatus, String> {
    crate::db::set_cached_db_url(&db_path);
    crate::init_database().map_err(|e| e.to_string())?;
    let local_peer_id = crate::get_local_peer_id().map_err(|e| e.to_string())?;

    Ok(MobileInitStatus {
        database_url: crate::get_database_url(),
        local_peer_id: local_peer_id.to_string(),
    })
}

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
    #[serial]
    fn init_mobile_database_uses_supplied_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("mobile.sqlite");
        crate::reset_db_url_cache();

        let status =
            init_mobile_database(db_path.to_string_lossy().into_owned()).expect("mobile init");

        assert_eq!(status.database_url, db_path.to_string_lossy());
        assert!(!status.local_peer_id.is_empty());
        assert!(db_path.exists());
    }
}
