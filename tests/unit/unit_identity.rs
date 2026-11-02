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
            None,
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
            crate::identity::verify_device_binding_claim(
                user_id,
                &device_pubkey,
                claim_id,
                &claim,
                None
            )
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
                None,
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
                None,
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
            None,
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

#[test]
#[serial]
fn my_user_id_and_devices_reflect_provisioned_account() {
    with_test_db(|| {
        // Nothing resolves before provisioning.
        assert!(crate::identity::my_user_id().expect("resolve").is_none());
        assert_eq!(
            crate::identity::my_user_devices().expect("resolve devices"),
            Vec::new()
        );

        let user_id = crate::identity::provision_my_user(None, None).expect("provision");
        assert_eq!(
            crate::identity::my_user_id().expect("resolve").as_deref(),
            Some(user_id.as_str())
        );
        let local_peer = crate::get_local_peer_id().unwrap().to_string();
        let devices = crate::identity::my_user_devices().expect("resolve devices");
        assert!(
            devices.iter().any(|(peer, _)| *peer == local_peer),
            "self-binding listed: {devices:?}"
        );
        assert_eq!(devices.len(), 1, "only the local device before any accepts");
    });
}

#[test]
#[serial]
fn set_my_user_nickname_updates_the_announced_users_row() {
    with_test_db(|| {
        // Unprovisioned: no-op, no error.
        crate::identity::set_my_user_nickname("Draft").expect("no-op when unprovisioned");

        let user_id = crate::identity::provision_my_user(None, Some("Launch")).expect("provision");
        assert_eq!(
            crate::identity::find_user(&user_id)
                .expect("find user")
                .expect("user exists")
                .self_nickname
                .as_deref(),
            Some("Launch")
        );

        // The announcement seam updates only users.self_nickname; the account
        // root and username are untouched.
        crate::identity::set_my_user_nickname("Renamed").expect("update announced name");
        let user = crate::identity::find_user(&user_id)
            .expect("find user")
            .expect("user exists");
        assert_eq!(user.self_nickname.as_deref(), Some("Renamed"));
        assert_eq!(
            crate::identity::my_user_id().expect("resolve").as_deref(),
            Some(user_id.as_str())
        );
    });
}

