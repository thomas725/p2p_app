//! Tests for db.rs module - database URL and identity functions

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
    temp_env::with_var("DATABASE_URL", Some(db_path.to_str().unwrap()), || {
        p2p_app::db::init_database().unwrap();
        f();
        p2p_app::db::release_db_lock();
    });
}

#[serial]
#[test]
fn test_get_database_url_env_set() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("DATABASE_URL", Some("/tmp/test.db"), || {
        let url = p2p_app::db::get_database_url();
        assert_eq!(url, "/tmp/test.db");
    });
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
fn test_get_database_url_from_env() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("DATABASE_URL", Some("/tmp/explicit.db"), || {
        let url = p2p_app::db::get_database_url();
        assert_eq!(url, "/tmp/explicit.db");
    });
}

#[serial]
#[test]
fn test_reset_db_url_cache() {
    temp_env::with_var("DATABASE_URL", Some("/tmp/test.db"), || {
        let url1 = p2p_app::get_database_url();
        assert_eq!(url1, "/tmp/test.db");

        temp_env::with_var("DATABASE_URL", Some("/tmp/other.db"), || {
            let url2 = p2p_app::get_database_url();
            assert!(!url2.is_empty());

            p2p_app::db::reset_db_url_cache();

            let url3 = p2p_app::get_database_url();
            assert_eq!(url3, "/tmp/other.db");
        });
    });
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
fn test_reset_db_url_cache_then_get_url_without_env() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("DATABASE_URL", None::<&str>, || {
        p2p_app::db::reset_db_url_cache();
        let url = p2p_app::db::get_database_url();
        assert!(!url.is_empty());
        p2p_app::db::release_db_lock();
    });
}

#[test]
fn test_sqlite_connect_fails_with_bad_path() {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("DATABASE_URL", Some("/nonexistent/dir/test.db"), || {
        p2p_app::db::reset_db_url_cache();
        let result = p2p_app::db::sqlite_connect();
        assert!(result.is_err());
    });
}
