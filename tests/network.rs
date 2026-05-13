//! Tests for network.rs module

use p2p_app::network::NetworkSize;

#[test]
fn test_network_size_display() {
    assert_eq!(format!("{}", NetworkSize::Small), "Small");
    assert_eq!(format!("{}", NetworkSize::Medium), "Medium");
    assert_eq!(format!("{}", NetworkSize::Large), "Large");
}

#[test]
fn test_network_size_from_peer_count() {
    // Small: 0-20 peers
    assert_eq!(NetworkSize::from_peer_count(5), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(20), NetworkSize::Small);
    
    // Medium: 21-100 peers
    assert_eq!(NetworkSize::from_peer_count(50), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(100), NetworkSize::Medium);
    
    // Large: 100+ peers
    assert_eq!(NetworkSize::from_peer_count(101), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(1000), NetworkSize::Large);
}

#[test]
fn test_network_size_boundaries() {
    // Test exact boundaries
    assert_eq!(NetworkSize::from_peer_count(0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(21), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(100), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(101), NetworkSize::Large);
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
fn test_network_size_clone() {
    let size = NetworkSize::Large;
    let cloned = size.clone();
    assert_eq!(size, cloned);
}
