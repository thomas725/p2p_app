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
use crate::generated::schema::{device_bindings, peers, users};
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

        let mut queued = queue_apply("serving-peer".into(), None, old.clone());
        queued.sort();
        let mut expected = vec![old_a.clone(), old_b.clone()];
        expected.sort();
        assert_eq!(queued, expected);
        assert_eq!(
            queue_apply("serving-peer".into(), Some(old), newer.clone()),
            vec![new.clone()]
        );

        // Blobs from the superseded snapshot no longer trigger an apply.
        assert!(note_blob_available(&old_a).is_empty());
        assert!(note_blob_available(&old_b).is_empty());
        assert!(!dir.path().join("a.txt").exists());

        crate::files::store_outgoing_bytes("c.txt", b"new").expect("store newest blob");
        let applied = note_blob_available(&new);
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
            queue_apply("gone-peer".into(), None, gone_manifest),
            vec![gone_id.clone()]
        );
        assert_eq!(
            queue_apply("stay-peer".into(), None, stay_manifest),
            vec![stay_id.clone()]
        );

        abandon_pending_from("gone-peer");
        crate::files::store_outgoing_bytes("gone.txt", b"gone").expect("store gone blob");
        crate::files::store_outgoing_bytes("stay.txt", b"stay").expect("store stay blob");
        assert!(note_blob_available(&gone_id).is_empty());
        assert!(!gone_dir.path().join("gone.txt").exists());

        let applied = note_blob_available(&stay_id);
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
fn accept_offer_keeps_the_remote_id_and_is_idempotent() {
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
        assert_eq!(share.owner_user_id, "owner-user");
        assert_eq!(share.kind, ShareKind::Public);

        let same = accept_offer(&offer, None).expect("repeat offer");
        assert_eq!(same.share_id, share.share_id);
        assert_eq!(list_shares().expect("list").len(), 1);
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

        let missing = queue_apply("serving-peer".to_string(), None, manifest);
        assert_eq!(missing, vec![id.clone()]);
        assert!(!dir.path().join("file.txt").exists());

        crate::files::store_outgoing_bytes("file.txt", b"payload").expect("store blob");
        let applied = note_blob_available(&id);
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
