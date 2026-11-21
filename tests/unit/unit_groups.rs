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
                attachment: None,
                reply_to: None,
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
            attachment: None,
            reply_to: None,
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
fn incoming_group_message_without_msg_id_dedupes_on_content() {
    with_test_db(|| {
        let group = create_public_group("lobby").expect("create");
        record_group_member(&group.group_id, "peer-a").expect("join a");
        let meta = GroupMessageMeta {
            sender_nickname: Some("alice".to_string()),
            msg_id: None,
            sent_at: Some(122.0),
            attachment: None,
            reply_to: None,
        };
        let first = save_incoming_group_message(&group.group_id, "peer-a", "repeat", meta.clone())
            .expect("first")
            .expect("row returned");
        assert!(
            first
                .msg_id
                .as_deref()
                .is_some_and(|id| id.starts_with("nil:")),
            "a stable nil:-prefixed synthetic key stands in for the missing msg_id"
        );

        let dup = save_incoming_group_message(&group.group_id, "peer-a", "repeat", meta)
            .expect("dup query");
        assert!(
            dup.is_none(),
            "a NULL msg_id must still dedup on the synthetic content key"
        );

        let loaded = load_group_messages(&group.group_id, 100).expect("load");
        assert_eq!(loaded.len(), 1);
    });
}

#[test]
#[serial(db)]
fn mark_group_message_deleted_hides_only_sent_rows() {
    with_test_db(|| {
        let group = create_public_group("delete-lobby").expect("create");
        let sent = save_outgoing_group_message(
            &group.group_id,
            "mine",
            GroupMessageMeta {
                msg_id: Some("g-sent".to_string()),
                sent_at: Some(2.0),
                ..GroupMessageMeta::default()
            },
        )
        .expect("save outgoing");
        assert_eq!(sent.deleted_at, None, "new rows start visible");
        let incoming = save_incoming_group_message(
            &group.group_id,
            "peer-a",
            "theirs",
            GroupMessageMeta {
                msg_id: Some("g-in".to_string()),
                sent_at: Some(1.0),
                ..GroupMessageMeta::default()
            },
        )
        .expect("save incoming")
        .expect("row");

        assert!(
            !mark_group_message_deleted(incoming.id).expect("incoming not deletable"),
            "only messages we sent can be hidden"
        );
        assert!(mark_group_message_deleted(sent.id).expect("delete sent"));
        assert!(
            !mark_group_message_deleted(sent.id).expect("second delete is a no-op"),
            "tombstones are idempotent"
        );

        let conn = &mut crate::sqlite_connect().expect("connect");
        let stamped = crate::generated::schema::group_messages::table
            .filter(crate::generated::schema::group_messages::id.eq(sent.id))
            .select(crate::generated::schema::group_messages::deleted_at)
            .first::<Option<f64>>(conn)
            .expect("read tombstone");
        assert!(
            stamped.is_some_and(|at| at > 0.0),
            "deletion stamps a Unix timestamp: {stamped:?}"
        );

        let loaded = load_group_messages(&group.group_id, 100).expect("history");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, incoming.id, "only the incoming row remains");
    });
}

#[test]
#[serial(db)]
fn edit_group_message_replaces_only_own_live_rows_and_marks_them_edited() {
    with_test_db(|| {
        let group = create_public_group("edit-lobby").expect("create");
        let sent = save_outgoing_group_message(
            &group.group_id,
            "before",
            GroupMessageMeta {
                msg_id: Some("g-edit".to_string()),
                sent_at: Some(2.0),
                ..GroupMessageMeta::default()
            },
        )
        .expect("save outgoing");
        let incoming = save_incoming_group_message(
            &group.group_id,
            "peer-a",
            "theirs",
            GroupMessageMeta {
                msg_id: Some("g-in".to_string()),
                sent_at: Some(1.0),
                ..GroupMessageMeta::default()
            },
        )
        .expect("save incoming")
        .expect("row");

        assert!(
            !edit_group_message(incoming.id, "new body").expect("incoming not editable"),
            "incoming group rows are never edited"
        );
        assert!(edit_group_message(sent.id, " new ").expect("edit sent"));

        let conn = &mut crate::sqlite_connect().expect("connect");
        let (body, edited_at) = crate::generated::schema::group_messages::table
            .filter(crate::generated::schema::group_messages::id.eq(sent.id))
            .select((
                crate::generated::schema::group_messages::content,
                crate::generated::schema::group_messages::edited_at,
            ))
            .first::<(String, Option<f64>)>(conn)
            .expect("read edited group row");
        assert_eq!(body, " new ");
        assert!(edited_at.is_some_and(|at| at > 0.0));
        assert_eq!(
            group_message_body(sent.id).expect("raw body").as_deref(),
            Some(" new ")
        );
        assert_eq!(
            group_message_display_body(sent.id)
                .expect("rendered body")
                .as_deref(),
            Some(" new  (edited)")
        );
        assert_eq!(
            group_message_body(incoming.id).expect("incoming body"),
            None
        );

        assert!(edit_group_message(sent.id, "second").expect("repeat edit"));
        let loaded = load_group_messages(&group.group_id, 100).expect("history");
        let row = loaded.iter().find(|m| m.id == sent.id).expect("edited row");
        assert_eq!(row.content, "second", "repeat edits replace the body");

        mark_group_message_deleted(sent.id).expect("hide edited row");
        assert!(
            !edit_group_message(sent.id, "after delete").expect("deleted row not editable"),
            "tombstoned group rows cannot be edited"
        );
    });
}

