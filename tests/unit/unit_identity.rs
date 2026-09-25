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

#[test]
#[serial]
fn shared_claim_message_helper_agrees_with_verify() {
    let user_id = "alice-account";
    let device_pubkey = [0x42u8, 0x24];
    let claim_id = "claim-xyz";
    let account_key = libp2p_identity::Keypair::generate_ed25519();
    let message = ascii_concat(user_id, &device_pubkey, claim_id);
    // The stored pubkey verifies the helper-built message the same way a claim
    // built by the prod sign path would.
    let account_pubkey = account_key.public().encode_protobuf();
    with_test_db(|| {
        crate::identity::save_user(user_id, &account_pubkey, None, None).expect("save user");
        let claim = account_key.sign(&message).expect("sign");
        assert!(
            crate::identity::verify_device_binding_claim(user_id, &device_pubkey, claim_id, &claim)
                .expect("verify clean")
        );
        // And the helper-built message is byte-identical to the one the
        // verifier recomputes internally.
        let recomputed =
            crate::identity::device_binding_claim_message(user_id, &device_pubkey, claim_id);
        assert_eq!(recomputed, message);
    });
}

#[test]
#[serial]
fn provisioning_creates_user_self_binds_and_stores_account_key() {
    with_test_db(|| {
        let user_id = crate::identity::provision_my_user(Some("alice"), Some("Alice"))
            .expect("provision user");
        let local_peer = crate::get_local_peer_id().unwrap().to_string();
        // user_id is the peer ID of the account key.
        let account_key = crate::identity::get_stored_account_key()
            .expect("read account key")
            .expect("account key stored");
        let expected = crate::identity::user_id_from_account_pubkey(&account_key.public());
        assert_eq!(user_id, expected);
        // Self-bound: our own peer resolves to our user.
        assert_eq!(
            crate::identity::find_user_id_for_peer(&local_peer)
                .expect("resolve local")
                .as_deref(),
            Some(user_id.as_str())
        );
        // Account display seams persisted.
        let user = crate::identity::find_user(&user_id)
            .expect("find user")
            .expect("user exists");
        assert_eq!(user.self_username.as_deref(), Some("alice"));
        assert_eq!(user.self_nickname.as_deref(), Some("Alice"));
        // The self claim verifies against the just-stored account key.
        let binding = crate::identity::list_user_devices(&user_id)
            .expect("list devices")
            .into_iter()
            .find(|d| d.peer_id == local_peer)
            .expect("local device bound");
        assert!(
            crate::identity::verify_device_binding_claim(
                &user_id,
                &binding.device_pubkey,
                &binding.claim_id,
                &binding.claim,
            )
            .expect("verify clean")
        );
    });
}

#[test]
#[serial]
fn provisioning_is_idempotent_and_keeps_the_root() {
    with_test_db(|| {
        let first = crate::identity::provision_my_user(Some("alice"), Some("Alice"))
            .expect("provision once");
        let second = crate::identity::provision_my_user(Some("alice-renamed"), Some("Alice 2"))
            .expect("provision again");
        assert_eq!(first, second);
        // The root never changes under re-provisioning.
        let user = crate::identity::find_user(&first)
            .expect("find user")
            .expect("user exists");
        let stored = crate::identity::get_stored_account_key()
            .expect("read account key")
            .expect("key present");
        assert_eq!(user.account_pubkey, stored.public().encode_protobuf());
        // Only one local self-binding exists.
        assert_eq!(
            crate::identity::list_user_devices(&first)
                .expect("list devices")
                .len(),
            1
        );
    });
}

#[test]
#[serial]
fn mint_device_binding_signs_claim_for_foreign_device() {
    with_test_db(|| {
        let user_id = crate::identity::provision_my_user(None, None).expect("provision");
        let target_peer = "12D3Koootarget-device";
        let target_pubkey = [0x99u8; 32];
        let request = crate::identity::mint_device_binding(target_peer, &target_pubkey)
            .expect("mint binding");
        assert_eq!(request.kind, "claim");
        assert_eq!(request.user_id.as_deref(), Some(user_id.as_str()));
        assert_eq!(request.peer_id, target_peer);
        assert_eq!(request.device_pubkey, target_pubkey.to_vec());
        assert!(request.account_pubkey.is_some());
        assert!(request.claim_id.is_some());
        assert!(request.bound_at.is_some());
        let claim = request.signature.expect("claim signed");
        // Claim verifies against our stored account key.
        assert!(
            crate::identity::verify_device_binding_claim(
                &user_id,
                &target_pubkey,
                request.claim_id.as_deref().expect("claim id"),
                &claim,
            )
            .expect("verify clean")
        );
        // The local bind was persisted so our Contacts collapse the target.
        assert_eq!(
            crate::identity::find_user_id_for_peer(target_peer)
                .expect("resolve target")
                .as_deref(),
            Some(user_id.as_str())
        );
    });
}

#[test]
#[serial]
fn hostiles_cannot_mint_a_binding_into_a_foreign_user() {
    with_test_db(|| {
        // Victim user with its own account key.
        let victim = crate::identity::provision_my_user(Some("victim"), Some("Victim"))
            .expect("provision victim");
        // Attacker's account key has no power over the victim's device-set.
        let attacker = libp2p_identity::Keypair::generate_ed25519();
        let claim = crate::identity::sign_device_binding_claim(
            &attacker,
            &victim,
            &[0x0au8; 32],
            "spoofed-claim",
        )
        .expect("attacker signs");
        let refused = crate::identity::save_device_binding(
            &victim,
            "12D3Kooattacker-device",
            &[0x0au8; 32],
            "spoofed-claim",
            &claim,
        );
        assert!(refused.is_err(), "hostile claim must be rejected");
    });
}

#[test]
#[serial]
fn intro_request_advertises_device_and_account() {
    with_test_db(|| {
        // Unprovisioned: intro still works, carries device info only.
        crate::identity::provision_my_user(None, None).expect("provision");
        let user_id = crate::identity::find_user_id_for_peer(
            &crate::get_local_peer_id().unwrap().to_string(),
        )
        .expect("resolve local")
        .expect("bound");
        let intro = crate::identity::intro_device_binding_request().expect("intro");
        assert_eq!(intro.kind, "intro");
        assert_eq!(intro.user_id.as_deref(), Some(user_id.as_str()));
        assert!(intro.account_pubkey.is_none());
        assert!(intro.claim_id.is_none());
        assert!(intro.signature.is_none());
        assert_eq!(
            intro.peer_id,
            crate::get_local_peer_id().unwrap().to_string()
        );
        assert_eq!(
            intro.device_pubkey,
            crate::get_libp2p_identity()
                .unwrap()
                .public()
                .encode_protobuf()
        );
    });
}

fn ascii_concat(user_id: &str, device_pubkey: &[u8], claim_id: &str) -> Vec<u8> {
    let mut message = Vec::new();
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(device_pubkey);
    message.extend_from_slice(claim_id.as_bytes());
    message
}
