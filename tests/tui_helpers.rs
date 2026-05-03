//! Tests for TUI helper functions

#[test]
fn test_sort_peers() {
    let mut peers = std::collections::VecDeque::from(vec![
        (
            "peer1".to_string(),
            "10:00".to_string(),
            "10:00".to_string(),
        ),
        (
            "peer2".to_string(),
            "09:00".to_string(),
            "09:00".to_string(),
        ),
        (
            "peer3".to_string(),
            "11:00".to_string(),
            "11:00".to_string(),
        ),
    ]);

    let mut peers_vec: Vec<_> = peers.drain(..).collect();
    peers_vec.sort_by(|a, b| b.2.cmp(&a.2));
    peers = peers_vec.into();

    assert_eq!(peers[0].0, "peer3");
    assert_eq!(peers[1].0, "peer1");
    assert_eq!(peers[2].0, "peer2");
}

#[test]
fn test_visibility_range() {
    fn calc_range(total: usize, offset: usize, visible: usize) -> (usize, usize) {
        (
            offset.min(total.saturating_sub(1)),
            (offset + visible).min(total),
        )
    }

    assert_eq!(calc_range(100, 50, 20), (50, 70));
}
