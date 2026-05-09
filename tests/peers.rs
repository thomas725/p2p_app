//! Tests for peers.rs module

mod test_utils;
use serial_test::serial;
use test_utils::setup_test_db;

// ── save_peer / load_peers ────────────────────────────────────────────────────

#[serial]
#[test]
fn test_save_peer_creates_record() {
    let _db = setup_test_db();
    let peer = p2p_app::save_peer("peer-001", &["addr1".to_string()]).unwrap();
    assert_eq!(peer.peer_id, "peer-001");
    assert_eq!(peer.addresses, "addr1");
}

#[serial]
#[test]
fn test_save_peer_multiple_addresses() {
    let _db = setup_test_db();
    let addrs = vec!["addr1".to_string(), "addr2".to_string()];
    let peer = p2p_app::save_peer("peer-002", &addrs).unwrap();
    assert_eq!(peer.addresses, "addr1,addr2");
}

#[serial]
#[test]
fn test_save_peer_upserts_on_conflict() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-003", &["old-addr".to_string()]).unwrap();
    let peer = p2p_app::save_peer("peer-003", &["new-addr".to_string()]).unwrap();
    assert_eq!(peer.peer_id, "peer-003");
    assert_eq!(peer.addresses, "new-addr");
}

#[serial]
#[test]
fn test_load_peers_empty() {
    let _db = setup_test_db();
    let peers = p2p_app::load_peers().unwrap();
    assert!(peers.is_empty());
}

#[serial]
#[test]
fn test_load_peers_returns_all() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-a", &[]).unwrap();
    p2p_app::save_peer("peer-b", &[]).unwrap();
    let peers = p2p_app::load_peers().unwrap();
    assert_eq!(peers.len(), 2);
}

#[serial]
#[test]
fn test_load_peers_ordered_by_last_seen_desc() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-first", &[]).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(5));
    p2p_app::save_peer("peer-second", &[]).unwrap();
    let peers = p2p_app::load_peers().unwrap();
    assert_eq!(peers[0].peer_id, "peer-second");
    assert_eq!(peers[1].peer_id, "peer-first");
}

// ── load_known_peers ──────────────────────────────────────────────────────────

#[serial]
#[test]
fn test_load_known_peers_empty() {
    let _db = setup_test_db();
    let peers = p2p_app::load_known_peers().unwrap();
    assert!(peers.is_empty());
}

#[serial]
#[test]
fn test_load_known_peers_from_peers_table() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-known", &[]).unwrap();
    let peers = p2p_app::load_known_peers().unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].peer_id, "peer-known");
}

#[serial]
#[test]
fn test_load_known_peers_from_messages_table() {
    let _db = setup_test_db();
    // Peer only appears in messages, not peers table
    p2p_app::save_message("msg", Some("peer-msg-only"), "topic", false, None).unwrap();
    let peers = p2p_app::load_known_peers().unwrap();
    assert!(peers.iter().any(|p| p.peer_id == "peer-msg-only"));
}

#[serial]
#[test]
fn test_load_known_peers_merges_both_tables() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-in-table", &[]).unwrap();
    p2p_app::save_message("msg", Some("peer-in-messages"), "topic", false, None).unwrap();
    let peers = p2p_app::load_known_peers().unwrap();
    assert_eq!(peers.len(), 2);
    let ids: Vec<&str> = peers.iter().map(|p| p.peer_id.as_str()).collect();
    assert!(ids.contains(&"peer-in-table"));
    assert!(ids.contains(&"peer-in-messages"));
}

#[serial]
#[test]
fn test_load_known_peers_deduplicates() {
    let _db = setup_test_db();
    // Peer in both tables should appear only once
    p2p_app::save_peer("peer-dup", &[]).unwrap();
    p2p_app::save_message("msg", Some("peer-dup"), "topic", false, None).unwrap();
    let peers = p2p_app::load_known_peers().unwrap();
    let count = peers.iter().filter(|p| p.peer_id == "peer-dup").count();
    assert_eq!(count, 1);
}

// ── save_peer_session / get_average_peer_count / get_recent_peer_count ────────

#[serial]
#[test]
fn test_save_peer_session() {
    let _db = setup_test_db();
    p2p_app::save_peer_session(5).unwrap();
    assert_eq!(p2p_app::get_recent_peer_count().unwrap(), 5);
}

#[serial]
#[test]
fn test_get_recent_peer_count_zero_when_empty() {
    let _db = setup_test_db();
    assert_eq!(p2p_app::get_recent_peer_count().unwrap(), 0);
}

#[serial]
#[test]
fn test_get_recent_peer_count_returns_latest() {
    let _db = setup_test_db();
    p2p_app::save_peer_session(3).unwrap();
    p2p_app::save_peer_session(7).unwrap();
    assert_eq!(p2p_app::get_recent_peer_count().unwrap(), 7);
}

#[serial]
#[test]
fn test_get_average_peer_count_zero_when_empty() {
    let _db = setup_test_db();
    assert_eq!(p2p_app::get_average_peer_count().unwrap(), 0.0);
}

#[serial]
#[test]
fn test_get_average_peer_count() {
    let _db = setup_test_db();
    p2p_app::save_peer_session(4).unwrap();
    p2p_app::save_peer_session(8).unwrap();
    let avg = p2p_app::get_average_peer_count().unwrap();
    assert!((avg - 6.0).abs() < 0.001);
}

// ── save_listen_ports / load_listen_ports ────────────────────────────────────

#[serial]
#[test]
fn test_load_listen_ports_defaults_none() {
    let _db = setup_test_db();
    // Need identity row first
    p2p_app::get_libp2p_identity().unwrap();
    let (tcp, quic) = p2p_app::load_listen_ports().unwrap();
    assert!(tcp.is_none());
    assert!(quic.is_none());
}

#[serial]
#[test]
fn test_save_and_load_listen_ports() {
    let _db = setup_test_db();
    p2p_app::get_libp2p_identity().unwrap();
    p2p_app::save_listen_ports(Some(4001), Some(4002)).unwrap();
    let (tcp, quic) = p2p_app::load_listen_ports().unwrap();
    assert_eq!(tcp, Some(4001));
    assert_eq!(quic, Some(4002));
}

#[serial]
#[test]
fn test_save_listen_ports_overwrite() {
    let _db = setup_test_db();
    p2p_app::get_libp2p_identity().unwrap();
    p2p_app::save_listen_ports(Some(4001), Some(4002)).unwrap();
    p2p_app::save_listen_ports(Some(5001), None).unwrap();
    let (tcp, quic) = p2p_app::load_listen_ports().unwrap();
    assert_eq!(tcp, Some(5001));
    assert!(quic.is_none());
}

// ── get_network_size ──────────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_network_size_small_when_empty() {
    let _db = setup_test_db();
    let size = p2p_app::get_network_size().unwrap();
    assert_eq!(size, p2p_app::network::NetworkSize::Small);
}

#[serial]
#[test]
fn test_get_network_size_medium() {
    let _db = setup_test_db();
    for _ in 0..5 { p2p_app::save_peer_session(8).unwrap(); }
    let size = p2p_app::get_network_size().unwrap();
    assert_eq!(size, p2p_app::network::NetworkSize::Medium);
}

#[serial]
#[test]
fn test_get_network_size_large() {
    let _db = setup_test_db();
    for _ in 0..5 { p2p_app::save_peer_session(20).unwrap(); }
    let size = p2p_app::get_network_size().unwrap();
    assert_eq!(size, p2p_app::network::NetworkSize::Large);
}
