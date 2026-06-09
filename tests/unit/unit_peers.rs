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
fn save_and_load_peers() {
    with_test_db(|| {
        save_peer("peer-a", &["/ip4/127.0.0.1/tcp/1234".to_string()]).expect("save peer");
        let loaded = load_peers().expect("load peers");
        assert!(loaded.iter().any(|p| p.peer_id == "peer-a"));
    });
}

#[test]
#[serial(db)]
fn load_known_peers_includes_message_only_peer() {
    with_test_db(|| {
        crate::save_message("hello", Some("peer-msg"), "topic-k", false, None).expect("save msg");
        let known = load_known_peers().expect("known peers");
        assert!(known.iter().any(|p| p.peer_id == "peer-msg"));
    });
}

#[test]
#[serial(db)]
fn peer_session_aggregates_work() {
    with_test_db(|| {
        save_peer_session(2).expect("save session");
        save_peer_session(4).expect("save session");
        let avg = get_average_peer_count().expect("avg");
        let recent = get_recent_peer_count().expect("recent");
        assert_eq!(avg, 3.0);
        assert_eq!(recent, 4);
    });
}

#[test]
#[serial(db)]
fn average_peer_count_is_zero_when_no_sessions() {
    with_test_db(|| {
        let avg = get_average_peer_count().expect("avg");
        assert_eq!(avg, 0.0);
    });
}

#[test]
#[serial(db)]
fn save_and_load_listen_ports() {
    with_test_db(|| {
        save_listen_ports(Some(3000), Some(4000)).expect("save ports");
        let (tcp, quic) = load_listen_ports().expect("load ports");
        assert_eq!(tcp, Some(3000));
        assert_eq!(quic, Some(4000));
    });
}
