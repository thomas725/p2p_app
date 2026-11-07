#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;
use network_interface::{Addr, NetworkInterface, V4IfAddr, V6IfAddr};
use serial_test::serial;
use std::net::{Ipv4Addr, Ipv6Addr};

fn v4(name: &str, ip: [u8; 4]) -> NetworkInterface {
    NetworkInterface::new_afinet(name, Ipv4Addr::from(ip), None, None, 1, false)
}

fn v6(name: &str, ip: Ipv6Addr) -> NetworkInterface {
    NetworkInterface::new_afinet6(name, ip, None, None, 1, false)
}

#[test]
#[serial(net_diag)]
fn snapshot_defaults_to_empty_after_reset() {
    reset();
    let details = snapshot();
    assert!(details.listen_addrs.is_empty());
    assert!(details.external_addrs.is_empty());
    assert!(details.external_candidates.is_empty());
    assert_eq!(details.nat_status, "Unknown");
    assert_eq!(details.reachability, "Unknown");
    assert_eq!(details.nat_probe_successes, 0);
    assert_eq!(details.nat_probe_failures, 0);
    assert_eq!(details.hole_punch_attempts, 0);
    assert_eq!(details.hole_punch_successes, 0);
    assert!(details.relay_reservations.is_empty());
    assert_eq!(details.relay_status, "No relay reservation");
    assert_eq!(details.dht_mode, "Unknown");
    assert!(!details.dht_connected);
    assert_eq!(details.dht_routing_peers, 0);
    assert!(!details.dht_announced);
    assert_eq!(details.dht_announce_status, "Not announced yet");
    assert!(details.dht_announce_at.is_none());
}

#[test]
#[serial(net_diag)]
fn listen_and_external_addresses_dedupe_and_expire() {
    reset();
    record_listen_addr("/ip4/192.168.1.2/tcp/4001");
    record_listen_addr("/ip4/192.168.1.2/tcp/4001");
    record_listen_addr("/ip6/::1/tcp/4001");
    record_external_candidate("/ip4/203.0.113.7/tcp/4001");
    record_external_candidate("/ip4/203.0.113.7/tcp/4001");
    record_external_addr("/ip4/203.0.113.7/tcp/4001");

    let details = snapshot();
    assert_eq!(
        details.listen_addrs,
        vec![
            "/ip4/192.168.1.2/tcp/4001".to_string(),
            "/ip6/::1/tcp/4001".to_string()
        ]
    );
    assert_eq!(
        details.external_candidates,
        vec!["/ip4/203.0.113.7/tcp/4001".to_string()]
    );
    assert_eq!(
        details.external_addrs,
        vec!["/ip4/203.0.113.7/tcp/4001".to_string()]
    );

    expire_external_addr("/ip4/203.0.113.7/tcp/4001");
    let details = snapshot();
    assert!(details.external_addrs.is_empty());
    assert!(details.external_candidates.is_empty());
    // Listen addresses are unaffected by expiry.
    assert_eq!(details.listen_addrs.len(), 2);
}

#[test]
#[serial(net_diag)]
fn nat_probes_count_and_public_status_wins_reachability() {
    reset();
    record_nat_probe(true);
    record_nat_probe(true);
    record_nat_probe(false);
    record_nat_status("Public", Some("/ip4/203.0.113.7/tcp/4001"));

    let details = snapshot();
    assert_eq!(details.nat_probe_successes, 2);
    assert_eq!(details.nat_probe_failures, 1);
    assert_eq!(details.nat_status, "Public");
    assert_eq!(
        details.nat_public_addr.as_deref(),
        Some("/ip4/203.0.113.7/tcp/4001")
    );
    assert_eq!(details.reachability, "Public (directly reachable)");
}

