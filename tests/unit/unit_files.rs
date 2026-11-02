use super::*;
use diesel::prelude::*;
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

#[test]
fn hash_bytes_is_lowercase_hex_sha256() {
    let hash = hash_bytes(b"hello world");
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    assert_eq!(hash, hash.to_lowercase());
    assert_ne!(hash_bytes(b"hello worlD"), hash_bytes(b"hello world"));
}

#[test]
fn total_chunks_rounds_up_to_fixed_slice() {
    assert_eq!(total_chunks(0), 0);
    assert_eq!(total_chunks(1), 1);
    assert_eq!(total_chunks(FILE_CHUNK_SIZE), 1);
    assert_eq!(total_chunks(FILE_CHUNK_SIZE + 1), 2);
    assert_eq!(total_chunks(FILE_CHUNK_SIZE * 3), 3);
}

#[test]
fn attachment_json_roundtrips() {
    let att = MessageAttachment {
        file_id: "deadbeef".into(),
        name: "notes.txt".into(),
        size: 42,
        mime: Some("text/plain".into()),
    };
    let json = attachment_to_json(&att).expect("serialize");
    let back = attachment_from_json(&json).expect("deserialize");
    assert_eq!(back, att);

    let no_mime = MessageAttachment { mime: None, ..att };
    let json = attachment_to_json(&no_mime).expect("serialize");
    assert!(json.contains("\"mime\":null"));
    assert_eq!(attachment_from_json(&json), Some(no_mime));

    assert_eq!(attachment_from_json("not json"), None);
}

#[test]
fn file_chip_renders_name_and_size() {
    let att = MessageAttachment {
        file_id: "aabb".into(),
        name: "notes.txt".into(),
        size: 2048,
        mime: None,
    };
    assert_eq!(file_chip(&att), "[file] notes.txt (2 KiB)");
}

#[test]
fn format_file_size_uses_binary_units_with_one_decimal() {
    assert_eq!(format_file_size(0), "0 B");
    assert_eq!(format_file_size(5), "5 B");
    assert_eq!(format_file_size(1023), "1023 B");
    assert_eq!(format_file_size(1024), "1 KiB");
    assert_eq!(format_file_size(1536), "1.5 KiB");
    assert_eq!(format_file_size(2 * 1024 * 1024), "2 MiB");
    // The send/receive ceiling rendered the way a chip shows it.
    assert_eq!(format_file_size(MAX_ATTACHMENT_SIZE), "64 MiB");
    // Rounding up across a unit boundary never leaves a bare "1024 <unit>".
    assert_eq!(format_file_size(1024 * 1024 - 1), "1 MiB");
    // Whole u64 range: caps at TiB and stays arithmetic-only (no floats).
    assert_eq!(format_file_size(u64::MAX), "16777216 TiB");
}

#[test]
fn render_message_body_composes_text_and_chip() {
    let att = MessageAttachment {
        file_id: "aabb".into(),
        name: "photo.png".into(),
        size: 5,
        mime: Some("image/png".into()),
    };
    assert_eq!(
        render_message_body("look", Some(&att)),
        "look [file] photo.png (5 B)"
    );
    assert!(render_message_body("", Some(&att)).contains("[file] photo.png (5 B)"));
    assert_eq!(render_message_body("plain", None), "plain");
    assert_eq!(render_message_body("", None), "");
}

#[test]
fn render_stored_body_hydrates_attachment_json_column() {
    let att = MessageAttachment {
        file_id: "c0ffee".into(),
        name: "archive.zip".into(),
        size: 99,
        mime: None,
    };
    let json = attachment_to_json(&att).expect("serialize");
    assert!(
        render_stored_body("", Some(&json)).contains("[file] archive.zip (99 B)"),
        "attachment-only row must still show the chip"
    );
    assert!(
        render_stored_body("for you", Some(&json)).contains("for you [file] archive.zip (99 B)")
    );
    assert_eq!(render_stored_body("hi", Some("garbage")), "hi");
    assert_eq!(render_stored_body("hi", None), "hi");
}

#[test]
fn mime_from_name_maps_known_extensions() {
    assert_eq!(mime_from_name("a.TXT").as_deref(), Some("text/plain"));
    assert_eq!(mime_from_name("photo.jpeg").as_deref(), Some("image/jpeg"));
    assert_eq!(mime_from_name("no-extension"), None);
    assert_eq!(mime_from_name(".hidden"), None);
}