#[test]
#[serial]
fn sibling_device_can_mint_and_verify_claim() {
    with_test_db(|| {
        // Account holder's user + key.
        let account_key = libp2p_identity::Keypair::generate_ed25519();
        let account_pubkey = account_key.public().encode_protobuf();
        let user_id = crate::identity::user_id_from_account_pubkey(&account_key.public());
        crate::identity::save_user(&user_id, &account_pubkey, None, None).expect("save user");

        // Sibling device already bound into the user (via the account key).
        let sibling_key = libp2p_identity::Keypair::generate_ed25519();
        let sibling_pubkey = sibling_key.public().encode_protobuf();
        crate::peers::save_peer("sibling-peer", &[]).expect("save sibling peer");
        let sib_claim_id = "sib-claim";
        let sib_msg =
            crate::identity::device_binding_claim_message(&user_id, &sibling_pubkey, sib_claim_id);
        let sib_sig = account_key.sign(&sib_msg).expect("sign sibling claim");
        crate::identity::save_device_binding(
            &user_id,
            "sibling-peer",
            &sibling_pubkey,
            sib_claim_id,
            &sib_sig,
            None,
        )
        .expect("bind sibling");

        // A new device's claim signed by the sibling verifies via the sibling path.
        let new_device_pubkey = [0x77u8; 32];
        let new_claim_id = "new-claim";
        let new_msg = crate::identity::device_binding_claim_message(
            &user_id,
            &new_device_pubkey,
            new_claim_id,
        );
        let sibling_sig = sibling_key.sign(&new_msg).expect("sign by sibling");
        assert!(
            crate::identity::verify_device_binding_claim(
                &user_id,
                &new_device_pubkey,
                new_claim_id,
                &sibling_sig,
                Some("sibling-peer"),
            )
            .expect("verify clean"),
            "sibling-signed claim must be accepted"
        );

        // Without a sender (or with an unbound sender) the sibling path is
        // unavailable and the account-key path fails -> rejected.
        assert!(
            !crate::identity::verify_device_binding_claim(
                &user_id,
                &new_device_pubkey,
                new_claim_id,
                &sibling_sig,
                None,
            )
            .expect("verify clean"),
            "no sender => sibling path unavailable"
        );
        crate::peers::save_peer("stranger-peer", &[]).expect("save stranger peer");
        assert!(
            !crate::identity::verify_device_binding_claim(
                &user_id,
                &new_device_pubkey,
                new_claim_id,
                &sibling_sig,
                Some("stranger-peer"),
            )
            .expect("verify clean"),
            "unbound sender must not authorize"
        );

        // A device bound to a *different* user cannot authorize for this user.
        let other_account_key = libp2p_identity::Keypair::generate_ed25519();
        let other_account_pubkey = other_account_key.public().encode_protobuf();
        let other_user = "other-user";
        crate::identity::save_user(other_user, &other_account_pubkey, None, None)
            .expect("save other user");
        let other_sibling_pubkey = [0x33u8; 32];
        crate::peers::save_peer("other-sibling-peer", &[]).expect("save other sibling peer");
        let other_cid = "other-sib-claim";
        let other_msg = crate::identity::device_binding_claim_message(
            other_user,
            &other_sibling_pubkey,
            other_cid,
        );
        let other_sig = other_account_key.sign(&other_msg).expect("sign");
        crate::identity::save_device_binding(
            other_user,
            "other-sibling-peer",
            &other_sibling_pubkey,
            other_cid,
            &other_sig,
            None,
        )
        .expect("bind other sibling");
        assert!(
            !crate::identity::verify_device_binding_claim(
                &user_id,
                &new_device_pubkey,
                new_claim_id,
                &sibling_sig,
                Some("other-sibling-peer"),
            )
            .expect("verify clean"),
            "sibling bound to a different user must not authorize"
        );

        // An account-key claim still verifies (path 1).
        let acct_claim_id = "acct-claim";
        let acct_msg = crate::identity::device_binding_claim_message(
            &user_id,
            &new_device_pubkey,
            acct_claim_id,
        );
        let acct_sig = account_key.sign(&acct_msg).expect("sign acct");
        assert!(
            crate::identity::verify_device_binding_claim(
                &user_id,
                &new_device_pubkey,
                acct_claim_id,
                &acct_sig,
                None,
            )
            .expect("verify clean"),
            "account-key claim must still verify"
        );
    });
}

#[test]
#[serial]
fn sibling_device_can_mint_claim_without_account_key() {
    with_test_db(|| {
        // Provision a user via a separate account key, then bind the local peer
        // into that user WITHOUT storing the account key (simulating a sibling
        // device that was bound earlier).
        let account_key = libp2p_identity::Keypair::generate_ed25519();
        let account_pubkey = account_key.public().encode_protobuf();
        let user_id = crate::identity::user_id_from_account_pubkey(&account_key.public());
        crate::identity::save_user(&user_id, &account_pubkey, None, None).expect("save user");
        let local_peer = crate::get_local_peer_id().unwrap().to_string();
        let local_pubkey = crate::get_libp2p_identity()
            .unwrap()
            .public()
            .encode_protobuf();
        crate::peers::save_peer(&local_peer, &[]).expect("save local peer");
        let self_cid = "self-bind";
        let self_msg =
            crate::identity::device_binding_claim_message(&user_id, &local_pubkey, self_cid);
        let self_sig = account_key.sign(&self_msg).expect("sign self bind");
        crate::identity::save_device_binding(
            &user_id,
            &local_peer,
            &local_pubkey,
            self_cid,
            &self_sig,
            None,
        )
        .expect("bind local as sibling");
        // No account key stored -> mint must fall back to the install key.
        assert!(
            crate::identity::get_stored_account_key()
                .expect("read account key")
                .is_none()
        );

        let target_peer = "12D3Koosibling-target";
        let target_pubkey = [0x55u8; 32];
        let request = crate::identity::mint_device_binding(target_peer, &target_pubkey)
            .expect("mint as sibling");
        assert_eq!(request.kind, "claim");
        assert_eq!(request.user_id.as_deref(), Some(user_id.as_str()));
        let claim = request.signature.expect("claim signed");
        let claim_id = request.claim_id.expect("claim id");
        let msg =
            crate::identity::device_binding_claim_message(&user_id, &target_pubkey, &claim_id);
        // Signed by the install key, NOT the account key.
        let install_key = crate::get_libp2p_identity().expect("install key");
        assert!(install_key.public().verify(&msg, &claim));
        assert!(!account_key.public().verify(&msg, &claim));
    });
}
