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
        crate::identity::save_user("alice", &[0x12; 32], Some("alice"), None).expect("save user");
        let user = crate::identity::find_user("alice")
            .expect("find user")
            .expect("user exists");
        assert_eq!(user.user_id, "alice");
        assert_eq!(user.account_pubkey, vec![0x12; 32]);
    });
}

#[test]
#[serial]
fn peer_resolves_to_bound_user() {
    with_test_db(|| {
        let account_key = libp2p_identity::Keypair::generate_ed25519();
        let account_pubkey = account_key.public().encode_protobuf();
        crate::identity::save_user("bob", &account_pubkey, Some("bob"), None).expect("save user");
        let message = b"bob\x13\x37device1";
        let claim = account_key.sign(message).expect("sign claim");
        crate::peers::save_peer("12D3Kooodevice-peer-1", &[]).expect("save peer");
        crate::identity::save_device_binding(
            "bob",
            "12D3Kooodevice-peer-1",
            &[0x13, 0x37],
            "device1",
            &claim,
        )
        .expect("save device binding");
        let resolved = crate::identity::find_user_id_for_peer("12D3Kooodevice-peer-1")
            .expect("resolve user")
            .expect("user bound to peer");
        assert_eq!(resolved, "bob");
        let unbound = crate::identity::find_user_id_for_peer("12D3Koounbound-peer")
            .expect("resolve unbound peer");
        assert!(unbound.is_none());
    });
}
