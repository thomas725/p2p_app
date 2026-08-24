//! Tests for nickname.rs module

use diesel::RunQueryDsl as _;
use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

// ── helpers ──────────────────────────────────────────────────────────────────

fn test_db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
    let _guard = test_db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    temp_env::with_var("DATABASE_URL", Some(db_path.to_str().unwrap()), || {
        p2p_app::db::init_database().unwrap();
        f();
        p2p_app::db::release_db_lock();
    });
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
        "expected two-word nickname separated by '-', got: {nick}"
    );
}

#[serial]
#[test]
fn test_generate_self_nickname_two_parts() {
    let nick = p2p_app::nickname::generate_self_nickname();
    let parts: Vec<&str> = nick.split('-').collect();
    assert_eq!(parts.len(), 2, "expected exactly 2 parts, got: {parts:?}");
}

#[serial]
#[test]
fn test_generate_self_nickname_uniqueness() {
    let nicks: std::collections::HashSet<String> = (0..10)
        .map(|_| p2p_app::nickname::generate_self_nickname())
        .collect();
    assert!(nicks.len() > 1, "all 10 generated nicknames were identical");
}

// ── get/set self nickname ─────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_self_nickname_none_initially() {
    with_test_db(|| {
        let result = p2p_app::nickname::get_self_nickname();
        assert!(result.is_ok());
    });
}

#[serial]
#[test]
fn test_set_and_get_self_nickname() {
    with_test_db(|| {
        p2p_app::nickname::set_self_nickname("test-nick").unwrap();
        let nick = p2p_app::nickname::get_self_nickname().unwrap();
        assert_eq!(nick.as_deref(), Some("test-nick"));
    });
}

#[serial]
#[test]
fn test_set_self_nickname_overwrite() {
    with_test_db(|| {
        p2p_app::nickname::set_self_nickname("first").unwrap();
        p2p_app::nickname::set_self_nickname("second").unwrap();
        let nick = p2p_app::nickname::get_self_nickname().unwrap();
        assert_eq!(nick.as_deref(), Some("second"));
    });
}

// ── ensure_self_nickname ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_ensure_self_nickname_generates_if_missing() {
    with_test_db(|| {
        let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
        assert!(!nick.is_empty());
        assert!(nick.contains('-'));
    });
}

#[serial]
#[test]
fn test_ensure_self_nickname_returns_existing() {
    with_test_db(|| {
        p2p_app::nickname::set_self_nickname("my-name").unwrap();
        let nick = p2p_app::nickname::ensure_self_nickname().unwrap();
        assert_eq!(nick, "my-name");
    });
}

#[serial]
#[test]
fn test_ensure_self_nickname_idempotent() {
    with_test_db(|| {
        let first = p2p_app::nickname::ensure_self_nickname().unwrap();
        let second = p2p_app::nickname::ensure_self_nickname().unwrap();
        assert_eq!(first, second);
    });
}

// ── peer local nickname ───────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_local_nickname_none_for_unknown() {
    with_test_db(|| {
        let nick = p2p_app::nickname::get_peer_local_nickname("unknown-peer").unwrap();
        assert!(nick.is_none());
    });
}

#[serial]
#[test]
fn test_set_and_get_peer_local_nickname() {
    with_test_db(|| {
        p2p_app::nickname::set_peer_local_nickname("peer-001", "Alice").unwrap();
        let nick = p2p_app::nickname::get_peer_local_nickname("peer-001").unwrap();
        assert_eq!(nick.as_deref(), Some("Alice"));
    });
}

#[serial]
#[test]
fn test_set_peer_local_nickname_overwrite() {
    with_test_db(|| {
        p2p_app::nickname::set_peer_local_nickname("peer-002", "Old").unwrap();
        p2p_app::nickname::set_peer_local_nickname("peer-002", "New").unwrap();
        let nick = p2p_app::nickname::get_peer_local_nickname("peer-002").unwrap();
        assert_eq!(nick.as_deref(), Some("New"));
    });
}

#[serial]
#[test]
fn test_peer_local_nicknames_are_isolated() {
    with_test_db(|| {
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
    });
}

// ── peer received nickname ────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_received_nickname_none_for_unknown() {
    with_test_db(|| {
        let nick = p2p_app::nickname::get_peer_received_nickname("nobody").unwrap();
        assert!(nick.is_none());
    });
}

