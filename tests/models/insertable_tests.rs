use crate::models_insertable::*;
use chrono::NaiveDateTime;

#[test]
fn test_new_identity_default() {
    let identity = NewIdentity::new(vec![1, 2, 3]);
    assert_eq!(identity.key, vec![1, 2, 3]);
    assert_eq!(identity.last_tcp_port, None);
    assert_eq!(identity.last_quic_port, None);
    assert_eq!(identity.self_nickname, None);
}

#[test]
fn test_new_message_broadcast() {
    let msg = NewMessage::broadcast("Hello".to_string(), "test-topic");
    assert_eq!(msg.content, "Hello");
    assert_eq!(msg.topic, "test-topic");
    assert_eq!(msg.peer_id, None);
    assert_eq!(msg.sent, 0);
    assert_eq!(msg.is_direct, 0);
    assert_eq!(msg.target_peer, None);
}

#[test]
fn test_new_message_direct() {
    let msg = NewMessage::direct("Private".to_string(), "test-topic", "peer123");
    assert_eq!(msg.content, "Private");
    assert_eq!(msg.topic, "test-topic");
    assert_eq!(msg.peer_id, None);
    assert_eq!(msg.sent, 0);
    assert_eq!(msg.is_direct, 1);
    assert_eq!(msg.target_peer, Some("peer123".to_string()));
}

#[test]
fn test_new_peer_session() {
    let session = NewPeerSession::new(5);
    assert_eq!(session.concurrent_peers, 5);
    assert!(session.recorded_at <= chrono::Utc::now().naive_utc());
}

#[test]
fn test_new_peer() {
    let peer = NewPeer::new(
        "peer123".to_string(),
        vec!["/ip4/127.0.0.1/tcp/8080".to_string()],
    );
    assert_eq!(peer.peer_id, "peer123");
    assert_eq!(peer.addresses, "/ip4/127.0.0.1/tcp/8080");
    assert_eq!(peer.peer_local_nickname, None);
    assert_eq!(peer.received_nickname, None);
    assert!(peer.first_seen <= chrono::Utc::now().naive_utc());
    assert!(peer.last_seen <= chrono::Utc::now().naive_utc());
}

#[test]
fn test_new_peer_with_nickname() {
    let peer = NewPeer::new(
        "peer123".to_string(),
        vec!["/ip4/127.0.0.1/tcp/8080".to_string()],
    )
    .with_nickname("Alice".to_string());
    assert_eq!(peer.peer_local_nickname, Some("Alice".to_string()));
}

#[test]
fn test_new_peer_multiple_addresses() {
    let peer = NewPeer::new(
        "peer123".to_string(),
        vec![
            "/ip4/127.0.0.1/tcp/8080".to_string(),
            "/ip4/192.168.1.1/tcp/9090".to_string(),
        ],
    );
    assert!(peer.addresses.contains("/ip4/127.0.0.1/tcp/8080"));
    assert!(peer.addresses.contains("/ip4/192.168.1.1/tcp/9090"));
}