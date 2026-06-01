use super::*;
use chrono::NaiveDateTime;
use p2p_app::generated::models_queryable::{MessageReceipt, Peer};
use p2p_app::peers::KnownPeer;

fn dt(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
}

// ── extract_nickname_maps ──────────────────────────────────────────────

#[test]
fn test_extract_nickname_maps_empty() {
    let (l, r, s) = extract_nickname_maps(&[]);
    assert!(l.is_empty());
    assert!(r.is_empty());
    assert!(s.is_empty());
}

#[test]
fn test_extract_nickname_maps_all_fields() {
    let peers = [Peer {
        id: 1,
        created_at: dt("2024-01-01 12:00:00"),
        peer_id: "peer-1".to_string(),
        addresses: "".to_string(),
        first_seen: dt("2024-01-01 12:00:00"),
        last_seen: dt("2024-01-01 12:00:00"),
        peer_local_nickname: Some("Alice".to_string()),
        self_nickname_for_peer: Some("SelfA".to_string()),
        received_nickname: Some("Bob".to_string()),
    }];
    let (l, r, s) = extract_nickname_maps(&peers);
    assert_eq!(l.get("peer-1"), Some(&"Alice".to_string()));
    assert_eq!(r.get("peer-1"), Some(&"Bob".to_string()));
    assert_eq!(s.get("peer-1"), Some(&"SelfA".to_string()));
}

#[test]
fn test_extract_nickname_maps_partial_fields() {
    let peers = [
        Peer {
            id: 1,
            created_at: dt("2024-01-01 12:00:00"),
            peer_id: "p1".to_string(),
            addresses: "".to_string(),
            first_seen: dt("2024-01-01 12:00:00"),
            last_seen: dt("2024-01-01 12:00:00"),
            peer_local_nickname: Some("Local".to_string()),
            self_nickname_for_peer: None,
            received_nickname: None,
        },
        Peer {
            id: 2,
            created_at: dt("2024-01-01 12:00:00"),
            peer_id: "p2".to_string(),
            addresses: "".to_string(),
            first_seen: dt("2024-01-01 12:00:00"),
            last_seen: dt("2024-01-01 12:00:00"),
            peer_local_nickname: None,
            self_nickname_for_peer: Some("SelfB".to_string()),
            received_nickname: Some("RecB".to_string()),
        },
    ];
    let (l, r, s) = extract_nickname_maps(&peers);
    assert_eq!(l.len(), 1);
    assert_eq!(r.len(), 1);
    assert_eq!(s.len(), 1);
    assert_eq!(l.get("p1"), Some(&"Local".to_string()));
    assert_eq!(r.get("p2"), Some(&"RecB".to_string()));
    assert_eq!(s.get("p2"), Some(&"SelfB".to_string()));
}

// ── deduplicate_peers ──────────────────────────────────────────────────

#[test]
fn test_deduplicate_peers_empty() {
    let result = deduplicate_peers(&[]);
    assert!(result.is_empty());
}

#[test]
fn test_deduplicate_peers_no_duplicates() {
    let peers = [
        KnownPeer {
            peer_id: "p1".to_string(),
            first_seen: dt("2024-01-01 12:00:00"),
            last_seen: dt("2024-01-02 12:00:00"),
        },
        KnownPeer {
            peer_id: "p2".to_string(),
            first_seen: dt("2024-01-03 12:00:00"),
            last_seen: dt("2024-01-04 12:00:00"),
        },
    ];
    let result = deduplicate_peers(&peers);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].0, "p1");
    assert_eq!(result[0].1, "2024-01-01 12:00:00");
    assert_eq!(result[0].2, "2024-01-02 12:00:00");
}

