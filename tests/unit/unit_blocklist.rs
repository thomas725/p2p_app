use super::*;
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

fn peer_id() -> libp2p::PeerId {
    libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
}

#[test]
fn block_unblock_and_list_roundtrip() {
    with_test_db(|| {
        let peer = peer_id().to_string();
        assert_eq!(list_blocked_peers().expect("list"), Vec::<String>::new());
        assert!(!is_blocked(&peer));

        assert!(block_peer(&peer).expect("block"), "first block inserts");
        assert!(is_blocked(&peer));
        assert_eq!(list_blocked_peers().expect("list"), vec![peer.clone()]);

        // Idempotent: a second block writes nothing and reports it.
        assert!(!block_peer(&peer).expect("re-block"), "already blocked");

        assert!(unblock_peer(&peer).expect("unblock"));
        assert!(!is_blocked(&peer));
        assert_eq!(list_blocked_peers().expect("list"), Vec::<String>::new());

        // Unblocking a peer that is not blocked is a no-op.
        assert!(!unblock_peer(&peer).expect("re-unblock"));
    });
}

#[test]
fn blocking_canonicalizes_the_peer_id() {
    with_test_db(|| {
        let peer = peer_id().to_string();
        let padded = format!("  {peer}  ");
        assert!(block_peer(&padded).expect("block with whitespace"));
        assert!(is_blocked(&peer), "stored in canonical form");
        assert_eq!(list_blocked_peers().expect("list"), vec![peer]);
    });
}

#[test]
fn checking_a_noncanonical_peer_id_matches_the_canonical_row() {
    // `block_peer` stores the canonical form; `is_blocked` must canonicalize
    // its input the same way, or a padded id checks a key that never matches.
    with_test_db(|| {
        let peer = peer_id().to_string();
        assert!(block_peer(&peer).expect("block"), "first block inserts");
        assert!(
            is_blocked(&format!("  {peer}  ")),
            "padded id is still blocked"
        );
    });
}

#[test]
fn invalid_peer_ids_are_rejected() {
    with_test_db(|| {
        let err = block_peer("").expect_err("empty");
        assert_eq!(err, "peer id must not be empty");
        let err = block_peer("not-a-peer-id").expect_err("garbage");
        assert!(err.starts_with("invalid peer id:"), "got: {err}");
        assert!(unblock_peer("not-a-peer-id").is_err());
        assert_eq!(list_blocked_peers().expect("list"), Vec::<String>::new());
    });
}

#[test]
fn blocklist_is_fail_open_when_the_database_cannot_be_opened() {
    // Point the DB at a path whose parent is a regular file: `sqlite_connect`
    // cannot create the directory, so the read fails. The check must then
    // answer "not blocked" rather than muting the whole network. (The
    // DB-writing calls still surface the error.)
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let blocker = dir.path().join("not-a-directory");
    std::fs::write(&blocker, b"x").expect("write blocker file");
    let db_path = blocker.join("test.db");
    crate::db::set_db_url(db_path.to_str().expect("db path"));

    let peer = peer_id().to_string();
    assert!(!is_blocked(&peer), "an unreadable DB must not mute anyone");
    assert!(
        block_peer(&peer).is_err(),
        "writes still report the broken database"
    );

    crate::db::reset_db_url();
}
