//! Shared test utilities for database-backed tests.
//!
//! All tests that touch the database must use [`with_test_db`] to ensure
//! serialisation (via a process-wide mutex) and cleanup between tests.

use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

/// Acquire this lock before touching `DATABASE_URL` or opening a `SQLite` DB
/// in tests. Because `DATABASE_URL` is a process-wide env var, parallel tests
/// will corrupt each other without serialisation.
pub fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run a test closure with an isolated temp database.
///
/// Acquires the global lock, creates a temp dir, sets `DATABASE_URL` to a
/// fresh SQLite file inside it, runs migrations, invokes `f`, then cleans up.
pub fn with_test_db(f: impl FnOnce()) {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    temp_env::with_var("DATABASE_URL", Some(db_path.to_str().unwrap()), || {
        p2p_app::db::init_database().unwrap();
        f();
        p2p_app::db::release_db_lock();
        p2p_app::db::reset_db_url_cache();
    });
}
