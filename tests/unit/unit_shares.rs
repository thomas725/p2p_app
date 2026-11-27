#![allow(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction,
    clippy::used_underscore_binding,
    clippy::significant_drop_tightening
)]

use super::*;
use crate::generated::models_insertable::{NewDeviceBinding, NewPeer, NewUser};
use crate::generated::schema::{device_bindings, peers, share_grants, users};
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

/// Install a verified-looking device binding directly. ACL resolution only
/// reads this row; the signature check belongs to the identity lane's tests.
fn bind_peer(peer_id: &str, user_id: &str) {
    use diesel::QueryDsl as _;
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect().expect("db connection");
    let now = chrono::Utc::now().naive_utc();
    diesel::insert_into(users::table)
        .values(NewUser {
            user_id: user_id.to_string(),
            account_pubkey: vec![1_u8; 32],
            self_username: None,
            self_nickname: None,
        })
        .on_conflict_do_nothing()
        .execute(conn)
        .expect("insert user");
    diesel::insert_into(peers::table)
        .values(NewPeer {
            peer_id: peer_id.to_string(),
            addresses: String::new(),
            first_seen: now,
            last_seen: now,
            peer_local_nickname: None,
            self_nickname_for_peer: None,
            received_nickname: None,
            generated_nickname: None,
        })
        .on_conflict_do_nothing()
        .execute(conn)
        .expect("insert peer");
    let exists: i64 = device_bindings::table
        .filter(device_bindings::peer_id.eq(peer_id))
        .count()
        .get_result(conn)
        .expect("count bindings");
    if exists == 0 {
        diesel::insert_into(device_bindings::table)
            .values(NewDeviceBinding {
                claim_id: format!("{peer_id}:{user_id}"),
                user_id: user_id.to_string(),
                peer_id: peer_id.to_string(),
                device_pubkey: vec![2_u8; 32],
                claim: vec![3_u8; 64],
            })
            .execute(conn)
            .expect("insert binding");
    }
}

fn entry(path: &str, file_id: String) -> ShareEntry {
    ShareEntry {
        path: path.to_string(),
        file_id,
        size: 3,
        mime: None,
    }
}

#[test]
fn validate_share_name_trims_and_rejects_bad_input() {
    assert_eq!(validate_share_name("  photos  ").expect("trim"), "photos");
    assert!(validate_share_name("   ").is_err());
    assert!(validate_share_name("line\nbreak").is_err());
    let too_long = "x".repeat(MAX_SHARE_NAME_LEN + 1);
    assert!(validate_share_name(&too_long).is_err());
}

#[test]
fn validate_share_path_normalises_and_rejects_traversal() {
    assert_eq!(
        validate_share_path("docs/readme.md").expect("valid"),
        "docs/readme.md"
    );
    assert_eq!(
        validate_share_path("docs/readme.md/").expect("trailing slash"),
        "docs/readme.md"
    );
    for bad in [
        "",
        "/etc/passwd",
        "../secret",
        "docs/../../secret",
        "docs/./secret",
        "docs//secret",
        "docs\\secret",
        "C:/secret",
        "docs/\0secret",
        "docs/\u{0001}secret",
    ] {
        assert!(validate_share_path(bad).is_err(), "accepted {bad:?}");
    }
}

#[test]
fn validate_manifest_checks_ids_paths_sizes_and_duplicates() {
    let id = crate::files::hash_bytes(b"abc");
    let manifest = ShareManifest {
        share_id: "s1".to_string(),
        name: "share".to_string(),
        revision: 1,
        entries: vec![entry("a.txt", id.clone())],
    };
    assert!(validate_manifest(&manifest).is_ok());

    let duplicate = ShareManifest {
        entries: vec![entry("a.txt", id.clone()), entry("a.txt", id.clone())],
        ..manifest.clone()
    };
    assert!(validate_manifest(&duplicate).is_err());

    let bad_id = ShareManifest {
        entries: vec![entry("a.txt", "../../etc/passwd".to_string())],
        ..manifest.clone()
    };
    assert!(validate_manifest(&bad_id).is_err());

    let bad_path = ShareManifest {
        entries: vec![entry("../escape", id)],
        ..manifest.clone()
    };
    assert!(validate_manifest(&bad_path).is_err());

    let too_big = ShareManifest {
        entries: vec![ShareEntry {
            size: crate::files::MAX_ATTACHMENT_SIZE.saturating_add(1),
            ..entry("big.bin", crate::files::hash_bytes(b"x"))
        }],
        ..manifest
    };
    assert!(validate_manifest(&too_big).is_err());
}

#[test]
fn manifest_json_roundtrip() {
    let manifest = ShareManifest {
        share_id: "s1".to_string(),
        name: "share".to_string(),
        revision: 7,
        entries: vec![entry("a.txt", crate::files::hash_bytes(b"abc"))],
    };
    let json = manifest_to_json(&manifest).expect("serialize");
    assert_eq!(manifest_from_json(&json), Some(manifest));
}