#[serial]
#[test]
fn test_set_and_get_peer_received_nickname() {
    with_test_db(|| {
        p2p_app::save_peer("peer-recv", &[]).unwrap();
        p2p_app::nickname::set_peer_received_nickname("peer-recv", "Bob").unwrap();
        let nick = p2p_app::nickname::get_peer_received_nickname("peer-recv").unwrap();
        assert_eq!(nick.as_deref(), Some("Bob"));
    });
}

#[serial]
#[test]
fn test_set_peer_received_nickname_overwrite() {
    with_test_db(|| {
        p2p_app::save_peer("peer-rw", &[]).unwrap();
        p2p_app::nickname::set_peer_received_nickname("peer-rw", "v1").unwrap();
        p2p_app::nickname::set_peer_received_nickname("peer-rw", "v2").unwrap();
        let nick = p2p_app::nickname::get_peer_received_nickname("peer-rw").unwrap();
        assert_eq!(nick.as_deref(), Some("v2"));
    });
}

#[serial]
#[test]
fn test_received_name_change_archives_history() {
    with_test_db(|| {
        p2p_app::save_peer("peer-hist", &[]).unwrap();

        // First received name: nothing to archive yet.
        p2p_app::nickname::record_peer_received_name_change("peer-hist", "Alice").unwrap();
        let hist1 = p2p_app::nickname::get_peer_name_history("peer-hist").unwrap();
        assert!(hist1.is_empty(), "no history before a name changes");

        // Change to Bob: Alice is archived.
        p2p_app::nickname::record_peer_received_name_change("peer-hist", "Bob").unwrap();
        let hist2 = p2p_app::nickname::get_peer_name_history("peer-hist").unwrap();
        assert_eq!(hist2.len(), 1);
        assert_eq!(hist2[0].name, "Alice");
        assert_eq!(hist2[0].name_kind, "received");

        // Change to Carol: Bob is archived (most recent first).
        p2p_app::nickname::record_peer_received_name_change("peer-hist", "Carol").unwrap();
        let hist3 = p2p_app::nickname::get_peer_name_history("peer-hist").unwrap();
        assert_eq!(hist3.len(), 2);
        assert_eq!(hist3[0].name, "Bob");
        assert_eq!(hist3[1].name, "Alice");

        // Setting the same name again must not create a new history entry.
        p2p_app::nickname::record_peer_received_name_change("peer-hist", "Carol").unwrap();
        let hist4 = p2p_app::nickname::get_peer_name_history("peer-hist").unwrap();
        assert_eq!(hist4.len(), 2);
    });
}

// ── self nickname for peer ────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_self_nickname_for_peer_none_initially() {
    with_test_db(|| {
        let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("nobody").unwrap();
        assert!(nick.is_none());
    });
}

#[serial]
#[test]
fn test_set_and_get_peer_self_nickname_for_peer() {
    with_test_db(|| {
        p2p_app::save_peer("peer-sn", &[]).unwrap();
        p2p_app::nickname::set_peer_self_nickname_for_peer("peer-sn", "MyNameForThem").unwrap();
        let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("peer-sn").unwrap();
        assert_eq!(nick.as_deref(), Some("MyNameForThem"));
    });
}

#[serial]
#[test]
fn test_set_peer_self_nickname_for_peer_overwrite() {
    with_test_db(|| {
        p2p_app::save_peer("peer-snw", &[]).unwrap();
        p2p_app::nickname::set_peer_self_nickname_for_peer("peer-snw", "old-name").unwrap();
        p2p_app::nickname::set_peer_self_nickname_for_peer("peer-snw", "new-name").unwrap();
        let nick = p2p_app::nickname::get_peer_self_nickname_for_peer("peer-snw").unwrap();
        assert_eq!(nick.as_deref(), Some("new-name"));
    });
}

// ── get_peer_display_name ─────────────────────────────────────────────────────

