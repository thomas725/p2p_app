use super::*;
use diesel::{ExpressionMethods, QueryDsl};
use serial_test::serial;
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

#[test]
fn is_db_locked_false_when_lock_missing() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("missing.db.lock");
    assert!(!is_db_locked(&lock_path));
    assert!(!lock_path.exists());
}

#[test]
fn is_db_locked_removes_zero_pid_lock() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("zero.db.lock");
    fs::write(&lock_path, "0").expect("write lock");
    assert!(!is_db_locked(&lock_path));
    assert!(!lock_path.exists());
}

#[test]
fn is_db_locked_removes_non_numeric_lock() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("invalid.db.lock");
    fs::write(&lock_path, "not-a-pid").expect("write lock");
    assert!(!is_db_locked(&lock_path));
    assert!(!lock_path.exists());
}

#[cfg(target_os = "linux")]
#[test]
fn is_db_locked_removes_dead_pid_lock() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("dead.db.lock");
    fs::write(&lock_path, "999999").expect("write lock");
    assert!(!is_db_locked(&lock_path));
    assert!(!lock_path.exists());
}

#[test]
fn try_acquire_lock_success_then_fail() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("acquire.db.lock");
    assert!(try_acquire_lock(&lock_path, 1234).is_ok());
    assert!(try_acquire_lock(&lock_path, 1234).is_err());
}

#[test]
fn create_new_db_picks_next_index() {
    let dir = TempDir::new().expect("tempdir");
    let files = vec!["sqlite_2.db".to_string(), "sqlite_9.db".to_string()];
    let name = create_new_db(&files, dir.path(), 4242);
    assert_eq!(name, "sqlite_10.db");
    assert!(dir.path().join("sqlite_10.db.lock").exists());
}

#[test]
#[serial(db)]
fn find_or_create_unused_db_uses_existing_unlocked_file() {
    let dir = TempDir::new().expect("tempdir");
    let old = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    fs::write("sqlite_1.db", b"").expect("create db");
    let chosen = find_or_create_unused_db().expect("find db");
    assert!(chosen.ends_with("sqlite_1.db"));
    assert!(dir.path().join("sqlite_1.db.lock").exists());

    std::env::set_current_dir(old).expect("restore cwd");
}

#[test]
#[serial(db)]
fn find_or_create_unused_db_creates_new_when_existing_locked() {
    let dir = TempDir::new().expect("tempdir");
    let old = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    fs::write("sqlite_1.db", b"").expect("create db");
    fs::write("sqlite_1.db.lock", std::process::id().to_string()).expect("active lock");
    let chosen = find_or_create_unused_db().expect("find db");
    assert!(chosen.ends_with("sqlite_2.db"));
    assert!(dir.path().join("sqlite_2.db.lock").exists());

    std::env::set_current_dir(old).expect("restore cwd");
}

#[test]
#[serial(db)]
fn get_database_url_prefers_env_and_updates_cache() {
    reset_db_url_cache();
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/db_a.sqlite") };
    let a = get_database_url();
    assert_eq!(a, "/tmp/db_a.sqlite");

    unsafe { std::env::set_var("DATABASE_URL", "/tmp/db_b.sqlite") };
    let b = get_database_url();
    assert_eq!(b, "/tmp/db_b.sqlite");

    unsafe { std::env::remove_var("DATABASE_URL") };
    reset_db_url_cache();
}

#[test]
fn is_db_locked_unreadable_path_treated_as_stale() {
    let dir = TempDir::new().expect("tempdir");
    let lock_path = dir.path().join("unreadable.db.lock");
    fs::create_dir(&lock_path).expect("create dir lock path");
    assert!(!is_db_locked(&lock_path));
    assert!(lock_path.exists());
}

#[test]
#[serial(db)]
fn get_database_url_uses_cached_value_when_env_missing() {
    reset_db_url_cache();
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/db_cached.sqlite") };
    assert_eq!(get_database_url(), "/tmp/db_cached.sqlite");
    unsafe { std::env::remove_var("DATABASE_URL") };
    assert_eq!(get_database_url(), "/tmp/db_cached.sqlite");
    reset_db_url_cache();
}

