use super::*;
use serial_test::serial;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn db_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn with_test_db(f: impl FnOnce()) {
    let _guard = db_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("test.db");
    crate::db::set_db_url(db_path.to_str().expect("db path"));
    crate::db::init_database().expect("init db");
    f();
    crate::db::release_db_lock();
    crate::db::reset_db_url();
}

// ── Identity ───────────────────────────────────────────────────────────────

#[test]
fn canonical_group_name_collapses_and_lowercases() {
    assert_eq!(
        canonical_group_name("  Rust  Enthusiasts "),
        "rust enthusiasts"
    );
    assert_eq!(canonical_group_name("Single"), "single");
}

#[test]
fn stable_group_id_is_deterministic_hex() {
    let a = stable_group_id("Rust Enthusiasts");
    let b = stable_group_id("rust   enthusiasts");
    assert_eq!(a, b, "same canonical name must map to the same group id");
    assert_eq!(a.len(), 16, "64-bit hash rendered as 16 hex chars");
    assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    assert_ne!(stable_group_id("one"), stable_group_id("two"));
}

#[test]
fn group_topic_helpers_roundtrip() {
    let id = stable_group_id("squiggles");
    let topic = group_topic(&id);
    assert!(is_group_topic(&topic));
    assert_eq!(group_id_from_topic(&topic), Some(id.as_str()));
    assert!(!is_group_topic("map-data-v1"));
    assert_eq!(group_id_from_topic("map-data-v1"), None);
}

// ── CRUD ───────────────────────────────────────────────────────────────────

#[test]
#[serial(db)]
fn create_public_group_is_idempotent_across_spelling() {
    with_test_db(|| {
        let first = create_public_group("  Rust Enthusiasts ").expect("create");
        assert_eq!(first.display_name, "Rust Enthusiasts");
        let second = create_public_group("rust enthusiasts").expect("create again");
        assert_eq!(first.id, second.id, "same canonical name reuses the group");
        assert_eq!(
            find_group(&first.group_id).expect("find").unwrap().id,
            first.id
        );
    });
}

#[test]
#[serial(db)]
fn list_groups_and_delete() {
    with_test_db(|| {
        let a = create_public_group("alpha").expect("alpha");
        let b = create_public_group("beta").expect("beta");
        let groups = list_groups().expect("list");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, a.id, "oldest group first");

        delete_group(&b.group_id).expect("delete beta");
        let remaining = list_groups().expect("list after delete");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].group_id, a.group_id);
    });
}

// ── Members ────────────────────────────────────────────────────────────────

#[test]
#[serial(db)]
fn record_group_member_is_idempotent_and_counts() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        record_group_member(&group.group_id, "peer-a").expect("join a");
        record_group_member(&group.group_id, "peer-a").expect("join a again");
        record_group_member(&group.group_id, "peer-b").expect("join b");
        assert_eq!(get_group_member_count(&group.group_id).expect("count"), 2);

        let counts = get_all_group_member_counts().expect("all counts");
        assert_eq!(counts.get(&group.group_id), Some(&2));
    });
}

#[test]
#[serial(db)]
fn list_groups_with_member_counts_reports_members() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        record_group_member(&group.group_id, "peer-a").expect("join");
        let summaries = list_groups_with_member_counts().expect("summaries");
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].member_count, 1);
    });
}

// ── Messages ───────────────────────────────────────────────────────────────

#[test]
#[serial(db)]
fn outgoing_group_message_has_no_peer_and_is_sent() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        let saved = save_outgoing_group_message(
            &group.group_id,
            "hello",
            GroupMessageMeta {
                sender_nickname: Some("me".to_string()),
                msg_id: Some("out-1".to_string()),
                sent_at: Some(123.0),
            },
        )
        .expect("save outgoing");
        assert_eq!(saved.peer_id, None);
        assert_eq!(saved.sent, 1);
        assert_eq!(saved.msg_id.as_deref(), Some("out-1"));

        let loaded = load_group_messages(&group.group_id, 100)
            .expect("load")
            .first()
            .cloned()
            .unwrap();
        assert_eq!(loaded.content, "hello");
        assert_eq!(loaded.sender_nickname.as_deref(), Some("me"));
    });
}

#[test]
#[serial(db)]
fn incoming_group_message_dedupes_on_msg_id() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        record_group_member(&group.group_id, "peer-a").expect("join a");
        let meta = GroupMessageMeta {
            sender_nickname: Some("alice".to_string()),
            msg_id: Some("in-1".to_string()),
            sent_at: Some(122.0),
        };
        let first = save_incoming_group_message(&group.group_id, "peer-a", "hey", meta.clone())
            .expect("first")
            .expect("row returned");
        assert_eq!(first.peer_id.as_deref(), Some("peer-a"));
        assert_eq!(first.sent, 0);

        let dup =
            save_incoming_group_message(&group.group_id, "peer-a", "hey", meta).expect("dup query");
        assert!(
            dup.is_none(),
            "identical (group_id, msg_id) must be dropped"
        );

        let loaded = load_group_messages(&group.group_id, 100).expect("load");
        assert_eq!(loaded.len(), 1);
    });
}

#[test]
#[serial(db)]
fn load_group_messages_orders_newest_first_and_respects_limit() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        for i in 0..5 {
            save_outgoing_group_message(
                &group.group_id,
                &format!("msg-{i}"),
                GroupMessageMeta {
                    sender_nickname: None,
                    msg_id: Some(format!("out-{i}")),
                    sent_at: Some(f64::from(i)),
                },
            )
            .expect("save");
        }
        let limited = load_group_messages(&group.group_id, 2).expect("load limited");
        assert_eq!(limited.len(), 2);
        assert_eq!(limited[0].msg_id.as_deref(), Some("out-4"), "newest first");
        assert_eq!(limited[1].msg_id.as_deref(), Some("out-3"));
    });
}

#[test]
#[serial(db)]
fn delete_group_removes_messages_and_members() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        record_group_member(&group.group_id, "peer-a").expect("join");
        save_outgoing_group_message(&group.group_id, "bye", GroupMessageMeta::default())
            .expect("save");
        delete_group(&group.group_id).expect("delete");

        assert!(find_group(&group.group_id).expect("find").is_none());
        assert_eq!(get_group_member_count(&group.group_id).expect("count"), 0);
        assert!(
            load_group_messages(&group.group_id, 100)
                .expect("load")
                .is_empty(),
            "messages cascade with the group"
        );
    });
}
