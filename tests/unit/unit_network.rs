use super::*;
use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    crate::db::set_db_url(db_path.to_str().expect("db path"));
    crate::db::init_database().expect("init db");
    f();
    crate::db::release_db_lock();
    crate::db::reset_db_url();
}

#[test]
fn display_outputs_expected_labels() {
    assert_eq!(NetworkSize::Small.to_string(), "Small");
    assert_eq!(NetworkSize::Medium.to_string(), "Medium");
    assert_eq!(NetworkSize::Large.to_string(), "Large");
}

#[test]
#[serial(db)]
fn get_network_size_defaults_small_when_no_sessions() {
    with_test_db(|| {
        let size = get_network_size().expect("network size");
        assert_eq!(size, NetworkSize::Small);
    });
}
