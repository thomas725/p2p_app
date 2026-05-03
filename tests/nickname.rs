//! Tests for nickname.rs module

#[test]
fn test_nickname_generation_format() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(!nick.is_empty());
    assert!(nick.contains('-'));
}

#[test]
fn test_nickname_generation_uniqueness() {
    let nick1 = p2p_app::nickname::generate_self_nickname();
    let nick2 = p2p_app::nickname::generate_self_nickname();
    assert_ne!(nick1, nick2);
}

#[test]
fn test_nickname_generation_has_parts() {
    let nick = p2p_app::nickname::generate_self_nickname();
    let parts: Vec<&str> = nick.split('-').collect();
    assert_eq!(parts.len(), 2);
}
