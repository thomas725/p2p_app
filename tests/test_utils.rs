//! Shared test utilities for database-backed tests.
//!
//! All tests that touch the database must use [`with_test_db`] to ensure
//! cleanup between tests. Isolation is provided by the per-thread database URL
//! (set via [`p2p_app::db::set_db_url`]), so each test gets its own database
//! without mutating a process-wide env var or global variable.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

/// Acquire this lock before opening a `SQLite` DB in tests. Tests that share a
/// deterministic on-disk layout (e.g. relying on the default path-selection)
/// serialise here; tests using [`with_test_db`] are already isolated by their
/// own thread-local database URL.
#[allow(clippy::too_long_first_doc_paragraph)]
pub fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Run a test closure with an isolated temp database.
///
/// Creates a temp dir, points the current thread's database URL at a fresh
/// `SQLite` file inside it, runs migrations, invokes `f`, then cleans up and
/// resets the thread-local URL so subsequent tests start clean.
#[allow(clippy::missing_panics_doc)]
pub fn with_test_db(f: impl FnOnce()) {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();
    f();
    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}