#[test]
#[serial(net_diag)]
fn private_status_and_relay_decide_reachability() {
    reset();
    record_nat_status("Private", None);
    assert_eq!(snapshot().reachability, "Private (behind NAT)");

    record_relay_reservation("12D3KooWRelay");
    let details = snapshot();
    assert_eq!(
        details.relay_reservations,
        vec!["12D3KooWRelay".to_string()]
    );
    assert_eq!(details.relay_status, "Reserved via 12D3KooWRelay");
    assert_eq!(details.reachability, "Relayed (reachable via a relay)");

    // A confirmed external address (but no relay/public verdict) is weaker.
    reset();
    record_external_addr("/ip4/203.0.113.7/tcp/4001");
    assert_eq!(snapshot().reachability, "External address known");

    // NAT Private is reported once no relay/external evidence exists.
    reset();
    record_nat_status("Private", None);
    assert_eq!(snapshot().reachability, "Private (behind NAT)");

    // An unknown NAT verdict with no evidence stays Unknown.
    reset();
    assert_eq!(snapshot().reachability, "Unknown");
}

#[test]
#[serial(net_diag)]
fn relay_status_override_keeps_reservations() {
    reset();
    record_relay_reservation("relay-a");
    record_relay_status("Reservation renewal failed");
    let details = snapshot();
    assert_eq!(details.relay_reservations, vec!["relay-a".to_string()]);
    assert_eq!(details.relay_status, "Reservation renewal failed");
}

#[test]
#[serial(net_diag)]
fn hole_punch_counts_attempts_and_successes() {
    reset();
    record_hole_punch(true);
    record_hole_punch(false);
    record_hole_punch(true);
    let details = snapshot();
    assert_eq!(details.hole_punch_attempts, 3);
    assert_eq!(details.hole_punch_successes, 2);
}

#[test]
#[serial(net_diag)]
fn dht_connectivity_tracks_mode_and_routing_peers() {
    reset();
    assert!(!snapshot().dht_connected);

    record_dht_routing_peers(0);
    record_dht_mode("Client");
    assert!(!snapshot().dht_connected);

    record_dht_routing_peers(4);
    let details = snapshot();
    assert_eq!(details.dht_mode, "Client");
    assert_eq!(details.dht_routing_peers, 4);
    assert!(details.dht_connected);

    // Server mode alone implies routability even with an empty table.
    reset();
    record_dht_mode("Server");
    let details = snapshot();
    assert_eq!(details.dht_mode, "Server");
    assert!(details.dht_connected);

    // Dropping back to Client with an empty table disconnects.
    record_dht_mode("Client");
    record_dht_routing_peers(0);
    assert!(!snapshot().dht_connected);
}

#[test]
#[serial(net_diag)]
fn dht_bootstrap_outcomes_are_counted() {
    reset();
    record_dht_bootstrap(true);
    record_dht_bootstrap(false);
    record_dht_bootstrap(false);
    let details = snapshot();
    assert_eq!(details.dht_bootstrap_successes, 1);
    assert_eq!(details.dht_bootstrap_failures, 2);
}

#[test]
#[serial(net_diag)]
fn announcement_lifecycle_and_rate_limit() {
    reset();
    assert!(announce_due(), "reset clears the rate-limit timestamp");
    assert!(!snapshot().dht_announced);

    record_dht_announce_attempt();
    let details = snapshot();
    assert_eq!(details.dht_announce_status, "Announcing…");
    assert!(
        !announce_due(),
        "an attempt inside the interval suppresses the next one"
    );

    record_dht_announce_result(true, "Published to DHT");
    let details = snapshot();
    assert!(details.dht_announced);
    assert_eq!(details.dht_announce_status, "Published to DHT");
    assert!(details.dht_announce_at.is_some());

    clear_announcement();
    let details = snapshot();
    assert!(!details.dht_announced);
    assert_eq!(details.dht_announce_status, "Not announced yet");
    assert!(announce_due(), "clearing re-arms the next announcement");

    let announced_at = snapshot().dht_announce_at;
    record_dht_announce_result(false, "PutRecord failed");
    let details = snapshot();
    assert!(!details.dht_announced);
    assert_eq!(details.dht_announce_status, "PutRecord failed");
    assert_eq!(
        details.dht_announce_at, announced_at,
        "a failed re-announce keeps the last successful timestamp"
    );
}