#[test]
#[serial(db)]
fn store_outgoing_file_roundtrips_and_serves() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("payload.txt");
        let bytes = b"hello attachment store".to_vec();
        std::fs::write(&path, &bytes).expect("write source");

        let att = store_outgoing_file(path.to_str().expect("path")).expect("store");
        assert_eq!(att.name, "payload.txt");
        assert_eq!(att.size, u64::try_from(bytes.len()).unwrap());
        assert_eq!(att.mime.as_deref(), Some("text/plain"));
        assert_eq!(att.file_id, hash_bytes(&bytes));

        let resolved = file_path(&att.file_id).expect("resolved path");
        assert_eq!(std::fs::read(&resolved).expect("read blob"), bytes);

        let (name, size, mime) = serve_file_meta(&att.file_id).expect("meta");
        assert_eq!(name, att.name);
        assert_eq!(size, att.size);
        assert_eq!(mime, att.mime);

        assert_eq!(serve_chunk(&att.file_id, 0), Some(bytes));
        assert_eq!(serve_chunk(&att.file_id, 1), None);
        assert_eq!(serve_chunk("no-such-file", 0), None);
    });
}

#[test]
#[serial(db)]
fn store_outgoing_file_is_idempotent_by_content_address() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("repeat.bin");
        let bytes = vec![0xAB; 4096];
        std::fs::write(&path, &bytes).expect("write source");

        let first = store_outgoing_file(path.to_str().expect("path")).expect("first");
        let second = store_outgoing_file(path.to_str().expect("path")).expect("second");
        assert_eq!(first, second);
        assert_eq!(second.file_id, hash_bytes(&bytes));
    });
}

#[test]
#[serial(db)]
fn serve_chunk_honors_fixed_slice_boundaries() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("big.bin");
        let chunk_bytes: usize = usize::try_from(FILE_CHUNK_SIZE).unwrap();
        let tail = chunk_bytes / 2;
        let mut bytes = vec![0u8; chunk_bytes * 2 + tail];
        for (i, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::try_from(i % 251).unwrap();
        }
        std::fs::write(&path, &bytes).expect("write source");

        let att = store_outgoing_file(path.to_str().expect("path")).expect("store");
        assert_eq!(total_chunks(att.size), 3);

        let mut assembled = Vec::new();
        for (i, chunk) in serve_chunk(&att.file_id, 0)
            .into_iter()
            .chain(serve_chunk(&att.file_id, 1))
            .chain(serve_chunk(&att.file_id, 2))
            .enumerate()
        {
            let expected = if i < 2 { chunk_bytes } else { tail };
            assert_eq!(chunk.len(), expected);
            assembled.extend_from_slice(&chunk);
        }
        assert_eq!(assembled, bytes);
        assert_eq!(hash_bytes(&assembled), att.file_id);
    });
}

#[test]
#[serial(db)]
fn store_outgoing_file_rejects_oversize() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tempdir");
        let path = dir.path().join("huge.bin");
        let overflow = usize::try_from(MAX_ATTACHMENT_SIZE)
            .unwrap()
            .saturating_add(1);
        let data = vec![0; overflow];
        std::fs::write(&path, &data).expect("write source");

        let err = store_outgoing_file(path.to_str().expect("path")).expect_err("must reject");
        let rendered = format!("{err:?}");
        assert!(rendered.contains("attachment too large"));
    });
}

#[test]
#[serial(db)]
fn receiving_file_finalizes_content_addressed() {
    with_test_db(|| {
        let bytes = b"received bytes to verify".to_vec();
        let file_id = hash_bytes(&bytes);
        let mut receiving =
            ReceivingFile::new(&file_id, "incoming.txt", Some("text/plain".into()), 0)
                .expect("new");
        let size = u64::try_from(bytes.len()).unwrap();
        receiving.size = size;

        let mid = size / 2;
        receiving
            .write_chunk(&bytes[..usize::try_from(mid).unwrap()])
            .expect("first half");
        assert!(!receiving.is_complete());
        receiving
            .write_chunk(&bytes[usize::try_from(mid).unwrap()..])
            .expect("second half");
        assert!(receiving.is_complete());

        let stored = receiving.finalize().expect("finalize");
        assert_eq!(std::fs::read(&stored).expect("read blob"), bytes);
        assert_eq!(file_path(&file_id).as_deref(), Some(stored.as_str()));
    });
}

#[test]
#[serial(db)]
fn receiving_file_empty_finalizes_without_blob() {
    with_test_db(|| {
        let file_id = hash_bytes(&[]);
        let receiving = ReceivingFile::new(&file_id, "empty.txt", None, 0).expect("new");
        assert!(receiving.is_complete());
        let stored = receiving.finalize().expect("finalize");
        assert_eq!(std::fs::read(&stored).expect("read blob"), Vec::<u8>::new());
        assert_eq!(file_path(&file_id).as_deref(), Some(stored.as_str()));
    });
}

