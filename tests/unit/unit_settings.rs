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
#[serial]
fn lan_only_defaults_to_false_and_roundtrips() {
    with_test_db(|| {
        assert!(!lan_only(), "a fresh install keeps internet discovery");
        set_lan_only(true).expect("enable");
        assert!(lan_only(), "enabled value is read back");
        set_lan_only(false).expect("disable");
        assert!(!lan_only(), "disabled value is read back");
    });
}

#[test]
#[serial]
fn set_lan_only_is_an_upsert_not_a_second_row() {
    with_test_db(|| {
        set_lan_only(true).expect("first write");
        set_lan_only(true).expect("repeated write");
        let conn = &mut crate::sqlite_connect().expect("connect");
        let count: i64 = app_settings::table
            .filter(app_settings::key.eq(KEY_LAN_ONLY))
            .count()
            .get_result(conn)
            .expect("count");
        assert_eq!(count, 1, "the key has one row regardless of writes");
    });
}

#[test]
#[serial]
fn theme_defaults_to_dark_teal_and_roundtrips() {
    with_test_db(|| {
        assert!(theme_dark(), "a fresh install keeps the dark palette");
        assert_eq!(theme_accent(), crate::theme::Accent::TEAL);
        crate::theme::set(false, "violet").expect("set light violet");
        assert!(!theme_dark());
        assert_eq!(theme_accent().as_str(), "violet");
        crate::theme::set(true, "amber").expect("set dark amber");
        assert!(theme_dark());
        assert_eq!(theme_accent().as_str(), "amber");
    });
}

#[test]
#[serial]
fn theme_rejects_unknown_accent_without_writing() {
    with_test_db(|| {
        crate::theme::set(false, "teal").expect("seed");
        assert!(crate::theme::set(true, "not-a-color").is_err());
        assert!(
            !theme_dark(),
            "a bad accent leaves the dark setting untouched"
        );
        assert_eq!(theme_accent().as_str(), "teal");
    });
}

#[test]
fn accent_cycles_wraps_around() {
    let mut accent = crate::theme::Accent::TEAL;
    for expected in ["blue", "violet", "rose", "amber", "green", "teal"] {
        accent = accent.next();
        assert_eq!(accent.as_str(), expected);
    }
}
