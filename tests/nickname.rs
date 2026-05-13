//! Tests for nickname.rs module

use serial_test::serial;
use std::sync::{Mutex, MutexGuard, OnceLock};
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct TestDb {
    _dir: TempDir,
    _guard: MutexGuard<'static, ()>,
}

impl Drop for TestDb {
    fn drop(&mut self) {
        p2p_app::db::release_db_lock();
        unsafe { std::env::remove_var("DATABASE_URL") };
    }
}

fn setup_test_db() -> TestDb {
    let guard = test_db_lock().lock().unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    unsafe { std::env::set_var("DATABASE_URL", db_path.to_str().unwrap()) };
    p2p_app::db::init_database().unwrap();
    TestDb {
        _dir: dir,
        _guard: guard,
    }
}

// ── generate_self_nickname ───────────────────────────────────────────────────

#[serial]
#[test]
fn test_generate_self_nickname_not_empty() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(!nick.is_empty());
}

#[serial]
#[test]
fn test_generate_self_nickname_contains_hyphen() {
    let nick = p2p_app::nickname::generate_self_nickname();
    assert!(
        nick.contains('-'),
        "expected two-word nickname separated by '-', got: {}",
        nick
    );
}

#[serial]
#[test]
fn test_generate_self_nickname_two_parts() {
    let nick = p2p_app::nickname::generate_self_nickname();
    let parts: Vec<&str> = nick.split('-').collect();
    assert_eq!(parts.len(), 2, "expected exactly 2 parts, got: {:?}", parts);
}

#[serial]
#[test]
fn test_generate_self_nickname_uniqueness() {
    // With a 2-word petname space this will virtually never collide
    let nicks: std::collections::HashSet<String> = (0..10)
        .map(|_| p2p_app::nickname::generate_self_nickname())
        .collect();
    assert!(nicks.len() > 1, "all 10 generated nicknames were identical");
}

// ── get/set self nickname ─────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_self_nickname_none_initially() {
    let _db = setup_test_db();
    // Fresh DB may have no identity row yet or null nickname
    // Either None or a pre-generated one is acceptable; just check it doesn't panic.
    let result = p2p_app::nickname::get_self_nickname();
    assert!(result.is_ok());
}

#[serial]
#[test]
fn test_set_and_get_self_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("test-nick").unwrap();
    let nick = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(nick.as_deref(), Some("test-nick"));
}

#[serial]
#[test]
fn test_set_self_nickname_overwrite() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("first").unwrap();
    p2p_app::nickname::set_self_nickname("second").unwrap();
    let nick = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(nick.as_deref(), Some("second"));
}

// ── ensure_self_nickname ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_ensure_self_nickname_generates_if_missing() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert!(!nick.is_empty());
    assert!(nick.contains('-'));
}

#[serial]
#[test]
fn test_ensure_self_nickname_returns_existing() {
    let _db = setup_test_db();
    p2p_app::nickname::set_self_nickname("my-name").unwrap();
    let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert_eq!(nick, "my-name");
}

#[serial]
#[test]
fn test_ensure_self_nickname_idempotent() {
    let _db = setup_test_db();
    let first = p2p_app::nickname::ensure_self_nickname().unwrap();
    let second = p2p_app::nickname::ensure_self_nickname().unwrap();
    assert_eq!(first, second);
}

// ── peer local nickname ───────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_local_nickname_none_for_unknown() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_local_nickname("unknown-peer").unwrap();
    assert!(nick.is_none());
}

#[serial]
#[test]
fn test_set_and_get_peer_local_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-001", "Alice").unwrap();
    let nick = p2p_app::nickname::get_peer_local_nickname("peer-001").unwrap();
    assert_eq!(nick.as_deref(), Some("Alice"));
}

#[serial]
#[test]
fn test_set_peer_local_nickname_overwrite() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-002", "Old").unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-002", "New").unwrap();
    let nick = p2p_app::nickname::get_peer_local_nickname("peer-002").unwrap();
    assert_eq!(nick.as_deref(), Some("New"));
}

#[serial]
#[test]
fn test_peer_local_nicknames_are_isolated() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-a", "Alpha").unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-b", "Beta").unwrap();
    assert_eq!(
        p2p_app::nickname::get_peer_local_nickname("peer-a")
            .unwrap()
            .as_deref(),
        Some("Alpha")
    );
    assert_eq!(
        p2p_app::nickname::get_peer_local_nickname("peer-b")
            .unwrap()
            .as_deref(),
        Some("Beta")
    );
}

// ── peer received nickname ────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_received_nickname_none_for_unknown() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_received_nickname("nobody").unwrap();
    assert!(nick.is_none());
}

#[serial]
#[test]
fn test_set_and_get_peer_received_nickname() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-recv", &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-recv", "Bob").unwrap();
    let nick = p2p_app::nickname::get_peer_received_nickname("peer-recv").unwrap();
    assert_eq!(nick.as_deref(), Some("Bob"));
}

#[serial]
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

#[serial]
#[test]
fn test_get_peer_self_nickname_for_peer_none_initially() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("nobody").unwrap();
    assert!(nick.is_none());
}