#[test]
fn wire_requests_and_responses_decode_with_defaults() {
    let request: crate::behavior::ShareRequest =
        serde_json::from_str(r#"{"kind":"list"}"#).expect("decode request");
    assert_eq!(request.kind, "list");
    assert_eq!(request.base_revision, 0);
    assert!(request.manifest.is_none());
    assert!(request.offer.is_none());

    let response: crate::behavior::ShareResponse =
        serde_json::from_str(r#"{"ok":true,"kind":"list"}"#).expect("decode response");
    assert!(response.ok);
    assert!(response.offers.is_empty());
    assert!(response.manifest.is_none());
}

#[test]
#[serial(db)]
fn create_list_and_delete_share() {
    with_test_db(|| {
        let share = create_share("Photos", "owner-user", ShareKind::Private, false, None)
            .expect("create share");
        assert_eq!(share.name, "Photos");
        assert_eq!(share.kind, ShareKind::Private);
        assert_eq!(share.revision, 0);
        assert!(
            list_shares()
                .expect("list")
                .iter()
                .any(|s| s.share_id == share.share_id)
        );

        let root = "/tmp/not-used";
        set_local_root(&share.share_id, Some(root)).expect("bind root");
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .local_root
                .as_deref(),
            Some(root)
        );

        delete_share(&share.share_id).expect("delete");
        assert!(get_share(&share.share_id).expect("get").is_none());
    });
}

#[test]
#[serial(db)]
fn bulk_grant_listing_groups_by_share() {
    with_test_db(|| {
        let first = create_share("First", "owner", ShareKind::Private, false, None)
            .expect("create first share");
        let second = create_share("Second", "owner", ShareKind::Private, false, None)
            .expect("create second share");
        set_grant(&first.share_id, "alice", ShareMode::ReadOnly).expect("alice grant");
        set_grant(&first.share_id, "bob", ShareMode::ReadWrite).expect("bob grant");
        set_grant(&second.share_id, "carol", ShareMode::ReadOnly).expect("carol grant");

        let all = list_all_grants().expect("bulk grants");
        assert_eq!(all.len(), 2);
        let first_grants = all.get(&first.share_id).expect("first grants");
        assert_eq!(first_grants.len(), 2);
        assert_eq!(
            first_grants
                .iter()
                .find(|g| g.user_id == "alice")
                .map(|g| g.mode),
            Some(ShareMode::ReadOnly)
        );
        assert_eq!(
            first_grants
                .iter()
                .find(|g| g.user_id == "bob")
                .map(|g| g.mode),
            Some(ShareMode::ReadWrite)
        );
        let second_grants = all.get(&second.share_id).expect("second grants");
        assert_eq!(second_grants.len(), 1);
        assert_eq!(second_grants[0].user_id, "carol");
    });
}

#[test]
#[serial(db)]
fn grants_are_upserted_and_listed() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        bind_peer("peer-alice-1", "alice");
        set_grant(&share.share_id, "alice", ShareMode::ReadOnly).expect("read grant");
        set_grant(&share.share_id, "alice", ShareMode::ReadWrite).expect("write grant");
        set_grant(&share.share_id, "bob", ShareMode::ReadOnly).expect("bob grant");
        let grants = list_grants(&share.share_id).expect("list grants");
        assert_eq!(grants.len(), 2);
        assert_eq!(
            grants.iter().find(|g| g.user_id == "alice").map(|g| g.mode),
            Some(ShareMode::ReadWrite)
        );

        assert!(can_read(&share.share_id, "peer-alice-1"));
        assert!(can_write(&share.share_id, "peer-alice-1"));

        remove_grant(&share.share_id, "alice").expect("remove");
        let grants = list_grants(&share.share_id).expect("list grants");
        assert_eq!(grants.len(), 1);
        assert_eq!(grants[0].user_id, "bob");

        // Revocation is immediate: the verified binding still exists, but the
        // grant row that authorised it does not.
        assert!(!can_read(&share.share_id, "peer-alice-1"));
        assert!(!can_write(&share.share_id, "peer-alice-1"));
    });
}

#[test]
#[serial(db)]
fn private_acl_uses_verified_user_ids_and_fails_closed() {
    with_test_db(|| {
        let share = create_share("Private", "owner-user", ShareKind::Private, false, None)
            .expect("create share");
        bind_peer("peer-alice-1", "alice");
        bind_peer("peer-bob-1", "bob");

        // Unbound peers and unknown users are denied even before grants exist.
        assert!(!can_read(&share.share_id, "unbound-peer"));
        assert!(!can_write(&share.share_id, "unbound-peer"));

        set_grant(&share.share_id, "alice", ShareMode::ReadOnly).expect("read grant");
        assert!(can_read(&share.share_id, "peer-alice-1"));
        assert!(!can_write(&share.share_id, "peer-alice-1"));
        assert!(!can_read(&share.share_id, "peer-bob-1"));

        set_grant(&share.share_id, "alice", ShareMode::ReadWrite).expect("write grant");
        assert!(can_write(&share.share_id, "peer-alice-1"));
        assert!(!can_write(&share.share_id, "peer-bob-1"));

        // A grant for an unbound user id never matches a peer that merely
        // claims the id on the wire.
        set_grant(&share.share_id, "ghost", ShareMode::ReadWrite).expect("ghost grant");
        assert!(!can_read(&share.share_id, "unbound-peer"));
    });
}

#[test]
#[serial(db)]
fn owner_devices_always_have_write_access() {
    with_test_db(|| {
        let share = create_share("Owned", "owner-user", ShareKind::Private, false, None)
            .expect("create share");
        bind_peer("peer-owner-device-2", "owner-user");
        assert!(can_write(&share.share_id, "peer-owner-device-2"));
    });
}

#[test]
#[serial(db)]
fn public_share_is_readable_by_default_and_write_is_opt_in() {
    with_test_db(|| {
        let ro = create_share("Public RO", "owner", ShareKind::Public, false, None)
            .expect("create share");
        let rw = create_share("Public RW", "owner", ShareKind::Public, true, None)
            .expect("create share");
        // Public shares do not require a binding at all.
        assert!(can_read(&ro.share_id, "any-peer"));
        assert!(!can_write(&ro.share_id, "any-peer"));
        assert!(can_read(&rw.share_id, "any-peer"));
        assert!(can_write(&rw.share_id, "any-peer"));
    });
}

