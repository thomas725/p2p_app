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
    temp_env::with_var(
        "DATABASE_URL",
        Some(db_path.to_str().expect("db path")),
        || {
            crate::db::init_database().expect("init db");
            f();
            crate::db::release_db_lock();
            crate::db::reset_db_url_cache();
        },
    );
}

#[test]
#[serial(db)]
fn self_nickname_roundtrip() {
    with_test_db(|| {
        set_self_nickname("my-nick").expect("set");
        let loaded = get_self_nickname().expect("get");
        assert_eq!(loaded.as_deref(), Some("my-nick"));
    });
}

#[test]
#[serial(db)]
fn ensure_self_nickname_sets_if_missing() {
    with_test_db(|| {
        let nick = ensure_self_nickname().expect("ensure");
        assert!(!nick.is_empty());
        let loaded = get_self_nickname().expect("get");
        assert_eq!(loaded.as_deref(), Some(nick.as_str()));
    });
}

#[test]
#[serial(db)]
fn ensure_self_nickname_returns_existing() {
    with_test_db(|| {
        set_self_nickname("already-set").expect("set");
        let nick = ensure_self_nickname().expect("ensure");
        assert_eq!(nick, "already-set");
    });
}

#[test]
#[serial(db)]
fn peer_nickname_precedence_local_over_received() {
    with_test_db(|| {
        set_peer_received_nickname("peer-1", "received").expect("set received");
        set_peer_local_nickname("peer-1", "local").expect("set local");
        assert_eq!(
            get_peer_received_nickname("peer-1").expect("get received"),
            Some("received".to_string())
        );
        assert_eq!(
            get_peer_local_nickname("peer-1").expect("get local"),
            Some("local".to_string())
        );
        let display = get_peer_display_name("peer-1").expect("display");
        assert!(display.starts_with("local "));
    });
}

#[test]
#[serial(db)]
fn peer_self_nickname_for_peer_roundtrip() {
    with_test_db(|| {
        set_peer_self_nickname_for_peer("peer-2", "remote-self").expect("set");
        let loaded = get_peer_self_nickname_for_peer("peer-2").expect("get");
        assert_eq!(loaded.as_deref(), Some("remote-self"));
    });
}

#[test]
#[serial(db)]
fn peer_display_name_received_then_fallback() {
    with_test_db(|| {
        set_peer_received_nickname("peer-r", "recv").expect("set received");
        let received_display = get_peer_display_name("peer-r").expect("display");
        assert!(received_display.starts_with("recv "));

        let fallback = get_peer_display_name("peer-fallback").expect("fallback");
        assert_eq!(fallback, crate::fmt::short_peer_id("peer-fallback"));
    });
}
