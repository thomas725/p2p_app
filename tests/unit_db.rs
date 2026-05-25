use super::*;
use serial_test::serial;
use std::fs;
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