#[test]
#[serial(db)]
fn receiving_file_hash_mismatch_removes_partial() {
    with_test_db(|| {
        let bytes = b"tampered".to_vec();
        let wrong_id = hash_bytes(b"different content");
        let mut receiving = ReceivingFile::new(&wrong_id, "bad.txt", None, 0).expect("new");
        receiving.size = u64::try_from(bytes.len()).unwrap();
        receiving.write_chunk(&bytes).expect("write");
        let partial_path = receiving.path.clone();

        let err = receiving.finalize().expect_err("hash mismatch");
        let rendered = format!("{err:?}");
        assert!(rendered.contains("hash mismatch"));
        assert!(!partial_path.exists(), "partial blob must be cleaned up");
        assert!(
            find_file(&wrong_id).expect("lookup").is_none(),
            "no files row may survive a failed transfer"
        );
    });
}

/// Insert a `messages` row whose `attachment_json` references `file_id`, so the
/// GC treats the blob as live content.
fn insert_message_with_attachment(file_id: &str, json: &str) {
    use crate::generated::models_insertable::NewMessage;
    use crate::generated::schema::messages::dsl::messages;
    let conn = &mut crate::sqlite_connect().expect("connect");
    let row = NewMessage {
        content: String::new(),
        peer_id: None,
        topic: "gc".to_string(),
        sent: 1,
        is_direct: 0,
        target_peer: None,
        msg_id: Some(format!("gc-{file_id}")),
        sent_at: None,
        sender_nickname: None,
        attachment_json: Some(json.to_string()),
        encrypted: 0,
    };
    diesel::insert_into(messages)
        .values(&row)
        .execute(conn)
        .expect("insert message row");
}

#[test]
#[serial(db)]
fn gc_sweeps_stale_parts_but_not_active_transfers() {
    with_test_db(|| {
        let dir = files_dir().expect("store dir");
        let stale = dir.join("deadbeef.part");
        let live = dir.join("c0ffee.part");
        std::fs::write(&stale, b"partial").expect("write stale");
        std::fs::write(&live, b"partial").expect("write live");

        let active = HashSet::from(["c0ffee".to_string()]);
        let stats = gc(&active, Duration::ZERO).expect("gc");

        assert_eq!(stats.incomplete_removed, 1);
        assert_eq!(stats.orphans_removed, 0);
        assert!(!stale.exists(), "stale .part must be swept");
        assert!(live.exists(), "in-flight .part must be kept");
    });
}

#[test]
#[serial(db)]
fn gc_keeps_parts_younger_than_grace() {
    with_test_db(|| {
        let fresh = files_dir().expect("store dir").join("fresh.part");
        std::fs::write(&fresh, b"partial").expect("write fresh");

        let stats = gc(&HashSet::new(), Duration::from_hours(1)).expect("gc");

        assert_eq!(stats.incomplete_removed, 0);
        assert!(
            fresh.exists(),
            "young partials must survive the grace window"
        );
    });
}

#[test]
#[serial(db)]
fn gc_removes_orphans_and_broken_rows_keeps_referenced() {
    with_test_db(|| {
        let dir = TempDir::new().expect("tempdir");

        // A blob a live message references survives the sweep.
        let kept_path = dir.path().join("kept.txt");
        std::fs::write(&kept_path, b"kept bytes").expect("write source");
        let kept = store_outgoing_file(kept_path.to_str().expect("path")).expect("store");
        insert_message_with_attachment(&kept.file_id, &attachment_to_json(&kept).expect("json"));
        assert!(find_file(&kept.file_id).expect("lookup").is_some());

        // A staged blob with a `files` row but no message reference is an orphan.
        let orphan_path = dir.path().join("orphan.bin");
        std::fs::write(&orphan_path, b"orphan bytes").expect("write source");
        let orphan = store_outgoing_file(orphan_path.to_str().expect("path")).expect("store");

        // A `files` row whose blob vanished is broken.
        let broken_path = dir.path().join("broken.bin");
        std::fs::write(&broken_path, b"broken bytes").expect("write source");
        let broken = store_outgoing_file(broken_path.to_str().expect("path")).expect("store");
        let broken_blob = files_dir().expect("store").join(&broken.file_id);
        std::fs::remove_file(broken_blob).expect("simulate lost blob");

        // A blob on disk with no `files` row at all.
        let rowless_id = hash_bytes(b"rowless");
        std::fs::write(files_dir().expect("store").join(&rowless_id), b"rowless")
            .expect("write rowless blob");

        let stats = gc(&HashSet::new(), Duration::ZERO).expect("gc");
        assert_eq!(stats.incomplete_removed, 0);
        // orphan: blob + row (2), broken row (1), rowless blob (1)
        assert_eq!(stats.orphans_removed, 4);

        // The referenced file is untouched.
        assert!(find_file(&kept.file_id).expect("lookup").is_some());
        assert!(files_dir().expect("store").join(&kept.file_id).exists());
        // Everything else is gone: orphaned, broken, rowless.
        assert!(find_file(&orphan.file_id).expect("lookup").is_none());
        assert!(!files_dir().expect("store").join(&orphan.file_id).exists());
        assert!(find_file(&broken.file_id).expect("lookup").is_none());
        assert!(!files_dir().expect("store").join(&rowless_id).exists());
    });
}
