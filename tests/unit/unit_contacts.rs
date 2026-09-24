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

/// Create a user whose account key we hold, returning the keypair.
fn make_account(user_id: &str) -> libp2p_identity::Keypair {
    let account_key = libp2p_identity::Keypair::generate_ed25519();
    let account_pubkey = account_key.public().encode_protobuf();
    crate::identity::save_user(user_id, &account_pubkey, Some(user_id), Some(user_id))
        .expect("save user");
    account_key
}

/// Save a peer row and bind it into `user_id` with a claim signed by that
/// user's account key over `user_id ‖ device_pubkey ‖ claim_id`.
fn bind_device(
    account_key: &libp2p_identity::Keypair,
    user_id: &str,
    peer_id: &str,
    device_pubkey: &[u8],
    claim_id: &str,
) {
    let mut message = Vec::new();
    message.extend_from_slice(user_id.as_bytes());
    message.extend_from_slice(device_pubkey);
    message.extend_from_slice(claim_id.as_bytes());
    let claim = account_key.sign(&message).expect("sign claim");
    crate::peers::save_peer(peer_id, &[]).expect("save peer");
    crate::identity::save_device_binding(user_id, peer_id, device_pubkey, claim_id, &claim)
        .expect("save binding");
}

#[test]
#[serial]
fn unbound_peers_are_single_device_contacts() {
    with_test_db(|| {
        crate::peers::save_peer("12D3Kooodevice-a", &[]).expect("save peer a");
        crate::peers::save_peer("12D3Kooodevice-b", &[]).expect("save peer b");

        let rows = crate::contacts::load_contacts().expect("load contacts");
        assert_eq!(rows.len(), 2);
        for row in &rows {
            assert!(!row.is_user, "unbound peers are not user contacts");
            assert_eq!(row.device_count, 1);
            assert_eq!(row.device_rows.len(), 1);
            assert_eq!(row.contact_id, row.device_rows[0].peer_id);
        }
    });
}

#[test]
#[serial]
fn multi_device_user_collapses_into_single_contact() {
    with_test_db(|| {
        let account_key = make_account("alice");
        bind_device(
            &account_key,
            "alice",
            "12D3Kooodevice-1",
            &[0xAB; 32],
            "claim-1",
        );
        bind_device(
            &account_key,
            "alice",
            "12D3Kooodevice-2",
            &[0xAB; 32],
            "claim-2",
        );
        crate::peers::save_peer("12D3Kooodevice-3", &[]).expect("save unbound peer");

        let rows = crate::contacts::load_contacts().expect("load contacts");
        assert_eq!(rows.len(), 2, "alice collapsed + one unbound peer");
        let alice = rows.iter().find(|r| r.is_user).expect("alice row");
        assert_eq!(alice.contact_id, "alice");
        assert_eq!(alice.device_count, 2);
        assert_eq!(alice.device_rows.len(), 2);
        let device_ids: std::collections::HashSet<&str> = alice
            .device_rows
            .iter()
            .map(|d| d.peer_id.as_str())
            .collect();
        assert!(device_ids.contains("12D3Kooodevice-1"));
        assert!(device_ids.contains("12D3Kooodevice-2"));

        let unbound = rows.iter().find(|r| !r.is_user).expect("unbound row");
        assert_eq!(unbound.contact_id, "12D3Kooodevice-3");
        assert_eq!(unbound.device_count, 1);
    });
}

#[test]
#[serial]
fn contact_row_aggregates_stats_across_devices() {
    with_test_db(|| {
        let account_key = make_account("bob");
        bind_device(
            &account_key,
            "bob",
            "12D3Kooodevice-1",
            &[0xAB; 32],
            "claim-1",
        );
        bind_device(
            &account_key,
            "bob",
            "12D3Kooodevice-2",
            &[0xAB; 32],
            "claim-2",
        );

        crate::save_message("dm-1", None, "topic-dm", true, Some("12D3Kooodevice-1"))
            .expect("save dm to device 1");
        crate::save_message("dm-2", None, "topic-dm", true, Some("12D3Kooodevice-2"))
            .expect("save dm to device 2");
        crate::peers::record_broadcast_recipients("msg-1", &["12D3Kooodevice-1".to_string()])
            .expect("record broadcast");

        let rows = crate::contacts::load_contacts().expect("load contacts");
        let bob = rows.iter().find(|r| r.is_user).expect("bob row");
        assert_eq!(bob.dm_count, 2, "DMs across both devices are summed");
        assert_eq!(bob.broadcast_sent_to_peer, 1, "broadcasts summed");
        assert_eq!(bob.device_count, 2);
    });
}