#[test]
#[serial(db)]
fn owner_can_change_visibility_without_enabling_private_writes() {
    with_test_db(|| {
        let share =
            create_share("Photos", "owner", ShareKind::Private, false, None).expect("create share");
        assert_eq!(share.kind, ShareKind::Private);

        let public =
            set_visibility(&share.share_id, "owner", ShareKind::Public, true).expect("make public");
        assert_eq!(public.kind, ShareKind::Public);
        assert!(public.public_write);
        assert!(can_read(&public.share_id, "any-peer"));
        assert!(can_write(&public.share_id, "any-peer"));

        let private = set_visibility(&share.share_id, "owner", ShareKind::Private, true)
            .expect("make private");
        assert_eq!(private.kind, ShareKind::Private);
        assert!(!private.public_write);
        assert!(!can_read(&private.share_id, "any-peer"));

        let stored = get_share(&share.share_id).expect("get").expect("present");
        assert_eq!(stored.kind, ShareKind::Private);
        assert!(!stored.public_write);
    });
}

#[test]
#[serial(db)]
fn visibility_changes_refuse_foreign_owners_and_unknown_shares() {
    with_test_db(|| {
        let share =
            create_share("Photos", "owner", ShareKind::Private, false, None).expect("create share");
        let error = set_visibility(&share.share_id, "stranger", ShareKind::Public, false)
            .expect_err("a peer must not reclassify our share");
        assert!(error.to_string().contains("not owned"));
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .kind,
            ShareKind::Private
        );

        assert!(set_visibility("missing", "owner", ShareKind::Public, false).is_err());
    });
}

#[test]
#[serial(db)]
fn save_manifest_persists_entries_and_revision() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        let id = crate::files::hash_bytes(b"abc");
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("a.txt", id.clone())],
        };
        save_manifest(&share.share_id, &manifest).expect("save");
        assert_eq!(load_manifest(&share.share_id).expect("load"), manifest);
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            1
        );
        assert!(referenced_file_ids().expect("gc refs").contains(&id));

        // A manifest for a different share must be refused.
        let wrong = ShareManifest {
            share_id: "other".to_string(),
            ..manifest
        };
        assert!(save_manifest(&share.share_id, &wrong).is_err());
    });
}

#[test]
#[serial(db)]
fn manifest_matches_share_accepts_only_a_current_snapshot() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        assert!(
            !manifest_matches_share(&share),
            "a share without a manifest has no valid write base"
        );

        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("docs/a.md", crate::files::hash_bytes(b"a"))],
        };
        save_manifest(&share.share_id, &manifest).expect("save");
        let stored = get_share(&share.share_id).expect("get").expect("present");
        assert!(manifest_matches_share(&stored));

        let stale = ShareManifest {
            revision: 2,
            ..manifest.clone()
        };
        assert!(!manifest_matches_share(&Share {
            revision: 1,
            manifest: Some(stale),
            ..stored.clone()
        }));
        assert!(!manifest_matches_share(&Share {
            manifest: Some(ShareManifest {
                share_id: "other-share".to_string(),
                ..manifest
            }),
            ..stored
        }));
    });
}

#[test]
#[serial(db)]
fn manifest_with_deletions_builds_a_validated_next_revision_from_the_store() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        let ids = [
            crate::files::hash_bytes(b"a"),
            crate::files::hash_bytes(b"b"),
            crate::files::hash_bytes(b"c"),
        ];
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 7,
            entries: vec![
                entry("docs/a.md", ids[0].clone()),
                entry("docs/b.md", ids[1].clone()),
                entry("docs/c.md", ids[2].clone()),
            ],
        };
        save_manifest(&share.share_id, &manifest).expect("save");

        let revised = manifest_with_deletions(
            &share.share_id,
            &["docs/b.md".to_string(), "docs/b.md".to_string()],
        )
        .expect("revise");
        assert_eq!(revised.share_id, share.share_id);
        assert_eq!(revised.name, "Docs");
        assert_eq!(revised.revision, 8);
        assert_eq!(
            revised
                .entries
                .iter()
                .map(|e| e.path.as_str())
                .collect::<Vec<_>>(),
            ["docs/a.md", "docs/c.md"]
        );
        assert!(validate_manifest(&revised).is_ok());

        // The helper only builds the snapshot; publishing still owns the write.
        assert_eq!(load_manifest(&share.share_id).expect("load"), manifest);
    });
}

#[test]
#[serial(db)]
fn manifest_with_deletions_rejects_empty_unsafe_and_unmatched_selections() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        let id = crate::files::hash_bytes(b"abc");
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("docs/a.md", id)],
        };
        save_manifest(&share.share_id, &manifest).expect("save");

        for removed in [
            Vec::new(),
            vec!["../secret".to_string()],
            vec!["docs/missing.md".to_string()],
        ] {
            assert!(
                manifest_with_deletions(&share.share_id, &removed).is_err(),
                "accepted selection {removed:?}"
            );
        }
        assert!(manifest_with_deletions("never-created", &["a".to_string()]).is_err());

        // A share without a manifest offers no canonical base to revise.
        let empty = create_share("Empty", "owner", ShareKind::Private, false, None)
            .expect("create empty share");
        assert!(
            manifest_with_deletions(&empty.share_id, &["a".to_string()]).is_err(),
            "revised a share with no manifest"
        );
    });
}

