use super::ConnectedTracker;

#[test]
fn starts_empty() {
    let t = ConnectedTracker::new();
    assert_eq!(t.len(), 0);
    assert!(t.is_empty());
    assert!(t.last_disconnection().is_none());
}

#[test]
fn records_connections_and_disconnections() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("p1".to_string());
    t.on_peer_connected("p2".to_string());
    assert_eq!(t.len(), 2);
    assert!(t.contains("p1"));
    assert!(t.contains("p2"));
    assert!(!t.contains("p3"));

    t.on_peer_disconnected("p1", 123.0);
    assert_eq!(t.len(), 1);
    assert!(!t.contains("p1"));
    assert_eq!(t.last_disconnection(), Some(("p1", 123.0)));
}

#[test]
fn last_disconnection_is_overwritten() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("p1".to_string());
    t.on_peer_disconnected("p1", 1.0);
    t.on_peer_connected("p2".to_string());
    t.on_peer_disconnected("p2", 2.0);
    assert_eq!(t.last_disconnection(), Some(("p2", 2.0)));
}

#[test]
fn reconnecting_does_not_record_new_last_disconnection() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("p1".to_string());
    t.on_peer_disconnected("p1", 10.0);
    t.on_peer_connected("p1".to_string());
    assert_eq!(t.len(), 1);
    assert_eq!(t.last_disconnection(), Some(("p1", 10.0)));
}

#[test]
fn duplicate_connect_is_idempotent() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("p1".to_string());
    t.on_peer_connected("p1".to_string());
    assert_eq!(t.len(), 1);
}

#[test]
fn connected_peer_ids_iterates_current_set() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("a".to_string());
    t.on_peer_connected("b".to_string());
    let mut ids: Vec<&str> = t.connected_peer_ids().collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["a", "b"]);
    t.on_peer_disconnected("a", 1.0);
    assert_eq!(t.connected_peer_ids().count(), 1);
}

#[test]
fn clear_forgets_everything() {
    let mut t = ConnectedTracker::new();
    t.on_peer_connected("p1".to_string());
    t.on_peer_disconnected("p1", 5.0);
    t.clear();
    assert!(t.is_empty());
    assert_eq!(t.len(), 0);
    assert!(t.last_disconnection().is_none());
}