#[test]
fn test_deduplicate_peers_skips_duplicates() {
    let peers = [
        KnownPeer {
            peer_id: "p1".to_string(),
            first_seen: dt("2024-01-01 12:00:00"),
            last_seen: dt("2024-01-02 12:00:00"),
        },
        KnownPeer {
            peer_id: "p1".to_string(),
            first_seen: dt("2024-01-01 12:00:00"),
            last_seen: dt("2024-01-03 12:00:00"),
        },
        KnownPeer {
            peer_id: "p2".to_string(),
            first_seen: dt("2024-01-04 12:00:00"),
            last_seen: dt("2024-01-05 12:00:00"),
        },
    ];
    let result = deduplicate_peers(&peers);
    assert_eq!(result.len(), 2);
    // First occurrence (most recent last_seen in DB order) is kept
    assert_eq!(result[0].0, "p1");
    assert_eq!(result[0].2, "2024-01-02 12:00:00");
    assert_eq!(result[1].0, "p2");
}

// ── organize_receipts ──────────────────────────────────────────────────

#[test]
fn test_organize_receipts_empty() {
    let (b, d) = organize_receipts(&[]);
    assert!(b.is_empty());
    assert!(d.is_empty());
}

#[test]
fn test_organize_receipts_broadcast_kind_0() {
    let receipts = [MessageReceipt {
        id: 1,
        msg_id: "msg-1".to_string(),
        peer_id: "peer-a".to_string(),
        kind: 0,
        confirmed_at: 100.0,
        created_at: dt("2024-01-01 12:00:00"),
    }];
    let (b, d) = organize_receipts(&receipts);
    assert_eq!(b.len(), 1);
    assert_eq!(b.get("msg-1").unwrap().get("peer-a"), Some(&100.0));
    assert!(d.is_empty());
}

#[test]
fn test_organize_receipts_dm_kind_1() {
    let receipts = [MessageReceipt {
        id: 1,
        msg_id: "dm-1".to_string(),
        peer_id: "peer-b".to_string(),
        kind: 1,
        confirmed_at: 200.0,
        created_at: dt("2024-01-01 12:00:00"),
    }];
    let (b, d) = organize_receipts(&receipts);
    assert!(b.is_empty());
    assert_eq!(d.len(), 1);
    assert_eq!(d.get("dm-1"), Some(&("peer-b".to_string(), 200.0)));
}

#[test]
fn test_organize_receipts_mixed() {
    let receipts = [
        MessageReceipt {
            id: 1,
            msg_id: "b-1".to_string(),
            peer_id: "pa".to_string(),
            kind: 0,
            confirmed_at: 10.0,
            created_at: dt("2024-01-01 12:00:00"),
        },
        MessageReceipt {
            id: 2,
            msg_id: "d-1".to_string(),
            peer_id: "pb".to_string(),
            kind: 1,
            confirmed_at: 20.0,
            created_at: dt("2024-01-01 12:00:00"),
        },
    ];
    let (b, d) = organize_receipts(&receipts);
    assert_eq!(b.len(), 1);
    assert_eq!(d.len(), 1);
}

#[test]
fn test_organize_receipts_multiple_peers_same_msg() {
    let receipts = [
        MessageReceipt {
            id: 1,
            msg_id: "b-1".to_string(),
            peer_id: "pa".to_string(),
            kind: 0,
            confirmed_at: 10.0,
            created_at: dt("2024-01-01 12:00:00"),
        },
        MessageReceipt {
            id: 2,
            msg_id: "b-1".to_string(),
            peer_id: "pb".to_string(),
            kind: 0,
            confirmed_at: 20.0,
            created_at: dt("2024-01-01 12:00:00"),
        },
    ];
    let (b, d) = organize_receipts(&receipts);
    assert_eq!(b.len(), 1);
    assert_eq!(b.get("b-1").unwrap().len(), 2);
    assert!(d.is_empty());
}

#[test]
fn test_recent_tui_logs_returns_last_100_in_order() {
    let logs: Vec<String> = (0..120).map(|i| format!("log-{i}")).collect();
    let recent = recent_tui_logs(&logs, 100);
    assert_eq!(recent.len(), 100);
    assert_eq!(recent.first().map(String::as_str), Some("log-20"));
    assert_eq!(recent.last().map(String::as_str), Some("log-119"));
}

#[test]
fn test_recent_tui_logs_handles_short_lists() {
    let logs: Vec<String> = vec!["a".into(), "b".into()];
    let recent = recent_tui_logs(&logs, 100);
    assert_eq!(recent, logs);
}