#[test]
#[serial(db)]
fn save_manifest_rejects_same_revision_with_different_content() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        let first_id = crate::files::hash_bytes(b"abc");
        let conflicting_id = crate::files::hash_bytes(b"xyz");
        let first = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("a.txt", first_id.clone())],
        };
        save_manifest(&share.share_id, &first).expect("save first");
        save_manifest(&share.share_id, &first).expect("idempotent re-fetch");

        let conflicting = ShareManifest {
            name: "Renamed".to_string(),
            entries: vec![entry("b.txt", conflicting_id.clone())],
            ..first.clone()
        };
        let error = save_manifest(&share.share_id, &conflicting)
            .expect_err("same revision with different content");
        assert!(error.to_string().contains("conflicts"), "{error}");
        assert_eq!(load_manifest(&share.share_id).expect("load"), first);
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            1
        );
        let refs = referenced_file_ids().expect("gc refs");
        assert!(refs.contains(&first_id));
        assert!(!refs.contains(&conflicting_id));
    });
}

#[test]
#[serial(db)]
fn snapshot_tree_stores_blobs_and_skips_symlinks() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tree dir");
        std::fs::create_dir_all(dir.path().join("nested")).expect("nested dir");
        std::fs::write(dir.path().join("a.txt"), b"alpha").expect("write a");
        std::fs::write(dir.path().join("nested/b.txt"), b"beta").expect("write b");
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.path().join("a.txt"), dir.path().join("link.txt"))
            .expect("symlink");

        let entries = snapshot_tree(dir.path()).expect("snapshot");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].path, "a.txt");
        assert_eq!(entries[1].path, "nested/b.txt");
        assert_eq!(entries[0].file_id, crate::files::hash_bytes(b"alpha"));
        assert!(crate::files::file_path(&entries[0].file_id).is_some());
    });
}

#[test]
#[serial(db)]
fn apply_manifest_writes_blobs_and_removes_only_tracked_files() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let h_new = crate::files::hash_bytes(b"new bytes");
        crate::files::store_outgoing_bytes("new.txt", b"new bytes").expect("store blob");
        std::fs::write(dir.path().join("untracked.txt"), b"keep me").expect("untracked");

        let previous = ShareManifest {
            share_id: "s1".to_string(),
            name: "share".to_string(),
            revision: 1,
            entries: vec![entry("old.txt", h_new.clone())],
        };
        std::fs::write(dir.path().join("old.txt"), b"old").expect("old file");
        let next = ShareManifest {
            entries: vec![entry("new.txt", h_new)],
            ..previous.clone()
        };
        let stats = apply_manifest("s1", dir.path(), Some(&previous), &next).expect("apply");
        assert_eq!(stats.written, 1);
        assert_eq!(stats.removed, 1);
        assert_eq!(
            std::fs::read(dir.path().join("new.txt")).expect("read new"),
            b"new bytes"
        );
        assert!(!dir.path().join("old.txt").exists());
        assert!(dir.path().join("untracked.txt").exists());
    });
}

#[test]
#[serial(db)]
fn apply_manifest_rejects_corrupt_blob() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let wrong_id = crate::files::hash_bytes(b"expected");
        crate::files::store_outgoing_bytes("a.txt", b"actual").expect("store blob");
        let manifest = ShareManifest {
            share_id: "s1".to_string(),
            name: "share".to_string(),
            revision: 1,
            entries: vec![entry("a.txt", wrong_id)],
        };
        assert!(apply_manifest("s1", dir.path(), None, &manifest).is_err());
        assert!(!dir.path().join("a.txt").exists());
    });
}

#[test]
#[serial(db)]
fn apply_manifest_cleans_up_staging_file_when_install_fails() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let id = crate::files::hash_bytes(b"abc");
        crate::files::store_outgoing_bytes("a.txt", b"abc").expect("store blob");
        // A directory at the tracked path makes the final rename fail after the
        // staging file was written. The cleanup must not strand a `.part` file.
        std::fs::create_dir(dir.path().join("a.txt")).expect("make target a directory");
        let manifest = ShareManifest {
            share_id: "s1".to_string(),
            name: "share".to_string(),
            revision: 1,
            entries: vec![entry("a.txt", id)],
        };
        assert!(apply_manifest("s1", dir.path(), None, &manifest).is_err());
        let debris: Vec<_> = std::fs::read_dir(dir.path())
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".part"))
            .collect();
        assert!(debris.is_empty(), "staging file leaked: {debris:?}");
    });
}

#[cfg(unix)]
#[test]
#[serial(db)]
fn apply_manifest_refuses_symlink_components() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let outside = TempDir::new().expect("outside dir");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("escape")).expect("symlink");
        let id = crate::files::hash_bytes(b"abc");
        crate::files::store_outgoing_bytes("a.txt", b"abc").expect("store blob");
        let manifest = ShareManifest {
            share_id: "s1".to_string(),
            name: "share".to_string(),
            revision: 1,
            entries: vec![entry("escape/evil.txt", id)],
        };
        assert!(apply_manifest("s1", dir.path(), None, &manifest).is_err());
        assert!(!outside.path().join("evil.txt").exists());
    });
}

#[test]
#[serial(db)]
fn accept_submitted_manifest_enforces_permission_and_revision() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Shared",
            "owner-user",
            ShareKind::Private,
            false,
            Some(dir.path().to_str().expect("root path")),
        )
        .expect("create share");
        bind_peer("writer-peer", "writer-user");
        set_grant(&share.share_id, "writer-user", ShareMode::ReadWrite).expect("grant");

        let first = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 0,
            entries: vec![],
        };
        let stats = accept_submitted_manifest(&share.share_id, "writer-peer", 0, &first)
            .expect("first submit");
        assert_eq!(stats.written, 0);
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            1
        );

        // Replaying the same base revision is a conflict, not an overwrite.
        let stale = accept_submitted_manifest(&share.share_id, "writer-peer", 0, &first);
        assert!(stale.is_err());

        // An unbound peer cannot write even with a stale revision.
        assert!(accept_submitted_manifest(&share.share_id, "stranger", 1, &first).is_err());
    });
}