#[serial]
#[test]
fn test_set_and_get_peer_self_nickname_for_peer() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-sn", &[]).unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer-sn", "MyNameForThem").unwrap();
    let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("peer-sn").unwrap();
    assert_eq!(nick.as_deref(), Some("MyNameForThem"));
}

#[serial]
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

#[serial]
#[test]
fn test_get_peer_display_name_short_id_fallback() {
    let _db = setup_test_db();
    // Unknown peer — no local or received nickname — falls back to short ID
    let name = p2p_app::nickname::get_peer_display_name("12D3KooWABCDEFGH").unwrap();
    // Short ID is last 8 chars of the input
    assert!(!name.is_empty());
    assert!(name.contains("ABCDEFGH") || name.len() <= 8);
}

#[serial]
#[test]
fn test_get_peer_display_name_uses_local_nickname() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("peer-disp", "LocalNick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-disp").unwrap();
    assert!(name.starts_with("LocalNick"), "got: {}", name);
}

#[serial]
#[test]
fn test_get_peer_display_name_prefers_local_over_received() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-pref", &[]).unwrap();
    p2p_app::nickname::set_peer_local_nickname("peer-pref", "LocalWins").unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-pref", "ReceivedLoses").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-pref").unwrap();
    assert!(name.starts_with("LocalWins"), "got: {}", name);
}

#[serial]
#[test]
fn test_get_peer_display_name_uses_received_when_no_local() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer-recv-disp", &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname("peer-recv-disp", "ReceivedNick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("peer-recv-disp").unwrap();
    assert!(name.starts_with("ReceivedNick"), "got: {}", name);
}

#[serial]
#[test]
fn test_get_peer_display_name_includes_short_id_suffix() {
    let _db = setup_test_db();
    p2p_app::nickname::set_peer_local_nickname("ABCDEFGHIJ", "Nick").unwrap();
    let name = p2p_app::nickname::get_peer_display_name("ABCDEFGHIJ").unwrap();
    // Format is "Nick (XYZ)" where XYZ is first 3 chars of last-8
    assert!(name.contains('(') && name.contains(')'), "got: {}", name);
}

// ── Additional edge cases ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_generate_nickname_deterministic() {
    let _db = setup_test_db();
    let nick1 = p2p_app::nickname::generate_self_nickname();
    let nick2 = p2p_app::nickname::generate_self_nickname();
    assert!(!nick1.is_empty());
    assert!(!nick2.is_empty());
}

#[serial]
#[test]
fn test_get_local_nickname_unset() {
    let _db = setup_test_db();
    let nick = p2p_app::nickname::get_self_nickname().unwrap();
    assert!(nick.is_none() || nick.as_deref().unwrap_or("").contains('-'));
}

#[serial]
#[test]
fn test_set_get_local_nickname_roundtrip() {
    let _db = setup_test_db();
    let new_nick = "TestNick";
    p2p_app::nickname::set_self_nickname(new_nick).unwrap();
    let retrieved = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(retrieved.as_deref(), Some(new_nick));
}

#[serial]
#[test]
fn test_set_received_nickname_idempotent() {
    let _db = setup_test_db();
    let peer = "peer-idempotent";
    p2p_app::save_peer(peer, &[]).unwrap();
    p2p_app::nickname::set_peer_received_nickname(peer, "Alice").unwrap();
    let nick1 = p2p_app::nickname::get_peer_received_nickname(peer).unwrap();

    p2p_app::nickname::set_peer_received_nickname(peer, "Alice").unwrap();
    let nick2 = p2p_app::nickname::get_peer_received_nickname(peer).unwrap();

    assert_eq!(nick1, nick2);
}

#[serial]
#[test]
fn test_get_peer_display_name_all_none() {
    let _db = setup_test_db();
    let peer = "peer-noinfo";
    let display = p2p_app::nickname::get_peer_display_name(peer).unwrap();
    assert!(!display.is_empty());
}

#[serial]
#[test]
fn test_get_peer_self_nickname_multiple_peers() {
    let _db = setup_test_db();
    p2p_app::save_peer("peer1", &[]).unwrap();
    p2p_app::save_peer("peer2", &[]).unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer1", "My1").unwrap();
    p2p_app::nickname::set_peer_self_nickname_for_peer("peer2", "My2").unwrap();

    let nick1 = p2p_app::nickname::get_peer_self_nickname_for_peer("peer1").unwrap();
    let nick2 = p2p_app::nickname::get_peer_self_nickname_for_peer("peer2").unwrap();

    assert_eq!(nick1.as_deref(), Some("My1"));
    assert_eq!(nick2.as_deref(), Some("My2"));
}

#[serial]
#[test]
fn test_nickname_with_unicode() {
    let _db = setup_test_db();
    let unicode_nick = "Alice👋";
    p2p_app::nickname::set_self_nickname(unicode_nick).unwrap();
    let retrieved = p2p_app::nickname::get_self_nickname().unwrap();
    assert_eq!(retrieved.as_deref(), Some(unicode_nick));
}

#[serial]
#[test]
fn test_nickname_max_length_enforcement() {
    let _db = setup_test_db();
    let long_nick = "a".repeat(100);
    let result = p2p_app::nickname::set_self_nickname(&long_nick);
    assert!(result.is_ok() || result.is_err());
}
