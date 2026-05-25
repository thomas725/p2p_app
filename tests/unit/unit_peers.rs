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
#[serial(db)]
fn save_and_load_peers() {
    let _db = setup_test_db();
    save_peer("peer-a", &["/ip4/127.0.0.1/tcp/1234".to_string()]).expect("save peer");
    let loaded = load_peers().expect("load peers");
    assert!(loaded.iter().any(|p| p.peer_id == "peer-a"));
}

#[test]
#[serial(db)]
fn load_known_peers_includes_message_only_peer() {
    let _db = setup_test_db();
    crate::save_message("hello", Some("peer-msg"), "topic-k", false, None).expect("save msg");
    let known = load_known_peers().expect("known peers");
    assert!(known.iter().any(|p| p.peer_id == "peer-msg"));
}

#[test]
#[serial(db)]
fn peer_session_aggregates_work() {
    let _db = setup_test_db();
    save_peer_session(2).expect("save session");
    save_peer_session(4).expect("save session");
    let avg = get_average_peer_count().expect("avg");
    let recent = get_recent_peer_count().expect("recent");
    assert_eq!(avg, 3.0);
    assert_eq!(recent, 4);
}

#[test]
#[serial(db)]
fn average_peer_count_is_zero_when_no_sessions() {
    let _db = setup_test_db();
    let avg = get_average_peer_count().expect("avg");
    assert_eq!(avg, 0.0);
}

#[test]
#[serial(db)]
fn save_and_load_listen_ports() {
    let _db = setup_test_db();
    save_listen_ports(Some(3000), Some(4000)).expect("save ports");
    let (tcp, quic) = load_listen_ports().expect("load ports");
    assert_eq!(tcp, Some(3000));
    assert_eq!(quic, Some(4000));
}