#[test]
#[serial(db)]
fn accept_submitted_manifest_reads_acl_on_the_write_connection() {
    with_test_db(|| {
        use diesel::{Connection as _, ExpressionMethods as _, QueryDsl as _, RunQueryDsl as _};

        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Revocable",
            "owner-user",
            ShareKind::Private,
            false,
            Some(dir.path().to_str().expect("root path")),
        )
        .expect("create share");
        bind_peer("writer-peer", "writer-user");
        set_grant(&share.share_id, "writer-user", ShareMode::ReadWrite).expect("grant");

        let first = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 0,
            entries: vec![],
        };
        accept_submitted_manifest(&share.share_id, "writer-peer", 0, &first).expect("first submit");

        // Permission reads follow the caller's connection: a revoke staged in
        // this transaction is visible to `permission_for_share` before commit,
        // which is exactly what the submission path relies on.
        {
            let conn = &mut crate::sqlite_connect().expect("db connection");
            conn.transaction::<(), color_eyre::Report, _>(|conn| {
                let loaded = get_share_conn(conn, &share.share_id)
                    .expect("load share")
                    .expect("present");
                assert_eq!(
                    permission_for_share(conn, &loaded, "writer-peer"),
                    Some(ShareMode::ReadWrite)
                );
                diesel::delete(
                    share_grants::table
                        .filter(share_grants::share_id.eq(&share.share_id))
                        .filter(share_grants::user_id.eq("writer-user")),
                )
                .execute(conn)
                .expect("stage revoke");
                assert_eq!(permission_for_share(conn, &loaded, "writer-peer"), None);
                Ok(())
            })
            .expect("transaction");
        }

        // With the revoke committed, the same submission is denied and leaves
        // the persisted manifest and revision untouched.
        let denied = accept_submitted_manifest(&share.share_id, "writer-peer", 1, &first)
            .expect_err("revoked writer");
        assert!(denied.to_string().contains("may not write"), "{denied}");
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            1
        );
        assert_eq!(load_manifest(&share.share_id).expect("load").revision, 1);
    });
}

#[test]
#[serial(db)]
fn stale_snapshot_with_changed_entries_is_rejected_until_rebased() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Writable",
            "owner",
            ShareKind::Public,
            true,
            Some(dir.path().to_str().expect("root")),
        )
        .expect("create share");
        let first = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 0,
            entries: vec![],
        };
        accept_submitted_manifest(&share.share_id, "any-peer", 0, &first).expect("first submit");

        let id = crate::files::hash_bytes(b"abc");
        crate::files::store_outgoing_bytes("changed.txt", b"abc").expect("store blob");
        let changed = ShareManifest {
            revision: 0,
            entries: vec![entry("changed.txt", id.clone())],
            ..first.clone()
        };

        let stale = accept_submitted_manifest(&share.share_id, "any-peer", 0, &changed)
            .expect_err("stale submission");
        assert!(stale.to_string().contains("stale share revision"));
        assert!(!dir.path().join("changed.txt").exists());
        assert!(!referenced_file_ids().expect("gc refs").contains(&id));

        // Refreshing the base revision lets the same snapshot through and the
        // accepted revision is derived from the share, not the submitted value.
        let stats = accept_submitted_manifest(&share.share_id, "any-peer", 1, &changed)
            .expect("retry at current revision");
        assert_eq!(stats.written, 1);
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            2
        );
        assert_eq!(
            load_manifest(&share.share_id).expect("manifest").revision,
            2
        );
        assert_eq!(
            std::fs::read(dir.path().join("changed.txt")).expect("installed"),
            b"abc"
        );
    });
}

#[test]
#[serial(db)]
fn newer_manifest_supersedes_a_pending_apply() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Remote",
            "owner",
            ShareKind::Public,
            false,
            Some(dir.path().to_str().expect("root")),
        )
        .expect("create share");
        let old_a = crate::files::hash_bytes(b"old-a");
        let old_b = crate::files::hash_bytes(b"old-b");
        let new = crate::files::hash_bytes(b"new");
        let old = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("a.txt", old_a.clone()), entry("b.txt", old_b.clone())],
        };
        let newer = ShareManifest {
            revision: 2,
            entries: vec![entry("c.txt", new.clone())],
            ..old.clone()
        };
        save_manifest(&share.share_id, &newer).expect("save newest manifest");

        let mut queued = queue_apply("serving-peer".into(), None, old.clone()).missing;
        queued.sort();
        let mut expected = vec![old_a.clone(), old_b.clone()];
        expected.sort();
        assert_eq!(queued, expected);
        assert_eq!(
            queue_apply("serving-peer".into(), Some(old), newer.clone()).missing,
            vec![new.clone()]
        );

        // Blobs from the superseded snapshot no longer trigger an apply.
        assert!(note_blob_available(&old_a, "serving-peer").is_empty());
        assert!(note_blob_available(&old_b, "serving-peer").is_empty());
        assert!(!dir.path().join("a.txt").exists());

        crate::files::store_outgoing_bytes("c.txt", b"new").expect("store newest blob");
        let applied = note_blob_available(&new, "serving-peer");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].revision, 2);
        assert_eq!(applied[0].stats.written, 1);
        assert_eq!(
            std::fs::read(dir.path().join("c.txt")).expect("newest"),
            b"new"
        );
    });
}