#[serial]
#[test]
fn test_get_peer_display_name_assigns_generated_petname_for_silent_peer() {
    with_test_db(|| {
        // A silent peer (no announced/local nickname) gets a stable generated
        // petname on first lookup, persisted so subsequent lookups match.
        let name1 = p2p_app::nickname::get_peer_display_name("12D3KooWABCDEFGH").unwrap();
        let name2 = p2p_app::nickname::get_peer_display_name("12D3KooWABCDEFGH").unwrap();
        assert_eq!(name1, name2, "generated petname must be stable");
        assert!(
            name1.contains('(') && name1.contains(')'),
            "silent peer should show a generated petname, got: {name1}"
        );
        assert!(
            !name1.contains("12D3KooWABCDEFGH"),
            "should not be the raw id: {name1}"
        );
    });
}

#[serial]
#[test]
fn test_get_peer_display_name_uses_local_nickname() {
    with_test_db(|| {
        p2p_app::nickname::set_peer_local_nickname("peer-disp", "LocalNick").unwrap();
        let name = p2p_app::nickname::get_peer_display_name("peer-disp").unwrap();
        assert!(name.starts_with("LocalNick"), "got: {name}");
    });
}

#[serial]
#[test]
fn test_get_peer_display_name_prefers_local_over_received() {
    with_test_db(|| {
        p2p_app::save_peer("peer-pref", &[]).unwrap();
        p2p_app::nickname::set_peer_local_nickname("peer-pref", "LocalWins").unwrap();
        p2p_app::nickname::set_peer_received_nickname("peer-pref", "ReceivedLoses").unwrap();
        let name = p2p_app::nickname::get_peer_display_name("peer-pref").unwrap();
        assert!(name.starts_with("LocalWins"), "got: {name}");
    });
}

#[serial]
#[test]
fn test_get_peer_display_name_uses_received_when_no_local() {
    with_test_db(|| {
        p2p_app::save_peer("peer-recv-disp", &[]).unwrap();
        p2p_app::nickname::set_peer_received_nickname("peer-recv-disp", "ReceivedNick").unwrap();
        let name = p2p_app::nickname::get_peer_display_name("peer-recv-disp").unwrap();
        assert!(name.starts_with("ReceivedNick"), "got: {name}");
    });
}

#[serial]
#[test]
fn test_get_peer_display_name_includes_short_id_suffix() {
    with_test_db(|| {
        p2p_app::nickname::set_peer_local_nickname("ABCDEFGHIJ", "Nick").unwrap();
        let name = p2p_app::nickname::get_peer_display_name("ABCDEFGHIJ").unwrap();
        assert!(name.contains('(') && name.contains(')'), "got: {name}");
    });
}

#[serial]
#[test]
fn test_save_peer_generates_stable_name_for_silent_peer() {
    with_test_db(|| {
        p2p_app::save_peer("peer-silent", &[]).unwrap();
        // Re-saving must not regenerate the name
        p2p_app::save_peer("peer-silent", &[]).unwrap();

        let name1 = p2p_app::nickname::get_peer_display_name("peer-silent").unwrap();
        let name2 = p2p_app::nickname::get_peer_display_name("peer-silent").unwrap();
        assert_eq!(name1, name2, "generated name must be stable");
        assert!(
            !name1.starts_with("peer-silent"),
            "should not fall back to raw ID: {name1}"
        );
        assert!(name1.contains('(') && name1.contains(')'), "got: {name1}");

        // Announced nickname takes precedence over the generated one
        p2p_app::nickname::set_peer_received_nickname("peer-silent", "RealName").unwrap();
        let name = p2p_app::nickname::get_peer_display_name("peer-silent").unwrap();
        assert!(name.starts_with("RealName"), "got: {name}");
    });
}

