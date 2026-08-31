//! Tests for db.rs module - database URL and identity functions

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

use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
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

#[serial]
#[test]
fn test_get_database_url_set_url() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    p2p_app::db::set_db_url("/tmp/test.db");
    let url = p2p_app::db::get_database_url();
    assert_eq!(url, "/tmp/test.db");
    p2p_app::db::reset_db_url();
}

#[serial]
#[test]
fn test_release_db_lock() {
    p2p_app::db::release_db_lock();
}

#[serial]
#[test]
fn test_init_database_succeeds() {
    with_test_db(|| {
        p2p_app::db::init_database().expect("init_database should succeed");
    });
}

#[serial]
#[test]
fn test_get_libp2p_identity_creates_keypair() {
    with_test_db(|| {
        let peer_id = p2p_app::db::get_local_peer_id().expect("should derive peer ID from keypair");
        assert!(!peer_id.to_string().is_empty());
    });
}

#[serial]
#[test]
fn test_get_libp2p_identity_is_stable() {
    with_test_db(|| {
        let id1 = p2p_app::db::get_local_peer_id().unwrap();
        let id2 = p2p_app::db::get_local_peer_id().unwrap();
        assert_eq!(id1, id2, "peer ID should be stable across calls");
    });
}

#[serial]
#[test]
fn test_get_local_peer_id() {
    with_test_db(|| {
        let peer_id = p2p_app::db::get_local_peer_id().expect("should return local peer ID");
        let s = peer_id.to_string();
        assert!(!s.is_empty());
        assert!(s.starts_with("12D3KooW"), "unexpected peer ID format: {s}");
    });
}

#[serial]
#[test]
fn test_sqlite_connect_runs_migrations() {
    with_test_db(|| {
        p2p_app::save_message("migration-check", None, "topic", false, None)
            .expect("messages table should exist after migration");
        p2p_app::save_peer("peer-check", &[]).expect("peers table should exist after migration");
    });
}

#[serial]
#[test]
fn test_get_database_url_from_set_url() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    p2p_app::db::set_db_url("/tmp/explicit.db");
    let url = p2p_app::db::get_database_url();
    assert_eq!(url, "/tmp/explicit.db");
    p2p_app::db::reset_db_url();
}

#[serial]
#[test]
fn test_reset_db_url_clears_set_url() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    p2p_app::db::set_db_url("/tmp/test.db");
    let url1 = p2p_app::get_database_url();
    assert_eq!(url1, "/tmp/test.db");

    p2p_app::db::set_db_url("/tmp/other.db");
    let url2 = p2p_app::get_database_url();
    assert!(!url2.is_empty());

    p2p_app::db::reset_db_url();
    let url3 = p2p_app::db::get_database_url();
    assert_ne!(url3, "/tmp/other.db");
    assert!(
        std::path::Path::new(&url3)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("db"))
    );
}

// ── Additional database edge cases ─────────────────────────────────────────────

#[serial]
#[test]
fn test_get_local_peer_id_deterministic() {
    with_test_db(|| {
        let id1 = p2p_app::get_local_peer_id().unwrap();
        let id2 = p2p_app::get_local_peer_id().unwrap();
        assert_eq!(id1.to_string(), id2.to_string());
    });
}

#[serial]
#[test]
fn test_get_libp2p_identity_format() {
    with_test_db(|| {
        let keypair = p2p_app::get_libp2p_identity().unwrap();
        let peer_id = libp2p::PeerId::from_public_key(&keypair.public());
        assert!(!peer_id.to_string().is_empty());
    });
}

#[serial]
#[test]
fn test_get_local_peer_id_matches_keypair() {
    with_test_db(|| {
        let keypair = p2p_app::get_libp2p_identity().unwrap();
        let stored_id = p2p_app::get_local_peer_id().unwrap();
        let computed_id = libp2p::PeerId::from_public_key(&keypair.public());
        assert_eq!(computed_id, stored_id);
    });
}

#[serial]
#[test]
fn test_release_db_lock_idempotent() {
    with_test_db(|| {
        p2p_app::db::release_db_lock();
        p2p_app::db::release_db_lock();
    });
}

#[serial]
#[test]
fn test_reset_db_url_then_get_url_without_set_url() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    p2p_app::db::reset_db_url();
    let url = p2p_app::db::get_database_url();
    assert!(!url.is_empty());
    p2p_app::db::release_db_lock();
}

#[test]
fn test_sqlite_connect_fails_with_bad_path() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // Use a regular file as parent directory: create_dir_all cannot replace
    // it with a directory, so the connect must fail even when running as root.
    let dir = tempfile::tempdir().expect("tempdir");
    let file_parent = dir.path().join("not_a_dir");
    std::fs::write(&file_parent, b"").expect("create file");
    let db_path = file_parent.join("sub").join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    let result = p2p_app::db::sqlite_connect();
    assert!(result.is_err());
    p2p_app::db::reset_db_url();
}

#[serial]
#[test]
fn test_db_path_determined_once_per_init() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().expect("tempdir");
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    p2p_app::db::reset_db_url();
    p2p_app::logging::clear_tui_logs();
    // No set URL -> path determination actually runs.
    let _ = p2p_app::db::init_database();
    let _ = std::env::set_current_dir(&old_cwd);
    let logs = p2p_app::logging::get_tui_logs();
    let cwd_count = logs.iter().filter(|l| l.contains("[DB] cwd=")).count();
    let checking_count = logs.iter().filter(|l| l.contains("[DB] checking")).count();
    println!("DEBUG cwd_count={cwd_count} checking_count={checking_count}");
    assert_eq!(
        cwd_count, 1,
        "path determination should log [DB] cwd exactly once per init_database call, got {cwd_count}"
    );
    assert!(
        checking_count <= 1,
        "path determination should check dbs at most once per init_database call, got {checking_count}"
    );
}

#[serial]
#[test]
fn test_concurrent_init_uses_isolated_databases() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::tempdir().expect("tempdir");
    // Pre-existing .db so the "checking" loop actually logs.
    std::fs::write(dir.path().join("sqlite.db"), b"").expect("seed db");
    std::fs::write(dir.path().join("sqlite.db.lock"), b"1").expect("seed lock");
    let old_cwd = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    p2p_app::db::reset_db_url();
    p2p_app::logging::clear_tui_logs();

    // Each thread has its own thread-local URL, so concurrent init_database
    // calls independently select a database. The PID-based lock file ensures
    // threads in the same process never pick the same file, so no migration race.
    let mut handles = Vec::new();
    for _ in 0..4 {
        handles.push(std::thread::spawn(|| p2p_app::db::init_database().is_ok()));
    }
    let mut all_ok = true;
    for h in handles {
        all_ok &= h.join().unwrap_or(false);
    }
    let _ = std::env::set_current_dir(&old_cwd);
    assert!(all_ok, "all concurrent init_database calls should succeed");
    let logs = p2p_app::logging::get_tui_logs();
    let cwd_count = logs.iter().filter(|l| l.contains("[DB] cwd=")).count();
    let checking_count = logs.iter().filter(|l| l.contains("[DB] checking")).count();
    println!("DEBUG concurrent cwd_count={cwd_count} checking_count={checking_count}");
    assert_eq!(
        cwd_count, 4,
        "each concurrent thread should determine its own path, got {cwd_count}"
    );
    assert!(
        checking_count >= 1,
        "path determination should run for concurrent threads, got {checking_count}"
    );
}