#[test]
#[serial(db)]
fn disconnect_abandons_only_that_peers_pending_apply() {
    with_test_db(|| {
        let gone_dir = TempDir::new().expect("gone root");
        let stay_dir = TempDir::new().expect("stay root");
        let gone_share = create_share(
            "Gone",
            "owner",
            ShareKind::Public,
            false,
            Some(gone_dir.path().to_str().expect("root")),
        )
        .expect("create gone share");
        let stay_share = create_share(
            "Stay",
            "owner",
            ShareKind::Public,
            false,
            Some(stay_dir.path().to_str().expect("root")),
        )
        .expect("create stay share");

        let gone_id = crate::files::hash_bytes(b"gone");
        let stay_id = crate::files::hash_bytes(b"stay");
        let gone_manifest = ShareManifest {
            share_id: gone_share.share_id.clone(),
            name: gone_share.name.clone(),
            revision: 1,
            entries: vec![entry("gone.txt", gone_id.clone())],
        };
        let stay_manifest = ShareManifest {
            share_id: stay_share.share_id.clone(),
            name: stay_share.name.clone(),
            revision: 1,
            entries: vec![entry("stay.txt", stay_id.clone())],
        };
        save_manifest(&gone_share.share_id, &gone_manifest).expect("save gone manifest");
        save_manifest(&stay_share.share_id, &stay_manifest).expect("save stay manifest");
        assert_eq!(
            queue_apply("gone-peer".into(), None, gone_manifest).missing,
            vec![gone_id.clone()]
        );
        assert_eq!(
            queue_apply("stay-peer".into(), None, stay_manifest).missing,
            vec![stay_id.clone()]
        );

        abandon_pending_from("gone-peer");
        crate::files::store_outgoing_bytes("gone.txt", b"gone").expect("store gone blob");
        crate::files::store_outgoing_bytes("stay.txt", b"stay").expect("store stay blob");
        assert!(note_blob_available(&gone_id, "gone-peer").is_empty());
        assert!(!gone_dir.path().join("gone.txt").exists());

        let applied = note_blob_available(&stay_id, "stay-peer");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].share_id, stay_share.share_id);
        assert_eq!(
            std::fs::read(stay_dir.path().join("stay.txt")).expect("stayed"),
            b"stay"
        );
    });
}

#[test]
#[serial(db)]
fn accept_offer_marks_the_remote_claim_unverified_and_is_idempotent() {
    with_test_db(|| {
        let offer = ShareOffer {
            share_id: "remote-share-42".to_string(),
            name: "Remote".to_string(),
            kind: ShareKind::Public,
            revision: 3,
            public_write: false,
            owner_user_id: Some("owner-user".to_string()),
        };
        let share = accept_offer(&offer, None).expect("accept");
        assert_eq!(share.share_id, "remote-share-42");
        assert_eq!(share.owner_user_id, unverified_claim_id("owner-user"));
        assert!(is_unknown_owner_id(&share.owner_user_id));
        assert_eq!(share.kind, ShareKind::Public);

        let same = accept_offer(&offer, None).expect("repeat offer");
        assert_eq!(same.share_id, share.share_id);
        assert_eq!(same.owner_user_id, unverified_claim_id("owner-user"));
        assert_eq!(list_shares().expect("list").len(), 1);
    });
}

#[test]
#[serial(db)]
fn accept_offer_rejects_marker_and_oversized_owner_claims() {
    with_test_db(|| {
        let marker_claim = accept_offer(
            &ShareOffer {
                share_id: "marker-claim".to_string(),
                name: "Marker".to_string(),
                kind: ShareKind::Private,
                revision: 0,
                public_write: false,
                owner_user_id: Some(unverified_claim_id("owner-user")),
            },
            None,
        )
        .expect("reject marker claim");
        assert_eq!(marker_claim.owner_user_id, unknown_owner_id("marker-claim"));

        let oversized_claim = accept_offer(
            &ShareOffer {
                share_id: "oversized-claim".to_string(),
                name: "Oversized".to_string(),
                kind: ShareKind::Private,
                revision: 0,
                public_write: false,
                owner_user_id: Some("x".repeat(MAX_OWNER_CLAIM_LEN + 1)),
            },
            None,
        )
        .expect("reject oversized claim");
        assert_eq!(
            oversized_claim.owner_user_id,
            unknown_owner_id("oversized-claim")
        );
        let marker_owner = unknown_owner_id("any");
        assert!(
            !record_verified_owner(&oversized_claim.share_id, Some(marker_owner.as_str()))
                .expect("marker is never a verified owner")
        );
    });
}

#[test]
#[serial(db)]
fn unverified_owner_claim_denies_owner_shortcuts_until_rust_verifies_it() {
    with_test_db(|| {
        let offer = ShareOffer {
            share_id: "claimed-share".to_string(),
            name: "Claimed".to_string(),
            kind: ShareKind::Private,
            revision: 0,
            public_write: false,
            owner_user_id: Some("owner-user".to_string()),
        };
        let share = accept_offer(&offer, None).expect("accept claim");
        assert_eq!(share.owner_user_id, unverified_claim_id("owner-user"));
        assert!(is_unknown_owner_id(&share.owner_user_id));
        assert_eq!(
            offer_for_share(&share.share_id)
                .expect("read offer")
                .expect("present")
                .owner_user_id,
            None,
            "a local marker is not a claim peers can verify"
        );

        bind_peer("owner-device", "owner-user");
        assert!(
            !can_write(&share.share_id, "owner-device"),
            "an offer's claim is not an ACL"
        );
        assert!(
            set_visibility(&share.share_id, "owner-user", ShareKind::Public, true).is_err(),
            "a claim cannot unlock owner-only visibility toggles"
        );

        assert!(
            !record_verified_owner(&share.share_id, Some("stranger-user"))
                .expect("wrong-user upgrade"),
            "a different verified user must not take over the claim"
        );
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .owner_user_id,
            unverified_claim_id("owner-user")
        );

        assert!(record_verified_owner(&share.share_id, Some("owner-user")).expect("upgrade"));
        let verified = get_share(&share.share_id).expect("get").expect("present");
        assert_eq!(verified.owner_user_id, "owner-user");
        assert!(can_write(&share.share_id, "owner-device"));
        assert_eq!(
            offer_for_share(&share.share_id)
                .expect("read offer")
                .expect("present")
                .owner_user_id
                .as_deref(),
            Some("owner-user")
        );
    });
}