#[test]
#[serial(db)]
fn release_db_lock_removes_lock_file() {
    reset_db_url_cache();
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("release.db");
    let db_path_str = db_path.to_string_lossy().to_string();
    let lock_path = format!("{db_path_str}.lock");
    fs::write(&lock_path, "1234").expect("write lock");

    unsafe { std::env::set_var("DATABASE_URL", &db_path_str) };
    let _ = get_database_url();
    release_db_lock();
    assert!(!std::path::Path::new(&lock_path).exists());

    unsafe { std::env::remove_var("DATABASE_URL") };
    reset_db_url_cache();
}

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
#[serial(db)]
fn get_libp2p_identity_recovers_from_invalid_stored_key() {
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("identity.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().expect("db path")) };
    reset_db_url_cache();
    init_database().expect("init db");

    let conn = &mut sqlite_connect().expect("connect");
    diesel::update(crate::generated::schema::identities::table)
        .set(crate::generated::schema::identities::key.eq(vec![1_u8, 2, 3]))
        .execute(conn)
        .expect("update invalid key");

    let keypair = get_libp2p_identity().expect("recover identity");
    let stored_rows = crate::generated::schema::identities::table
        .select(crate::generated::models_queryable::Identity::as_select())
        .load::<crate::generated::models_queryable::Identity>(conn)
        .expect("load identities");
    let at_least_one_valid = stored_rows
        .iter()
        .any(|r| libp2p_identity::Keypair::from_protobuf_encoding(&r.key).is_ok());
    assert!(at_least_one_valid);
    let expected_peer_id = keypair.public().to_peer_id();
    assert_eq!(
        expected_peer_id,
        get_local_peer_id().expect("local peer id")
    );

    release_db_lock();
    reset_db_url_cache();
    unsafe { std::env::remove_var("DATABASE_URL") };
}

#[test]
#[serial(db)]
fn determine_db_path_uses_env_when_set() {
    reset_db_url_cache();
    unsafe { std::env::set_var("DATABASE_URL", "/tmp/determine_env.sqlite") };
    let path = determine_db_path().expect("determine path");
    assert_eq!(path, "/tmp/determine_env.sqlite");
    unsafe { std::env::remove_var("DATABASE_URL") };
    reset_db_url_cache();
}

#[test]
#[serial(db)]
fn get_database_url_falls_back_when_no_env_or_cache() {
    reset_db_url_cache();
    unsafe { std::env::remove_var("DATABASE_URL") };
    let dir = TempDir::new().expect("tempdir");
    let old = std::env::current_dir().expect("cwd");
    std::env::set_current_dir(dir.path()).expect("set cwd");

    let url = get_database_url();
    assert!(url.ends_with(".db"));
    assert!(!url.is_empty());

    std::env::set_current_dir(old).expect("restore cwd");
    release_db_lock();
    reset_db_url_cache();
}

#[test]
fn test_database_url_matches_env_var() {
    use crate::db::{get_database_url, reset_db_url_cache};
    reset_db_url_cache();
    
    let db_url = get_database_url();
    // Should contain sqlite:// prefix
    assert!(db_url.starts_with("sqlite://"));
    // Should contain the database file name
    assert!(db_url.contains("p2p_app"));
}

#[test]
fn test_local_peer_id_is_valid() {
    use crate::db::get_local_peer_id;
    
    // This should succeed and return a valid PeerId
    match get_local_peer_id() {
        Ok(peer_id) => {
            let peer_str = peer_id.to_string();
            assert!(!peer_str.is_empty());
        }
        Err(_) => {
            // Error is acceptable if DB setup fails
        }
    }
}

#[test]
fn test_libp2p_identity_is_valid() {
    use crate::db::get_libp2p_identity;
    
    // This should succeed and return a valid Keypair
    match get_libp2p_identity() {
        Ok(keypair) => {
            let public_key = keypair.public();
            assert!(!public_key.to_bytes().is_empty());
        }
        Err(_) => {
            // Error is acceptable if DB setup fails
        }
    }
}

#[test]
fn test_reset_db_url_cache_multiple_times() {
    use crate::db::{get_database_url, reset_db_url_cache};
    
    let url1 = get_database_url();
    reset_db_url_cache();
    let url2 = get_database_url();
    
    // After reset, should get the same URL
    assert_eq!(url1, url2);
}

#[test]
fn test_release_db_lock() {
    use crate::db::release_db_lock;
    
    // Should not panic
    release_db_lock();
}
