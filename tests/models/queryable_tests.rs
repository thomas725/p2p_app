use crate::generated::models_queryable::*;
use chrono::NaiveDateTime;

fn create_test_message(sent: i32, is_direct: i32, peer_id: Option<&str>) -> Message {
    Message {
        id: 1,
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .unwrap()
            .naive_utc(),
        content: "test".to_string(),
        peer_id: peer_id.map(String::from),
        topic: "test".to_string(),
        sent,
        is_direct,
        target_peer: None,
    }
}

#[test]
fn test_message_is_sent() {
    assert!(create_test_message(1, 0, None).is_sent());
    assert!(!create_test_message(0, 0, None).is_sent());
}

#[test]
fn test_message_is_broadcast() {
    assert!(create_test_message(0, 0, None).is_broadcast());
    assert!(!create_test_message(0, 1, None).is_broadcast());
}

#[test]
fn test_message_is_direct_message() {
    assert!(create_test_message(0, 1, None).is_direct_message());
    assert!(!create_test_message(0, 0, None).is_direct_message());
}

#[test]
fn test_message_is_from_local_user() {
    assert!(create_test_message(0, 0, None).is_from_local_user());
    assert!(!create_test_message(0, 0, Some("peer1")).is_from_local_user());
}

fn create_test_peer(peer_local_nickname: Option<&str>, received_nickname: Option<&str>) -> Peer {
    Peer {
        id: 1,
        created_at: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .unwrap()
            .naive_utc(),
        peer_id: "12D3KooWTestPeerIdABCDEFGH".to_string(),
        addresses: "/ip4/127.0.0.1/tcp/1234".to_string(),
        first_seen: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .unwrap()
            .naive_utc(),
        last_seen: chrono::DateTime::<chrono::Utc>::from_timestamp(0, 0)
            .unwrap()
            .naive_utc(),
        peer_local_nickname: peer_local_nickname.map(String::from),
        received_nickname: received_nickname.map(String::from),
    }
}

#[test]
fn test_peer_display_name_local_nickname() {
    let peer = create_test_peer(Some("Alice"), Some("Bob"));
    assert_eq!(peer.display_name(), "Alice");
}

#[test]
fn test_peer_display_name_received_nickname() {
    let peer = create_test_peer(None, Some("Charlie"));
    assert_eq!(peer.display_name(), "Charlie");
}

#[test]
fn test_peer_display_name_fallback() {
    let peer = create_test_peer(None, None);
    assert_eq!(peer.display_name().len(), 8);
}

#[test]
fn test_peer_address_list() {
    let peer = Peer {
        id: 1,
        created_at: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        peer_id: "12D3KooWTest".to_string(),
        addresses: "/ip4/127.0.0.1/tcp/1234,/ip4/192.168.1.1/tcp/5678,".to_string(),
        first_seen: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        last_seen: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        peer_local_nickname: None,
        received_nickname: None,
    };
    let addrs = peer.address_list();
    assert_eq!(addrs.len(), 2);
    assert_eq!(addrs[0], "/ip4/127.0.0.1/tcp/1234");
    assert_eq!(addrs[1], "/ip4/192.168.1.1/tcp/5678");
}

#[test]
fn test_peer_address_list_empty() {
    let peer = Peer {
        id: 1,
        created_at: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        peer_id: "12D3KooWTest".to_string(),
        addresses: String::new(),
        first_seen: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        last_seen: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        peer_local_nickname: None,
        received_nickname: None,
    };
    assert!(peer.address_list().is_empty());
}

fn create_test_identity(last_tcp: Option<i32>, last_quic: Option<i32>) -> Identity {
    Identity {
        id: 1,
        created_at: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
        key: vec![],
        last_tcp_port: last_tcp,
        last_quic_port: last_quic,
        self_nickname: None,
    }
}

#[test]
fn test_identity_last_port_tcp() {
    let identity = create_test_identity(Some(8080), None);
    assert_eq!(identity.last_port(), Some(8080));
}

#[test]
fn test_identity_last_port_quic() {
    let identity = create_test_identity(None, Some(9090));
    assert_eq!(identity.last_port(), Some(9090));
}

#[test]
fn test_identity_last_port_both() {
    let identity = create_test_identity(Some(8080), Some(9090));
    assert_eq!(identity.last_port(), Some(8080));
}

#[test]
fn test_identity_last_port_none() {
    let identity = create_test_identity(None, None);
    assert_eq!(identity.last_port(), None);
}

#[test]
fn test_peer_session_peer_count() {
    let session = PeerSession {
        id: 1,
        concurrent_peers: 5,
        recorded_at: chrono::NaiveDateTime::from_timestamp_opt(0, 0).unwrap(),
    };
    assert_eq!(session.peer_count(), 5);
}