#[test]
#[serial(db)]
fn group_tombstones_are_excluded_from_backfill_but_keep_the_watermark() {
    with_test_db(|| {
        let group = create_public_group("backfill-delete").expect("create");
        let mut ids = Vec::new();
        for (content, sent_at) in [("bf-old", 1.0), ("bf-hidden", 2.0), ("bf-newest", 3.0)] {
            let row = save_outgoing_group_message(
                &group.group_id,
                content,
                GroupMessageMeta {
                    msg_id: Some(content.to_string()),
                    sent_at: Some(sent_at),
                    ..GroupMessageMeta::default()
                },
            )
            .expect("save");
            ids.push(row.id);
        }
        mark_group_message_deleted(ids[1]).expect("hide middle");

        let after = load_group_messages_after(&group.group_id, 0.0, 100).expect("backfill");
        let msg_ids: Vec<&str> = after.iter().filter_map(|m| m.msg_id.as_deref()).collect();
        assert_eq!(
            msg_ids,
            ["bf-old", "bf-newest"],
            "a local tombstone must never be republished to peers"
        );

        // The watermark is unfiltered even when the newest row is hidden:
        // peers must not resend a row we deleted locally.
        mark_group_message_deleted(ids[2]).expect("hide newest");
        assert_eq!(
            latest_group_sent_at(&group.group_id).expect("watermark"),
            Some(3.0),
            "tombstones still hold the group watermark"
        );
        let after = load_group_messages_after(&group.group_id, 0.0, 100).expect("backfill again");
        let msg_ids: Vec<&str> = after.iter().filter_map(|m| m.msg_id.as_deref()).collect();
        assert_eq!(msg_ids, ["bf-old"]);
    });
}

