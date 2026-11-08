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

fn peer_id() -> libp2p::PeerId {
    libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
}

#[test]
fn kind_roundtrips_through_its_stored_string() {
    assert_eq!(NetworkPeerKind::Bootstrap.as_str(), "bootstrap");
    assert_eq!(NetworkPeerKind::Relay.as_str(), "relay");
    assert_eq!(
        NetworkPeerKind::parse(" Bootstrap "),
        Ok(NetworkPeerKind::Bootstrap)
    );
    assert_eq!(NetworkPeerKind::parse("RELAY"), Ok(NetworkPeerKind::Relay));
    assert!(NetworkPeerKind::parse("dht").is_err());
}

#[test]
fn bootstrap_validation_only_requires_a_parseable_multiaddr() {
    let addr = validate_network_peer(NetworkPeerKind::Bootstrap, " /ip4/1.2.3.4/tcp/4001 ")
        .expect("valid bootstrap");
    assert_eq!(addr, "/ip4/1.2.3.4/tcp/4001");

    assert!(
        validate_network_peer(NetworkPeerKind::Bootstrap, "   ").is_err(),
        "empty address must be rejected"
    );
    assert!(
        validate_network_peer(NetworkPeerKind::Bootstrap, "not-a-multiaddr").is_err(),
        "unparseable address must be rejected"
    );
}

#[test]
fn relay_validation_requires_an_embedded_peer_id() {
    assert!(
        validate_network_peer(NetworkPeerKind::Relay, "/ip4/1.2.3.4/tcp/4001").is_err(),
        "relay without /p2p/<id> must be rejected"
    );

    let peer = peer_id();
    let valid = format!("/ip4/1.2.3.4/tcp/4001/p2p/{peer}");
    assert_eq!(
        validate_network_peer(NetworkPeerKind::Relay, &valid).expect("valid relay"),
        valid
    );
}

#[test]
#[serial]
fn add_list_dedupe_and_remove() {
    with_test_db(|| {
        let bootstrap = "/ip4/1.2.3.4/tcp/4001";
        let relay = format!("/ip4/5.6.7.8/tcp/4001/p2p/{}", peer_id());

        let added = add_network_peer("bootstrap", bootstrap).expect("add bootstrap");
        assert_eq!(added.kind, "bootstrap");
        assert_eq!(added.multiaddr, bootstrap);
        assert!(!added.created_at.is_empty());

        let duplicate = add_network_peer("bootstrap", bootstrap).expect("re-add bootstrap");
        assert_eq!(duplicate.id, added.id, "duplicate add is a no-op");

        let relay_row = add_network_peer("relay", &relay).expect("add relay");
        assert_eq!(relay_row.kind, "relay");

        let listed = list_network_peers().expect("list");
        assert_eq!(listed.len(), 2, "duplicates must not create rows");
        assert_eq!(listed[0], added);

        assert!(remove_network_peer(relay_row.id).expect("remove relay"));
        assert!(
            !remove_network_peer(relay_row.id).expect("remove again"),
            "removing a missing row reports false"
        );
        let listed = list_network_peers().expect("list after remove");
        assert_eq!(listed, vec![added]);
    });
}

#[test]
#[serial]
fn add_rejects_unknown_kind_and_invalid_address() {
    with_test_db(|| {
        assert!(add_network_peer("dht", "/ip4/1.2.3.4/tcp/4001").is_err());
        assert!(add_network_peer("bootstrap", "nope").is_err());
        assert!(add_network_peer("relay", "/ip4/1.2.3.4/tcp/4001").is_err());
        assert!(list_network_peers().expect("list").is_empty());
    });
}

#[test]
#[serial]
fn stored_addrs_are_kind_filtered_and_parsed() {
    with_test_db(|| {
        let relay = format!("/ip4/5.6.7.8/tcp/4001/p2p/{}", peer_id());
        add_network_peer("bootstrap", "/ip4/1.2.3.4/tcp/4001").expect("bootstrap");
        add_network_peer("relay", &relay).expect("relay");

        let bootstrap = stored_bootstrap_addrs();
        assert_eq!(
            bootstrap,
            vec!["/ip4/1.2.3.4/tcp/4001".parse::<Multiaddr>().expect("parse")]
        );
        let relays = stored_relay_addrs();
        assert_eq!(relays, vec![relay.parse::<Multiaddr>().expect("parse")]);
    });
}

#[cfg(feature = "relay")]
#[test]
#[serial]
fn relay_union_keeps_every_persisted_entry() {
    with_test_db(|| {
        let first = format!("/ip4/5.6.7.8/tcp/4001/p2p/{}", peer_id());
        let second = format!("/ip4/9.9.9.9/tcp/4001/p2p/{}", peer_id());
        add_network_peer("relay", &first).expect("first relay");
        add_network_peer("relay", &second).expect("second relay");

        let effective = crate::relay::effective_relay_addrs();
        for addr in &[&first, &second] {
            let addr = addr.parse::<Multiaddr>().expect("parse");
            assert!(
                effective.contains(&addr),
                "persisted relay {addr} missing from the effective list"
            );
        }
    });
}
