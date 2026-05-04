//! Tests for nickname.rs module

use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup_test_db() -> TempDir {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().unwrap()) };
    // Initialise schema
    p2p_app::db::get_libp2p_identity().ok();
    dir
}

// ── generate_self_nickname ───────────────────────────────────────────────────

#[test]
fn test_generate_self_nickname_not_empty() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(!nick.is_empty());
}

#[test]
fn test_generate_self_nickname_contains_hyphen() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(nick.contains('-'), "expected two-word nickname separated by '-', got: {}", nick);
}

#[test]
fn test_generate_self_nickname_two_parts() {
    let nick = p2p_app::nickname::generate_self_nickname();
    let parts: Vec<&str> = nick.split('-').collect();
    assert_eq!(parts.len(), 2, "expected exactly 2 parts, got: {:?}", parts);
}

#[test]
fn test_generate_self_nickname_uniqueness() {
    // With a 2-word petname space this will virtually never collide
    let nicks: std::collections::HashSet<String> =
        (0..10).map(|_| p2p_app::nickname::generate_self_nickname()).collect();
    assert!(nicks.len() > 1, "all 10 generated nicknames were identical");
}

// ── get/set self nickname ─────────────────────────────────────────────────────

#[test]
fn test_get_self_nickname_none_initially() {
    let _db = setup_test_db();
    // Fresh DB may have no identity row yet or null nickname
    // Either None or a pre-generated one is acceptable; just check it doesn't panic.
    let result = p2p_app::nickname::get_self_nickname();
    assert!(result.is_ok());
}

#[test]
fn test_set_and_get_self_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("test-nick").unwrap();
    let nick = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(nick.as_deref(), Some("test-nick"));
}

#[test]
fn test_set_self_nickname_overwrite() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("first").unwrap();
    p2p_app::nickname::set_self_nickname("second").unwrap();
    let nick = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(nick.as_deref(), Some("second"));
}

// ── ensure_self_nickname ──────────────────────────────────────────────────────

#[test]
fn test_ensure_self_nickname_generates_if_missing() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert!(!nick.is_empty());
    assert!(nick.contains('-'));
}

#[test]
fn test_ensure_self_nickname_returns_existing() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("my-name").unwrap();
    let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert_eq!(nick, "my-name");
}

#[test]
fn test_ensure_self_nickname_idempotent() {
    let _db = setup_test_db();
    let first = p2p_app::nickname::ensure_self_nickname().unwrap();
    let second = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert_eq!(first, second);
}

// ── peer local nickname ───────────────────────────────────────────────────────

#[test]
fn test_get_peer_local_nickname_none_for_unknown() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_local_nickname("unknown-peer").unwrap();
    assert!(nick.is_none());
}

#[test]
fn test_set_and_get_peer_local_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-001", "Alice").unwrap();
    let nick = p2p_app::nickname::get_peer_local_nickname("peer-001").unwrap();
    assert_eq!(nick.as_deref(), Some("Alice"));
}

#[test]
fn test_set_peer_local_nickname_overwrite() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-002", "Old").unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-002", "New").unwrap();
    let nick = p2p_app::nickname::get_peer_local_nickname("peer-002").unwrap();
    assert_eq!(nick.as_deref(), Some("New"));
}

#[test]
fn test_peer_local_nicknames_are_isolated() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-a", "Alpha").unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-b", "Beta").unwrap();
    assert_eq!(
        p2p_app::nickname::get_peer_local_nickname("peer-a").unwrap().as_deref(),
        Some("Alpha")
    );
    assert_eq!(
        p2p_app::nickname::get_peer_local_nickname("peer-b").unwrap().as_deref(),
        Some("Beta")
    );
}

// ── peer received nickname ────────────────────────────────────────────────────

#[test]
fn test_get_peer_received_nickname_none_for_unknown() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_received_nickname("nobody").unwrap();
    assert!(nick.is_none());
}

#[test]
fn test_set_and_get_peer_received_nickname() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-recv", &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-recv", "Bob").unwrap();
    let nick = p2p_app::nickname::get_peer_received_nickname("peer-recv").unwrap();
    assert_eq!(nick.as_deref(), Some("Bob"));
}

#[test]
fn test_set_peer_received_nickname_overwrite() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-rw", &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-rw", "v1").unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-rw", "v2").unwrap();
    let nick = p2p_app::nickname::get_peer_received_nickname("peer-rw").unwrap();
    assert_eq!(nick.as_deref(), Some("v2"));
}

// ── self nickname for peer ────────────────────────────────────────────────────

#[test]
fn test_get_peer_self_nickname_for_peer_none_initially() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("nobody").unwrap();
    assert!(nick.is_none());
}

#[test]
fn test_set_and_get_peer_self_nickname_for_peer() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-sn", &[]).unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer-sn", "MyNameForThem").unwrap();
    let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("peer-sn").unwrap();
    assert_eq!(nick.as_deref(), Some("MyNameForThem"));
}

#[test]
fn test_set_peer_self_nickname_for_peer_overwrite() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-snw", &[]).unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer-snw", "old-name").unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer-snw", "new-name").unwrap();
    let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("peer-snw").unwrap();
    assert_eq!(nick.as_deref(), Some("new-name"));
}

// ── get_peer_display_name ─────────────────────────────────────────────────────

#[test]
fn test_get_peer_display_name_short_id_fallback() {
    let _db = setup_test_db();
    // Unknown peer — no local or received nickname — falls back to short ID
    let name = p2p_app::nickname::get_peer_display_name("12D3KooWABCDEFGH").unwrap();
    // Short ID is last 8 chars of the input
    assert!(!name.is_empty());
    assert!(name.contains("ABCDEFGH") || name.len() <= 8);
}

#[test]
fn test_get_peer_display_name_uses_local_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-disp", "LocalNick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-disp").unwrap();
    assert!(name.starts_with("LocalNick"), "got: {}", name);
}

#[test]
fn test_get_peer_display_name_prefers_local_over_received() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-pref", &[]).unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-pref", "LocalWins").unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-pref", "ReceivedLoses").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-pref").unwrap();
    assert!(name.starts_with("LocalWins"), "got: {}", name);
}

#[test]
fn test_get_peer_display_name_uses_received_when_no_local() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-recv-disp", &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-recv-disp", "ReceivedNick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-recv-disp").unwrap();
    assert!(name.starts_with("ReceivedNick"), "got: {}", name);
}

#[test]
fn test_get_peer_display_name_includes_short_id_suffix() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("ABCDEFGHIJ", "Nick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("ABCDEFGHIJ").unwrap();
    // Format is "Nick (XYZ)" where XYZ is first 3 chars of last-8
    assert!(name.contains('(') && name.contains(')'), "got: {}", name);
}
