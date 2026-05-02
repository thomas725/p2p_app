//! Tests for nickname.rs module

#[test]
fn test_nickname_generation_format() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(!nick.is_empty());
    assert!(nick.contains('-'));
}
