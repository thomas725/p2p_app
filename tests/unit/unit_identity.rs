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
fn user_roundtrip_and_hostile_device_invariant() {
    with_test_db(|| {
        crate::identity::save_user(
            "alice",
            &[0x12; 32],
            Some("alice"),
            None,
        )
        .expect("save user");
        let user = crate::identity::find_user("alice")
            .expect("find user")
            .expect("user exists");
        assert_eq!(user.user_id, "alice");
        assert_eq!(user.account_pubkey, vec![0x12; 32]);
    });
}
