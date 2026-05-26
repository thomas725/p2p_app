use super::*;
use serial_test::serial;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestDb {
    _dir: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for TestDb {
    fn drop(&mut self) {
        crate::db::release_db_lock();
        crate::db::reset_db_url_cache();
        unsafe { std::env::remove_var("DATABASE_URL") };
    }
}

fn setup_test_db() -> TestDb {
    let guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().expect("db path")) };
    crate::db::init_database().expect("init db");
    TestDb {
        _dir: dir,
        _guard: guard,
    }
}

#[test]
fn display_outputs_expected_labels() {
    assert_eq!(NetworkSize::Small.to_string(), "Small");
    assert_eq!(NetworkSize::Medium.to_string(), "Medium");
    assert_eq!(NetworkSize::Large.to_string(), "Large");
}

#[test]
#[serial(db)]
fn get_network_size_uses_average_peer_count() {
    let _db = setup_test_db();
    crate::peers::save_peer_session(1).expect("save");
    crate::peers::save_peer_session(2).expect("save");
    crate::peers::save_peer_session(3).expect("save");
    let size = get_network_size().expect("network size");
    assert_eq!(size, NetworkSize::Small);
}
