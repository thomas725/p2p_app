//! Tests for network.rs module

use p2p_app::network::NetworkSize;

#[test]
fn test_network_size_display() {
    assert_eq!(NetworkSize::Small.to_string(), "Small");
    assert_eq!(NetworkSize::Medium.to_string(), "Medium");
    assert_eq!(NetworkSize::Large.to_string(), "Large");
}

#[test]
fn test_network_size_from_peer_count() {
    // Small: 0-3 peers
    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);

    // Medium: 4-15 peers
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);

    // Large: 16+ peers
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);
}

#[test]
fn test_network_size_boundaries() {
    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);
}

#[test]
fn test_network_size_equality() {
    assert_eq!(NetworkSize::Small, NetworkSize::Small);
    assert_eq!(NetworkSize::Medium, NetworkSize::Medium);
    assert_eq!(NetworkSize::Large, NetworkSize::Large);
    assert_ne!(NetworkSize::Small, NetworkSize::Medium);
    assert_ne!(NetworkSize::Medium, NetworkSize::Large);
}

#[test]
fn test_network_size_copy() {
    let size = NetworkSize::Large;
    let copied = size;
    assert_eq!(size, copied);
}