#[test]
#[serial(db)]
fn verified_owner_upgrade_never_rewrites_no_claim_or_locally_owned_rows() {
    with_test_db(|| {
        let anonymous = accept_offer(
            &ShareOffer {
                share_id: "no-claim".to_string(),
                name: "No claim".to_string(),
                kind: ShareKind::Private,
                revision: 0,
                public_write: false,
                owner_user_id: None,
            },
            None,
        )
        .expect("accept anonymous offer");
        assert_eq!(anonymous.owner_user_id, unknown_owner_id("no-claim"));
        assert!(
            !record_verified_owner(&anonymous.share_id, Some("owner-user"))
                .expect("no-claim upgrade"),
            "only an exact claim marker may be upgraded"
        );
        assert_eq!(
            get_share(&anonymous.share_id)
                .expect("get")
                .expect("present")
                .owner_user_id,
            unknown_owner_id("no-claim")
        );

        let owned = create_share("Mine", "owner-user", ShareKind::Private, false, None)
            .expect("create owned share");
        assert!(
            !record_verified_owner(&owned.share_id, Some("owner-user")).expect("owned upgrade"),
            "a locally owned row is never a claim marker"
        );
        assert_eq!(
            get_share(&owned.share_id)
                .expect("get")
                .expect("present")
                .owner_user_id,
            "owner-user"
        );
    });
}

#[test]
#[serial(db)]
fn verified_owner_can_publish_to_a_mirror_holding_their_claim() {
    with_test_db(|| {
        let share = accept_offer(
            &ShareOffer {
                share_id: "mirror-share".to_string(),
                name: "Mirror".to_string(),
                kind: ShareKind::Private,
                revision: 0,
                public_write: false,
                owner_user_id: Some("owner-user".to_string()),
            },
            None,
        )
        .expect("accept offer");
        assert_eq!(share.owner_user_id, unverified_claim_id("owner-user"));
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 0,
            entries: vec![],
        };

        bind_peer("stranger-device", "stranger-user");
        assert!(
            accept_submitted_manifest(&share.share_id, "stranger-device", 0, &manifest).is_err(),
            "a stranger must not publish through an unverified claim"
        );
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .owner_user_id,
            unverified_claim_id("owner-user")
        );

        bind_peer("owner-device", "owner-user");
        let stats = accept_submitted_manifest(&share.share_id, "owner-device", 0, &manifest)
            .expect("publish");
        assert_eq!(stats.written, 0);
        let stored = get_share(&share.share_id).expect("get").expect("present");
        assert_eq!(stored.owner_user_id, "owner-user");
        assert_eq!(stored.revision, 1);
    });
}

#[test]
fn offer_for_maps_share_and_owner_without_inventing_ownership() {
    let manifest = ShareManifest {
        share_id: "share-1".to_string(),
        name: "Docs".to_string(),
        revision: 7,
        entries: vec![],
    };
    let share = Share {
        share_id: manifest.share_id.clone(),
        owner_user_id: "owner-user".to_string(),
        name: manifest.name.clone(),
        kind: ShareKind::Public,
        public_write: true,
        local_root: None,
        revision: manifest.revision,
        manifest: Some(manifest),
    };

    let offer = offer_for(
        &share,
        share.kind,
        share.public_write,
        Some(&share.owner_user_id),
    );
    assert_eq!(offer.share_id, "share-1");
    assert_eq!(offer.name, "Docs");
    assert_eq!(offer.kind, ShareKind::Public);
    assert!(offer.public_write);
    assert_eq!(offer.revision, 7);
    assert_eq!(offer.owner_user_id.as_deref(), Some("owner-user"));

    let anonymous = offer_for(&share, ShareKind::Private, false, None);
    assert_eq!(anonymous.kind, ShareKind::Private);
    assert!(!anonymous.public_write);
    assert_eq!(anonymous.owner_user_id, None);
}

#[test]
#[serial(db)]
fn offer_for_share_never_advertises_the_unknown_owner_marker() {
    with_test_db(|| {
        let owned = create_share("Owned", "owner-user", ShareKind::Private, false, None)
            .expect("create owned share");
        let owned_offer = offer_for_share(&owned.share_id)
            .expect("read owned share")
            .expect("owned share exists");
        assert_eq!(owned_offer.owner_user_id.as_deref(), Some("owner-user"));
        assert_eq!(owned_offer.kind, ShareKind::Private);
        assert_eq!(owned_offer.revision, owned.revision);

        let anonymous_offer = ShareOffer {
            share_id: "remote-share".to_string(),
            name: "Remote".to_string(),
            kind: ShareKind::Public,
            revision: 3,
            public_write: false,
            owner_user_id: None,
        };
        let remote = accept_offer(&anonymous_offer, None).expect("accept anonymous offer");
        assert_eq!(remote.owner_user_id, unknown_owner_id("remote-share"));
        assert!(is_unknown_owner_id(&remote.owner_user_id));

        let advertised = offer_for_share("remote-share")
            .expect("read remote share")
            .expect("remote share exists");
        assert_eq!(advertised.share_id, "remote-share");
        assert_eq!(advertised.name, "Remote");
        assert_eq!(advertised.kind, ShareKind::Public);
        // A stub has no local manifest yet: revision 0 leaves room for the
        // owner's first manifest to be accepted (see `create_share_with_id`).
        assert_eq!(advertised.revision, 0);
        assert_eq!(
            advertised.owner_user_id, None,
            "a local-only marker is not an ownership claim peers can verify"
        );
    });
}