#[test]
#[serial(db)]
fn ensure_group_for_invite_materializes_and_is_idempotent() {
    with_test_db(|| {
        let group = ensure_group_for_invite("abc123def4567890").expect("materialize");
        assert_eq!(group.group_id, "abc123def4567890");
        assert_eq!(
            group.display_name, "abc123def4567890",
            "display name falls back to the id"
        );
        assert_eq!(group.is_private, 0, "invited groups are treated as public");

        let again = ensure_group_for_invite("abc123def4567890").expect("idempotent");
        assert_eq!(again.id, group.id, "re-ensuring reuses the existing row");

        let existing = create_public_group("lobby").expect("create");
        let merged = ensure_group_for_invite(&existing.group_id).expect("existing");
        assert_eq!(merged.id, existing.id, "a group joined by name is reused");
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
                    attachment: None,
                    reply_to: None,
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

#[test]
#[serial(db)]
fn delete_group_removes_reactions_on_its_messages() {
    with_test_db(|| {
        let group = create_public_group("react-lobby").expect("create");
        let msg_id = "group-msg-with-reaction";
        save_outgoing_group_message(
            &group.group_id,
            "react to me",
            GroupMessageMeta {
                msg_id: Some(msg_id.to_string()),
                ..GroupMessageMeta::default()
            },
        )
        .expect("save");
        crate::reactions::apply(msg_id, "reactor-peer", "👍", false).expect("react");

        assert!(
            !crate::reactions::summaries_for(msg_id)
                .expect("summaries before")
                .is_empty(),
            "reaction exists before the delete"
        );

        delete_group(&group.group_id).expect("delete");

        assert!(
            crate::reactions::summaries_for(msg_id)
                .expect("summaries after")
                .is_empty(),
            "reactions must not outlive the group message they belong to"
        );
    });
}

// ── Backfill (§11.5) ──────────────────────────────────────────────────────

#[test]
fn backfill_topic_helpers_roundtrip() {
    let id = stable_group_id("raid-three");
    let topic = group_backfill_topic(&id);
    assert!(is_backfill_topic(&topic));
    assert!(topic.starts_with(&format!("{GROUP_TOPIC_PREFIX}{id}")));
    assert_eq!(group_id_from_backfill_topic(&topic), Some(id.as_str()));
    assert!(
        !is_backfill_topic(&group_topic(&id)),
        "chat topic is not backfill"
    );
    assert_eq!(
        group_id_from_backfill_topic("group:not-backfilled"),
        None,
        "a chat topic has no backfill group id"
    );
    // The routing hazard: a backfill topic also passes `is_group_topic`, so the
    // swarm handler must check `is_backfill_topic` FIRST.
    assert!(
        is_group_topic(&topic),
        "backfill topics overlap the group prefix; routing must disambiguate"
    );
}

#[test]
fn backfill_topic_predicate_requires_group_prefix() {
    // The predicate must agree with the extractor it guards: a foreign topic
    // that merely ends in `:backfill` must fall through to normal routing
    // instead of being sent to `group_id_from_backfill_topic` and rejected.
    assert!(!is_backfill_topic("notifications:backfill"));
    assert!(!is_backfill_topic(":backfill"));
    assert!(!is_backfill_topic("backfill"));
    assert!(is_backfill_topic("group:abc:backfill"));
    assert!(!is_backfill_topic("group:abc"));
}

#[test]
fn backfill_request_serializes_both_ways() {
    let req = crate::GroupBackfillRequest {
        group_id: "abc123".to_string(),
        after: 42.5,
    };
    let json = serde_json::to_string(&req).expect("serialize");
    let parsed: crate::GroupBackfillRequest = serde_json::from_str(&json).expect("parse");
    assert_eq!(parsed.group_id, "abc123");
    assert!(
        (parsed.after - 42.5).abs() < f64::EPSILON,
        "after round-trips exactly"
    );
    let defaults: crate::GroupBackfillRequest =
        serde_json::from_str(r#"{"group_id":"g"}"#).expect("defaults");
    assert!(
        defaults.after.abs() < f64::EPSILON,
        "missing `after` backfills the full gap"
    );
}

#[test]
#[serial(db)]
fn latest_group_sent_at_returns_newest_watermark_skipping_nulls() {
    use super::save_outgoing_group_message;
    with_test_db(|| {
        let group = create_public_group("watermark").expect("create");
        assert_eq!(
            latest_group_sent_at(&group.group_id).expect("empty watermark"),
            None,
            "no history yet"
        );
        for i in 1..=3 {
            save_outgoing_group_message(
                &group.group_id,
                &format!("msg-{i}"),
                GroupMessageMeta {
                    sender_nickname: None,
                    msg_id: Some(format!("wm-{i}")),
                    sent_at: Some(f64::from(i)),
                    attachment: None,
                    reply_to: None,
                },
            )
            .expect("save");
        }
        assert_eq!(
            latest_group_sent_at(&group.group_id).expect("watermark"),
            Some(3.0)
        );
        // Messages without a `sent_at` never become the watermark.
        let empty_group = create_public_group("nullwater").expect("create");
        save_outgoing_group_message(&empty_group.group_id, "legacy", GroupMessageMeta::default())
            .expect("save legacy");
        assert_eq!(
            latest_group_sent_at(&empty_group.group_id).expect("null watermark"),
            None
        );
    });
}

#[test]
#[serial(db)]
fn load_group_messages_after_is_strict_bounded_and_oldest_first() {
    use super::save_outgoing_group_message;
    with_test_db(|| {
        let group = create_public_group("backfill-gap").expect("create");
        for i in 0..5 {
            save_outgoing_group_message(
                &group.group_id,
                &format!("msg-{i}"),
                GroupMessageMeta {
                    sender_nickname: None,
                    msg_id: Some(format!("bf-{i}")),
                    sent_at: Some(f64::from(i)),
                    attachment: None,
                    reply_to: None,
                },
            )
            .expect("save");
        }
        assert!(
            load_group_messages_after(&group.group_id, 4.5, 100)
                .expect("no newer")
                .is_empty()
        );
        let after = load_group_messages_after(&group.group_id, 2.0, 100).expect("after");
        let ids: Vec<&str> = after.iter().filter_map(|m| m.msg_id.as_deref()).collect();
        assert_eq!(ids, ["bf-3", "bf-4"], "strictly newer, oldest first");
        let bounded = load_group_messages_after(&group.group_id, 0.0, 2).expect("bounded");
        let ids: Vec<&str> = bounded.iter().filter_map(|m| m.msg_id.as_deref()).collect();
        assert_eq!(ids, ["bf-1", "bf-2"], "strict `>` skips the zero watermark");
    });
}
