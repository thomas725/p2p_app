#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;

fn peer_id() -> libp2p::PeerId {
    libp2p::identity::Keypair::generate_ed25519()
        .public()
        .to_peer_id()
}

#[test]
fn parse_relay_addrs_skips_entries_without_a_peer_id() {
    let relays = parse_relay_addrs("/ip4/1.2.3.4/tcp/4001, ,not-a-multiaddr");
    assert!(
        relays.is_empty(),
        "relays without /p2p/<id> must be dropped"
    );
}

#[test]
fn parse_relay_addrs_keeps_valid_entries_and_dedupes() {
    let peer = peer_id();
    let valid = format!("/ip4/1.2.3.4/tcp/4001/p2p/{peer}");
    let input = format!("{valid}, {valid}");
    let relays = parse_relay_addrs(&input);
    assert_eq!(relays, vec![valid.parse::<Multiaddr>().unwrap()]);
}

#[test]
fn relay_peer_id_reads_the_embedded_peer() {
    let peer = peer_id();
    let addr = format!("/ip4/1.2.3.4/tcp/4001/p2p/{peer}")
        .parse::<Multiaddr>()
        .unwrap();
    assert_eq!(relay_peer_id(&addr), Some(peer));
    let plain = "/ip4/1.2.3.4/tcp/4001".parse::<Multiaddr>().unwrap();
    assert_eq!(relay_peer_id(&plain), None);
}

#[test]
fn relayed_listen_addr_appends_the_circuit_protocol_once() {
    let peer = peer_id();
    let relay = format!("/ip4/1.2.3.4/tcp/4001/p2p/{peer}")
        .parse::<Multiaddr>()
        .unwrap();
    let relayed = relayed_listen_addr(&relay).unwrap();
    assert!(relayed.to_string().ends_with("/p2p-circuit"));
    assert_eq!(relayed_listen_addr(&relayed), Some(relayed.clone()));
}

#[test]
fn relayed_listen_addr_requires_a_relay_peer_id() {
    let plain = "/ip4/1.2.3.4/tcp/4001".parse::<Multiaddr>().unwrap();
    assert_eq!(relayed_listen_addr(&plain), None);
}
