use super::db_url_cache;

/// Test utility: Clears the cached database URL for test isolation.
pub fn reset_db_url_cache() {
    if let Ok(mut cached) = db_url_cache().lock() {
        *cached = None;
    }
}
