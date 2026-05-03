//! Tests for network.rs module

use p2p_app::network::{NetworkSize, get_network_size};

#[test]
fn test_network_size_small() {
    assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(1.0), NetworkSize::Small);
    assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);
}

#[test]
fn test_network_size_medium() {
    assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(10.0), NetworkSize::Medium);
    assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);
}

#[test]
fn test_network_size_large() {
    assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    assert_eq!(NetworkSize::from_peer_count(100.0), NetworkSize::Large);
}

#[test]
fn test_network_size_debug() {
    let debug_small = format!("{:?}", NetworkSize::Small);
    let debug_medium = format!("{:?}", NetworkSize::Medium);
    let debug_large = format!("{:?}", NetworkSize::Large);
    assert!(debug_small.contains("Small"));
    assert!(debug_medium.contains("Medium"));
    assert!(debug_large.contains("Large"));
}

#[test]
fn test_network_size_clone() {
    let small = NetworkSize::Small;
    let cloned = small.clone();
    assert_eq!(small, cloned);
}

#[test]
fn test_network_size_partial_eq() {
    assert_eq!(NetworkSize::Small, NetworkSize::Small);
    assert_ne!(NetworkSize::Small, NetworkSize::Medium);
}