#[test]
#[serial]
fn display_name_falls_back_to_user_self_nickname() {
    with_test_db(|| {
        let account_key = make_account("charlie");
        bind_device(
            &account_key,
            "charlie",
            "12D3Kooodevice-1",
            &[0xAB; 32],
            "claim-1",
        );

        let rows = crate::contacts::load_contacts().expect("load contacts");
        let charlie = rows.iter().find(|r| r.is_user).expect("charlie row");
        assert!(
            charlie.display_name.starts_with("charlie ("),
            "falls back to the user's self nickname, got {}",
            charlie.display_name
        );
        assert!(
            charlie.display_name.ends_with(')'),
            "display name carries the (suffix) convention"
        );
    });
}

#[test]
#[serial]
fn owner_nickname_override_wins_over_self_nickname() {
    with_test_db(|| {
        // This install must be bound to a user to *own* overrides.
        let owner_key = make_account("me");
        let local_peer = crate::get_local_peer_id().expect("local peer id");
        bind_device(
            &owner_key,
            "me",
            &local_peer.to_string(),
            &[0x42; 32],
            "claim-me",
        );

        let contact_key = make_account("dave");
        bind_device(
            &contact_key,
            "dave",
            "12D3Kooodevice-1",
            &[0xAB; 32],
            "claim-1",
        );

        crate::identity::add_contact("me", "dave", Some("Davey")).expect("add contact");

        let rows = crate::contacts::load_contacts().expect("load contacts");
        let dave = rows
            .iter()
            .find(|r| r.contact_id == "dave")
            .expect("dave row");
        assert!(
            dave.display_name.starts_with("Davey ("),
            "owner-side override wins, got {}",
            dave.display_name
        );

        // Remove the override; the self nickname wins again.
        crate::identity::remove_contact("me", "dave").expect("remove contact");
        let rows = crate::contacts::load_contacts().expect("reload contacts");
        let dave = rows
            .iter()
            .find(|r| r.contact_id == "dave")
            .expect("dave row");
        assert!(
            dave.display_name.starts_with("dave ("),
            "self nickname wins once the override is removed, got {}",
            dave.display_name
        );
    });
}

#[test]
fn build_contact_rows_is_pure_and_groups_live_inputs() {
    use crate::contacts::{ContactDeviceInput, ContactRow};
    use crate::messages::PeerMessageStats;
    let devices = vec![
        ContactDeviceInput {
            peer_id: "dev-A".to_string(),
            first_seen: "2024-01-01 08:00:00".to_string(),
            last_seen: "2024-03-01 09:00:00".to_string(),
            received_nickname: None,
            local_nickname: None,
        },
        ContactDeviceInput {
            peer_id: "dev-B".to_string(),
            first_seen: "2024-01-02 08:00:00".to_string(),
            last_seen: "2024-02-01 09:00:00".to_string(),
            received_nickname: None,
            local_nickname: None,
        },
        ContactDeviceInput {
            peer_id: "dev-C".to_string(),
            first_seen: "2024-01-01 08:00:00".to_string(),
            last_seen: "2024-01-10 09:00:00".to_string(),
            received_nickname: None,
            local_nickname: None,
        },
    ];
    let display_names = std::collections::HashMap::from([
        ("dev-A".to_string(), "Device A".to_string()),
        ("dev-B".to_string(), "Device B".to_string()),
        ("dev-C".to_string(), "Device C".to_string()),
    ]);
    let peer_user = std::collections::HashMap::from([
        ("dev-A".to_string(), "alice".to_string()),
        ("dev-B".to_string(), "alice".to_string()),
    ]);
    let users = std::collections::HashMap::new();
    let overrides = std::collections::HashMap::new();
    let stats = std::collections::HashMap::from([
        (
            "dev-A".to_string(),
            PeerMessageStats {
                dm_count: 3,
                broadcast_sent_to_peer: 1,
            },
        ),
        (
            "dev-B".to_string(),
            PeerMessageStats {
                dm_count: 2,
                broadcast_sent_to_peer: 4,
            },
        ),
    ]);

    let rows =
        crate::contacts::build_contact_rows(&devices, &display_names, &peer_user, &users, &overrides, &stats);
    assert_eq!(rows.len(), 2, "alice collapsed + device-C alone");

    let alice = rows
        .iter()
        .find(|r| r.contact_id == "alice")
        .expect("alice row");
    assert!(alice.is_user);
    assert_eq!(alice.device_count, 2);
    assert_eq!(alice.dm_count, 5, "DMs summed across devices");
    assert_eq!(alice.broadcast_sent_to_peer, 5, "broadcasts summed");
    assert_eq!(alice.first_seen, "2024-01-01 08:00:00", "group min");
    assert_eq!(alice.last_seen, "2024-03-01 09:00:00", "group max");

    let solo = rows
        .iter()
        .find(|r| r.contact_id == "dev-C")
        .expect("device C row");
    assert!(!solo.is_user);

    // sort_canonical_rows: name desc with stable contact-key ties.
    let sorted = crate::contacts::sort_canonical_rows(&rows, 0, false);
    let ids: Vec<&str> = sorted.iter().map(|r: &ContactRow| r.contact_id.as_str()).collect();
    assert_eq!(ids, vec!["dev-C", "alice"], "name desc via display names");
}