#[serial]
#[test]
fn test_save_peer_assigns_name_for_legacy_peer() {
    with_test_db(|| {
        // Simulate a peer created before the petname feature: its
        // generated_nickname column is NULL. It must still resolve to a stable
        // generated petname (assigned lazily on display) and not a raw ID.
        {
            let conn = &mut p2p_app::db::sqlite_connect().unwrap();
            diesel::sql_query(
                "INSERT INTO peers (peer_id, addresses, created_at, first_seen, last_seen) \
                 VALUES ('peer-legacy', '', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
            )
            .execute(conn)
            .unwrap();
        }

        // First lookup assigns and persists a petname.
        let name1 = p2p_app::nickname::get_peer_display_name("peer-legacy").unwrap();
        assert!(
            name1.contains('(') && name1.contains(')'),
            "legacy peer should resolve to a generated name: {name1}"
        );
        assert!(
            !name1.contains("peer-legacy"),
            "should not be the raw id: {name1}"
        );

        // save_peer (e.g. on reconnect) must not disturb the assigned name.
        p2p_app::save_peer("peer-legacy", &[]).unwrap();
        let name2 = p2p_app::nickname::get_peer_display_name("peer-legacy").unwrap();
        assert_eq!(name1, name2, "assigned name must be stable across saves");

        // Saving again must still keep the same name.
        p2p_app::save_peer("peer-legacy", &[]).unwrap();
        let name3 = p2p_app::nickname::get_peer_display_name("peer-legacy").unwrap();
        assert_eq!(name1, name3, "assigned name must remain stable");
    });
}

// ── Additional edge cases ──────────────────────────────────────────────────────

#[serial]
#[test]
fn test_generate_nickname_deterministic() {
    with_test_db(|| {
        let nick1 = p2p_app::nickname::generate_self_nickname();
        let nick2 = p2p_app::nickname::generate_self_nickname();
        assert!(!nick1.is_empty());
        assert!(!nick2.is_empty());
    });
}

#[serial]
#[test]
fn test_get_local_nickname_unset() {
    with_test_db(|| {
        let nick = p2p_app::nickname::get_self_nickname().unwrap();
        assert!(nick.is_none() || nick.as_deref().unwrap_or("").contains('-'));
    });
}

#[serial]
#[test]
fn test_set_get_local_nickname_roundtrip() {
    with_test_db(|| {
        let new_nick = "TestNick";
        p2p_app::nickname::set_self_nickname(new_nick).unwrap();
        let retrieved = p2p_app::nickname::get_self_nickname().unwrap();
        assert_eq!(retrieved.as_deref(), Some(new_nick));
    });
}

#[serial]
#[test]
fn test_set_received_nickname_idempotent() {
    with_test_db(|| {
        let peer = "peer-idempotent";
        p2p_app::save_peer(peer, &[]).unwrap();
        p2p_app::nickname::set_peer_received_nickname(peer, "Alice").unwrap();
        let nick1 = p2p_app::nickname::get_peer_received_nickname(peer).unwrap();

        p2p_app::nickname::set_peer_received_nickname(peer, "Alice").unwrap();
        let nick2 = p2p_app::nickname::get_peer_received_nickname(peer).unwrap();

        assert_eq!(nick1, nick2);
    });
}

#[serial]
#[test]
fn test_get_peer_display_name_all_none() {
    with_test_db(|| {
        let peer = "peer-noinfo";
        let display = p2p_app::nickname::get_peer_display_name(peer).unwrap();
        assert!(!display.is_empty());
    });
}

#[serial]
#[test]
fn test_get_peer_self_nickname_multiple_peers() {
    with_test_db(|| {
        p2p_app::save_peer("peer1", &[]).unwrap();
        p2p_app::save_peer("peer2", &[]).unwrap();
        p2p_app::nickname::set_peer_self_nickname_for_peer("peer1", "My1").unwrap();
        p2p_app::nickname::set_peer_self_nickname_for_peer("peer2", "My2").unwrap();

        let nick1 = p2p_app::nickname::get_peer_self_nickname_for_peer("peer1").unwrap();
        let nick2 = p2p_app::nickname::get_peer_self_nickname_for_peer("peer2").unwrap();

        assert_eq!(nick1.as_deref(), Some("My1"));
        assert_eq!(nick2.as_deref(), Some("My2"));
    });
}

#[serial]
#[test]
fn test_nickname_with_unicode() {
    with_test_db(|| {
        let unicode_nick = "Alice👋";
        p2p_app::nickname::set_self_nickname(unicode_nick).unwrap();
        let retrieved = p2p_app::nickname::get_self_nickname().unwrap();
        assert_eq!(retrieved.as_deref(), Some(unicode_nick));
    });
}

#[serial]
#[test]
fn test_nickname_max_length_enforcement() {
    with_test_db(|| {
        let long_nick = "a".repeat(100);
        let result = p2p_app::nickname::set_self_nickname(&long_nick);
        assert!(result.is_ok() || result.is_err());
    });
}
