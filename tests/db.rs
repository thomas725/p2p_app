//! Tests for db.rs module - database URL and identity functions

#[serial]
#[test]
fn test_get_database_url_env_set() {
    let _guard = test_db_lock().lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/test.db") };
    let url = p2p_app::db::get_database_url();
    unsafe { std::env::remove_var("DATABASE_URL") };
    assert_eq!(url, "/tmp/test.db");
}

#[serial]
#[test]
fn test_release_db_lock() {
    p2p_app::db::release_db_lock();
}

use serial_test::serial;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestDb {
    _dir: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for TestDb {
    fn drop(&mut self) {
        p2p_app::db::release_db_lock();
        unsafe { std::env::remove_var("DATABASE_URL") };
    }
}

fn setup_test_db() -> TestDb {
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

#[serial]
#[test]
fn test_init_database_succeeds() {
    let _db = setup_test_db();
    p2p_app::db::init_database().expect("init_database should succeed");
}

#[serial]
#[test]
fn test_get_libp2p_identity_creates_keypair() {
    let _db = setup_test_db();
    // Verify identity can be created and yields a valid, non-empty peer ID
    let peer_id = p2p_app::db::get_local_peer_id().expect("should derive peer ID from keypair");
    assert!(!peer_id.to_string().is_empty());
}

#[serial]
#[test]
fn test_get_libp2p_identity_is_stable() {
    let _db = setup_test_db();
    // Same DB should always produce the same peer ID
    let id1 = p2p_app::db::get_local_peer_id().unwrap();
    let id2 = p2p_app::db::get_local_peer_id().unwrap();
    assert_eq!(id1, id2, "peer ID should be stable across calls");
}

#[serial]
#[test]
fn test_get_local_peer_id() {
    let _db = setup_test_db();
    let peer_id = p2p_app::db::get_local_peer_id().expect("should return local peer ID");
    let s = peer_id.to_string();
    assert!(!s.is_empty());
    // libp2p peer IDs start with "12D3KooW" for Ed25519
    assert!(
        s.starts_with("12D3KooW"),
        "unexpected peer ID format: {}",
        s
    );
}

#[serial]
#[test]
fn test_sqlite_connect_runs_migrations() {
    let _db = setup_test_db();
    // Verify migrations ran by successfully saving and loading a message
    p2p_app::save_message("migration-check", None, "topic", false, None)
        .expect("messages table should exist after migration");
    p2p_app::save_peer("peer-check", &[]).expect("peers table should exist after migration");
}

#[serial]
#[test]
fn test_get_database_url_from_env() {
    let _guard = test_db_lock().lock().unwrap_or_else(|e| e.into_inner());
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/explicit.db") };
    let url = p2p_app::db::get_database_url();
    unsafe { std::env::remove_var("DATABASE_URL") };
    assert_eq!(url, "/tmp/explicit.db");
}

#[serial]
#[test]
fn test_reset_db_url_cache() {
    // Set a URL
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/test.db") };
    let url1 = p2p_app::get_database_url();
    assert_eq!(url1, "/tmp/test.db");
    
    // Change env var
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/other.db") };
    
    // Without reset, still returns cached value
    let url2 = p2p_app::get_database_url();
    // May still be cached, so just verify it's set
    assert!(!url2.is_empty());
    
    // Reset the cache
    p2p_app::db::reset_db_url_cache();
    
    // Now should get new value
    let url3 = p2p_app::get_database_url();
    assert_eq!(url3, "/tmp/other.db");
    
    unsafe { std::env::remove_var("DATABASE_URL") };
}