#[test]
#[serial(net_diag)]
fn announce_record_json_round_trips() {
    let payload = announce_record_json(
        "12D3KooWSelf",
        &["/ip4/203.0.113.7/tcp/4001".to_string()],
        true,
    )
    .expect("serialization");
    let parsed: serde_json::Value = serde_json::from_slice(&payload).expect("valid json");
    assert_eq!(parsed["peerId"], "12D3KooWSelf");
    assert_eq!(parsed["addresses"][0], "/ip4/203.0.113.7/tcp/4001");
    assert_eq!(parsed["public"], true);

    let empty = announce_record_json("peer", &[], false).expect("serialization");
    let parsed: serde_json::Value = serde_json::from_slice(&empty).expect("valid json");
    assert_eq!(parsed["addresses"], serde_json::json!([]));
    assert_eq!(parsed["public"], false);
}

#[test]
fn loopback_multiaddrs_are_recognised() {
    assert!(is_loopback_addr("/ip4/127.0.0.1/tcp/4001"));
    assert!(is_loopback_addr("/ip4/127.9.9.9/tcp/4001"));
    assert!(is_loopback_addr("/ip6/::1/tcp/4001"));
    assert!(is_loopback_addr("/ip6/0:0:0:0:0:0:0:1/tcp/4001"));
    assert!(!is_loopback_addr("/ip4/192.168.1.2/tcp/4001"));
    assert!(!is_loopback_addr("/ip6/fe80::1/tcp/4001"));
    assert!(!is_loopback_addr("/dns4/example.org/tcp/4001"));
}

#[test]
fn reachable_addresses_skip_loopback_and_dedupe() {
    let details = NetworkDetails {
        listen_addrs: vec![
            "/ip4/127.0.0.1/tcp/4001".to_string(),
            "/ip4/192.168.1.2/tcp/4001".to_string(),
            "/ip4/203.0.113.7/tcp/4001".to_string(),
        ],
        external_addrs: vec![
            "/ip6/::1/tcp/4001".to_string(),
            "/ip4/203.0.113.7/tcp/4001".to_string(),
        ],
        ..NetworkDetails::default()
    };
    assert_eq!(
        details.reachable_addresses(),
        vec![
            "/ip4/203.0.113.7/tcp/4001".to_string(),
            "/ip4/192.168.1.2/tcp/4001".to_string()
        ]
    );
}

#[test]
fn group_interfaces_folds_addresses_per_name() {
    let list = vec![
        v4("eth0", [192, 168, 1, 2]).with_mac_addr(Some("aa:bb:cc:dd:ee:ff".to_string())),
        v4("eth0", [10, 0, 0, 2]),
        v6("eth0", Ipv6Addr::LOCALHOST),
        v4("lo", [127, 0, 0, 1]),
        v4("wlan0", [192, 168, 1, 9]),
    ];
    let rows = group_interfaces(list);
    assert_eq!(
        rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>(),
        vec!["eth0", "lo", "wlan0"]
    );

    let eth0 = rows.first().expect("eth0 row");
    assert_eq!(eth0.mac.as_deref(), Some("aa:bb:cc:dd:ee:ff"));
    assert_eq!(eth0.ipv4, vec!["192.168.1.2", "10.0.0.2"]);
    assert_eq!(eth0.ipv6, vec!["::1"]);
    assert!(!eth0.internal);

    let wlan0 = rows.get(2).expect("wlan0 row");
    assert!(wlan0.mac.is_none(), "empty MACs are dropped");
    assert_eq!(wlan0.ipv4, vec!["192.168.1.9"]);
    assert!(wlan0.ipv6.is_empty());
}