#[test]
#[serial(db)]
fn queue_apply_waits_for_blobs_then_installs_the_manifest() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Remote",
            "owner",
            ShareKind::Public,
            false,
            Some(dir.path().to_str().expect("root")),
        )
        .expect("create share");
        let id = crate::files::hash_bytes(b"payload");
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("file.txt", id.clone())],
        };
        save_manifest(&share.share_id, &manifest).expect("save manifest");

        let missing = queue_apply("serving-peer".to_string(), None, manifest).missing;
        assert_eq!(missing, vec![id.clone()]);
        assert!(!dir.path().join("file.txt").exists());

        crate::files::store_outgoing_bytes("file.txt", b"payload").expect("store blob");
        let applied = note_blob_available(&id, "serving-peer");
        assert_eq!(applied.len(), 1);
        assert_eq!(applied[0].share_id, share.share_id);
        assert_eq!(applied[0].stats.written, 1);
        assert_eq!(
            std::fs::read(dir.path().join("file.txt")).expect("installed"),
            b"payload"
        );
    });
}

#[test]
#[serial(db)]
fn public_write_accepts_unbound_peer_when_enabled() {
    with_test_db(|| {
        let share =
            create_share("Open", "owner", ShareKind::Public, true, None).expect("create share");
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 0,
            entries: vec![],
        };
        assert!(accept_submitted_manifest(&share.share_id, "any-peer", 0, &manifest).is_ok());
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            1
        );
    });
}

#[test]
#[serial(db)]
fn spoofed_offer_cannot_rewrite_local_acl_or_bound_root() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        // A share we own, bound to a folder, with a published manifest.
        let owned = create_share(
            "Mine",
            "my-user",
            ShareKind::Private,
            false,
            Some(dir.path().to_str().expect("root")),
        )
        .expect("create share");
        let id = crate::files::hash_bytes(b"abc");
        let manifest = ShareManifest {
            share_id: owned.share_id.clone(),
            name: owned.name.clone(),
            revision: 1,
            entries: vec![entry("a.txt", id.clone())],
        };
        save_manifest(&owned.share_id, &manifest).expect("save manifest");

        // A peer claims the same id but tries to flip the ACL and root.
        let spoof = ShareOffer {
            share_id: owned.share_id.clone(),
            name: "Hijacked".to_string(),
            kind: ShareKind::Public,
            revision: 99,
            public_write: true,
            owner_user_id: Some("attacker".to_string()),
        };
        let result = accept_offer(&spoof, None).expect("offer is a no-op, not an error");
        assert_eq!(result.kind, ShareKind::Private);
        assert_eq!(result.name, "Mine");
        assert_eq!(result.owner_user_id, "my-user");
        assert!(!result.public_write || result.kind == ShareKind::Public);

        let stored = get_share(&owned.share_id).expect("get").expect("present");
        assert_eq!(stored.name, "Mine");
        assert_eq!(stored.kind, ShareKind::Private);
        assert_eq!(stored.owner_user_id, "my-user");
        assert_eq!(stored.revision, 1);
        assert_eq!(
            stored.local_root.as_deref(),
            Some(dir.path().to_str().expect("root"))
        );
    });
}

#[test]
#[serial(db)]
fn save_manifest_rejects_missing_share_and_rollback() {
    with_test_db(|| {
        let share =
            create_share("Docs", "owner", ShareKind::Private, false, None).expect("create share");
        let v1 = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![],
        };
        let v2 = ShareManifest {
            revision: 2,
            ..v1.clone()
        };

        // Unknown shares never get orphaned entry rows.
        let ghost = ShareManifest {
            share_id: "never-created".to_string(),
            ..v1.clone()
        };
        assert!(save_manifest("never-created", &ghost).is_err());

        save_manifest(&share.share_id, &v2).expect("save newest");
        // A replayed older response must never roll local state back.
        assert!(save_manifest(&share.share_id, &v1).is_err());
        assert_eq!(
            get_share(&share.share_id)
                .expect("get")
                .expect("present")
                .revision,
            2
        );
        // An equal-revision re-fetch is an idempotent no-op.
        save_manifest(&share.share_id, &v2).expect("idempotent re-fetch");
    });
}

#[test]
#[serial(db)]
fn queue_apply_installs_immediately_when_every_blob_is_local() {
    with_test_db(|| {
        let dir = TempDir::new().expect("root dir");
        let share = create_share(
            "Rename",
            "owner",
            ShareKind::Public,
            false,
            Some(dir.path().to_str().expect("root")),
        )
        .expect("create share");
        let id = crate::files::hash_bytes(b"payload");
        crate::files::store_outgoing_bytes("file.txt", b"payload").expect("store blob first");
        let manifest = ShareManifest {
            share_id: share.share_id.clone(),
            name: share.name.clone(),
            revision: 1,
            entries: vec![entry("file.txt", id)],
        };
        save_manifest(&share.share_id, &manifest).expect("save manifest");

        // No blob is missing, so no fetch will ever trigger `note_blob_available`.
        // The manifest must still be applied to the bound folder now.
        let result = queue_apply("serving-peer".to_string(), None, manifest);
        assert!(result.missing.is_empty());
        assert_eq!(result.applied.len(), 1);
        assert_eq!(result.applied[0].peer_id, "serving-peer");
        assert_eq!(
            std::fs::read(dir.path().join("file.txt")).expect("installed immediately"),
            b"payload"
        );
    });
}
