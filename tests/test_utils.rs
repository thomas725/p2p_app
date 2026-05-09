//! Shared test utilities for database-backed tests.
//!
//! All tests that touch the database must use [`TestDb`] to ensure
//! serialisation (via a process-wide mutex) and cleanup between tests.

use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

/// Acquire this lock before touching `DATABASE_URL` or opening a SQLite DB
/// in tests. Because `DATABASE_URL` is a process-wide env var, parallel tests
/// will corrupt each other without serialisation.
pub fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// RAII guard that:
/// 1. Acquires the global `test_db_lock` so DB tests run serially.
/// 2. Points `DATABASE_URL` at a fresh temp file.
/// 3. Runs migrations so the schema is ready.
/// 4. On drop: releases the DB lock and removes `DATABASE_URL` from env.
pub struct TestDb {
    pub _dir: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl TestDb {
    #[allow(dead_code)]
    /// Path to the SQLite file for this test.
    pub fn path(&self) -> String {
        self._dir
            .path()
            .join("test.db")
            .to_str()
            .unwrap()
            .to_string()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        p2p_app::db::release_db_lock();
        p2p_app::db::reset_db_url_cache();
        unsafe { std::env::remove_var("DATABASE_URL") };
    }
}

/// Create and initialise an isolated test database.
pub fn setup_test_db() -> TestDb {
    let guard = test_db_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().unwrap()) };
    p2p_app::db::init_database().unwrap();
    TestDb {
        _dir: dir,
        _guard: guard,
    }
}