#[test]
fn group_interfaces_keeps_internal_flag_and_tolerates_empty_mac() {
    let mut lo = v4("lo", [127, 0, 0, 1]);
    lo.internal = true;
    let list = vec![
        lo,
        v4("eth0", [10, 0, 0, 2]).with_mac_addr(Some(String::new())),
    ];
    let rows = group_interfaces(list);
    let lo = rows.iter().find(|row| row.name == "lo").expect("lo row");
    assert!(lo.internal);
    let eth0 = rows
        .iter()
        .find(|row| row.name == "eth0")
        .expect("eth0 row");
    assert!(eth0.mac.is_none());
}

#[test]
fn group_interfaces_dedupes_repeated_addresses() {
    let list = vec![v4("eth0", [10, 0, 0, 2]), v4("eth0", [10, 0, 0, 2])];
    let rows = group_interfaces(list);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows.first().expect("eth0").ipv4, vec!["10.0.0.2"]);
}

#[test]
fn grouped_addresses_are_copied_from_the_crate_types() {
    // Guards the match arms in `group_interfaces` against a crate API change.
    let addr = Addr::V4(V4IfAddr {
        ip: Ipv4Addr::new(192, 168, 0, 5),
        broadcast: None,
        netmask: None,
    });
    assert_eq!(addr.ip().to_string(), "192.168.0.5");
    let addr = Addr::V6(V6IfAddr {
        ip: Ipv6Addr::LOCALHOST,
        broadcast: None,
        netmask: None,
    });
    assert_eq!(addr.ip().to_string(), "::1");
}

#[test]
#[serial(net_diag)]
fn relay_circuit_addresses_complete_the_self_peer_id() {
    reset();
    // Confirmed form (what the relay client places in `external_addrs`).
    record_external_addr("/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1");
    // Raw listener form can omit the trailing self id.
    record_listen_addr("/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit");

    let details = snapshot();
    assert_eq!(
        details.relay_circuit_addresses("self-1"),
        vec!["/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1".to_string()]
    );
}

#[test]
#[serial(net_diag)]
fn announce_addresses_prefers_circuits_behind_nat() {
    reset();
    record_nat_status("Private", None);
    record_listen_addr("/ip4/192.168.1.9/tcp/4001");
    record_external_addr("/ip4/203.0.113.7/tcp/4001");
    record_external_addr("/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1");
    record_relay_reservation("relay-a");

    let details = snapshot();
    assert_eq!(
        details.announce_addresses("self-1"),
        vec!["/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1".to_string()],
        "behind NAT only the relayed circuit is dialable"
    );

    // A private node without a reservation has nothing better than the normal
    // reachable list, so it keeps the old behaviour (mDNS peers can still
    // dial it); circuit addresses found there are appended unchanged.
    expire_relay_reservation("relay-a");
    let details = snapshot();
    assert_eq!(
        details.announce_addresses("self-1"),
        vec![
            "/ip4/203.0.113.7/tcp/4001".to_string(),
            "/ip4/192.168.1.9/tcp/4001".to_string(),
            "/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1".to_string(),
        ]
    );
}

#[test]
#[serial(net_diag)]
fn announce_addresses_appends_circuits_when_not_private() {
    reset();
    record_nat_status("Public", Some("/ip4/203.0.113.7/tcp/4001"));
    record_external_addr("/ip4/203.0.113.7/tcp/4001");
    record_listen_addr("/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit");

    let details = snapshot();
    assert_eq!(
        details.announce_addresses("self-1"),
        vec![
            "/ip4/203.0.113.7/tcp/4001".to_string(),
            "/ip4/198.51.100.9/tcp/7000/p2p/relay-a/p2p-circuit/p2p/self-1".to_string(),
        ]
    );
}

#[test]
#[serial(net_diag)]
fn expire_relay_reservation_clears_the_last_status() {
    reset();
    record_relay_reservation("relay-a");
    expire_relay_reservation("relay-a");
    let details = snapshot();
    assert!(details.relay_reservations.is_empty());
    assert_eq!(details.relay_status, "No relay reservation");
}
