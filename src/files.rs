//! Content-addressed file attachment store (files lane §12).
//!
//! Every attachment is addressed by the SHA-256 hex of its bytes (`file_id`),
//! so repeated transfers of identical content reuse a single on-disk blob,
//! transfer integrity is verified by re-hashing on finalize, and receivers
//! deduplicate simply by checking whether `file_id` is already stored.
//!
//! The bytes live under `files_dir()` (a `*.files` directory derived from the
//! active database path); the [`files`](crate::generated::schema::files) table
//! records metadata plus the absolute stored path so serving (and the
//! frontends' open/save affordances) have exactly one lookup.
//!
//! Transfers are receiver-driven over the `/p2p-chat/files/1.0.0`
//! `request_response` lane: the receiver asks for metadata, then pulls
//! fixed-size chunks sequentially. Senders prepare a file with
//! [`store_outgoing_file`] (which also verifies the size cap) so it is ready to
//! serve as soon as the message it rides on arrives.

use crate::generated::models_insertable::NewFile;
use crate::generated::schema::files::dsl::files;
use color_eyre::eyre::{Context, eyre};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::time::Duration;

/// Size of one chunk pulled over the file-transfer lane (fits the cbor codec's
/// default 10 MiB response cap).
pub const FILE_CHUNK_SIZE: u64 = 1024 * 1024;

/// Accept/reject ceiling for a single attachment. Bounds transfer memory and
/// bandwidth; the metadata handshake advertises `size` so the receiver refuses
/// (and answers `error`) before pulling.
pub const MAX_ATTACHMENT_SIZE: u64 = 64 * 1024 * 1024;

/// Delivery-facing descriptor of one file attachment.
///
/// Rides inside `DirectMessage`/`BroadcastMessage`/`GroupMessage` on the wire
/// and in the `attachment_json` column of `messages`/`group_messages` on disk.
/// The descriptor itself identifies the content; the bytes are fetched over the
/// file lane on the receiver's side.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MessageAttachment {
    /// SHA-256 hex of the file bytes (content address)
    pub file_id: String,
    /// Original filename as the sender advertised it
    pub name: String,
    /// Total byte length
    pub size: u64,
    /// Best-effort MIME type hint (`None` when unknown)
    pub mime: Option<String>,
}

/// Render an attachment descriptor as a plain-text chat chip, e.g.
/// `[file] notes.txt (2048)`. Used verbatim by frontends (TUI chat rows +
/// input) so a sent and a received file always look identical.
#[must_use]
pub fn file_chip(attachment: &MessageAttachment) -> String {
    format!("[file] {} ({})", attachment.name, attachment.size)
}

/// Compose a chat line body from message text plus an attachment descriptor:
/// text first, chip appended; an attachment-only message renders as just the
/// chip so a file never shows up as a blank row.
#[must_use]
pub fn render_message_body(content: &str, attachment: Option<&MessageAttachment>) -> String {
    if content.trim().is_empty() {
        attachment.map_or_else(String::new, file_chip)
    } else if let Some(attachment) = attachment {
        format!("{content} {}", file_chip(attachment))
    } else {
        content.to_string()
    }
}

/// Hydrate a chat row body from a DB `attachment_json` column (files lane §12)
/// plus the stored message text — the inverse of what the send path persisted.
#[must_use]
pub fn render_stored_body(content: &str, attachment_json: Option<&str>) -> String {
    render_message_body(
        content,
        attachment_json.and_then(attachment_from_json).as_ref(),
    )
}

/// SHA-256 of `data` as a lowercase hex string — the content address
/// (`file_id`) for the bytes.
#[must_use]
pub fn hash_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len().saturating_mul(2));
    for byte in digest {
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

/// Directory that holds the attachment blobs, derived from the active database
/// path: `<db>.files`. Created on first use.
///
/// # Errors
/// Returns an error if the directory cannot be created.
pub fn files_dir() -> color_eyre::Result<std::path::PathBuf> {
    let dir = std::path::PathBuf::from(format!("{}.files", crate::get_database_url()));
    std::fs::create_dir_all(&dir)
        .wrap_err_with(|| format!("failed to create attachment store dir {}", dir.display()))?;
    Ok(dir)
}

/// Absolute on-disk path a `file_id`'s blob lives at (whether or not it exists
/// yet).
///
/// # Errors
/// Returns an error if the store directory cannot be created.
pub fn store_path_for(file_id: &str) -> color_eyre::Result<std::path::PathBuf> {
    Ok(files_dir()?.join(file_id))
}

/// Best-effort MIME type hint from a filename's final extension.
#[must_use]
pub fn mime_from_name(name: &str) -> Option<String> {
    let ext = name.rsplit('.').next()?.to_ascii_lowercase();
    let mime = match ext.as_str() {
        "txt" | "md" | "log" | "rtf" => "text/plain",
        "html" | "htm" => "text/html",
        "csv" => "text/csv",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "m4a" => "audio/mp4",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        _ => return None,
    };
    Some(mime.to_string())
}

/// Serialize an attachment descriptor for the `attachment_json` column.
#[must_use]
pub fn attachment_to_json(attachment: &MessageAttachment) -> Option<String> {
    serde_json::to_string(attachment).ok()
}

/// Deserialize an attachment descriptor from `attachment_json`.
#[must_use]
pub fn attachment_from_json(json: &str) -> Option<MessageAttachment> {
    serde_json::from_str(json).ok()
}

/// Prepare a local file for sending: copy (and hash) its bytes into the store
/// and record the `files` row. Returns the delivery descriptor the frontend
/// attaches to the outgoing message.
///
/// Idempotent by content address: a file already in the store is not rewritten.
///
/// # Errors
/// Returns an error if the file is missing, unreadable, or over
/// [`MAX_ATTACHMENT_SIZE`].
pub fn store_outgoing_file(path: &str) -> color_eyre::Result<MessageAttachment> {
    let data =
        std::fs::read(path).wrap_err_with(|| format!("failed to read attachment {path:?}"))?;
    let size =
        u64::try_from(data.len()).map_err(|_| eyre!("attachment byte length out of range"))?;
    if size > MAX_ATTACHMENT_SIZE {
        return Err(eyre!(
            "attachment too large: {size} bytes (max {MAX_ATTACHMENT_SIZE})"
        ));
    }
    let file_id = hash_bytes(&data);
    let name = std::path::Path::new(path).file_name().map_or_else(
        || "attachment".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let mime = mime_from_name(&name);

    let store_path = store_path_for(&file_id)?;
    if !store_path.exists() {
        std::fs::write(&store_path, &data).wrap_err_with(|| {
            format!("failed to write attachment blob {}", store_path.display())
        })?;
    }
    insert_file_row(&file_id, &name, size, mime.as_deref(), 1, &store_path)?;

    Ok(MessageAttachment {
        file_id,
        name,
        size,
        mime,
    })
}

/// Absolute path of a stored attachment, as the frontends open/save it.
#[must_use]
pub fn file_path(file_id: &str) -> Option<String> {
    if let Ok(row) = find_file(file_id)
        && let Some(row) = row
    {
        return Some(row.path);
    }
    store_path_for(file_id)
        .ok()
        .and_then(|p| p.exists().then_some(p.display().to_string()))
}

/// Number of fixed-size chunks a `size`-byte file splits into ({0} for empty).
#[must_use]
pub const fn total_chunks(size: u64) -> u64 {
    size.div_ceil(FILE_CHUNK_SIZE)
}

/// Serve the metadata handshake for a stored file.
#[must_use]
pub fn serve_file_meta(file_id: &str) -> Option<(String, u64, Option<String>)> {
    find_file(file_id).ok().flatten().map(|row| {
        let size = u64::try_from(row.size).unwrap_or(u64::MAX);
        (row.name, size, row.mime)
    })
}

/// Serve one chunk of a stored file (the sender side of the pull-based lane).
#[must_use]
pub fn serve_chunk(file_id: &str, chunk_index: u64) -> Option<Vec<u8>> {
    use std::io::{Read, Seek, SeekFrom};
    let offset = chunk_index.checked_mul(FILE_CHUNK_SIZE)?;
    let path = store_path_for(file_id).ok()?;
    if !path.exists() {
        return None;
    }
    let mut file = std::fs::File::open(&path).ok()?;
    let file_len = file.metadata().ok()?.len();
    if offset >= file_len {
        return None;
    }
    file.seek(SeekFrom::Start(offset)).ok()?;
    let remaining = file_len.saturating_sub(offset);
    let take = usize::try_from(remaining.min(FILE_CHUNK_SIZE)).ok()?;
    let mut data = vec![0; take];
    file.read_exact(&mut data).ok()?;
    Some(data)
}

/// Look up the `files` row for a `file_id`.
///
/// # Errors
/// Returns an error if the database query fails.
pub fn find_file(
    file_id: &str,
) -> color_eyre::Result<Option<crate::generated::models_queryable::File>> {
    use diesel::{
        ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
    };
    let conn = &mut crate::sqlite_connect()?;
    files
        .filter(crate::generated::schema::files::file_id.eq(file_id))
        .select(crate::generated::models_queryable::File::as_select())
        .first(conn)
        .optional()
        .wrap_err("failed to look up attachment")
}

fn insert_file_row(
    file_id: &str,
    name: &str,
    size: u64,
    mime: Option<&str>,
    direction: i32,
    path: &std::path::Path,
) -> color_eyre::Result<()> {
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect()?;
    let row = NewFile {
        file_id: file_id.to_string(),
        name: name.to_string(),
        size: i64::try_from(size).unwrap_or(i64::MAX),
        mime: mime.map(std::string::ToString::to_string),
        direction,
        path: path.display().to_string(),
    };
    diesel::insert_into(files)
        .values(&row)
        .on_conflict(crate::generated::schema::files::file_id)
        .do_nothing()
        .execute(conn)
        .wrap_err("failed to record attachment file")?;
    Ok(())
}

/// In-progress inbound transfer assembled chunk-by-chunk on the receiver.
///
/// Owned by the swarm handler (keyed by `file_id` in a process-wide map), so
/// partial progress survives the individual request/response round-trips. The
/// blob is appended to a temporary `.part` path inside [`files_dir`] and only
/// renamed into the store once the hash verifies against `file_id`.
#[derive(Debug, Clone)]
pub struct ReceivingFile {
    /// Content address the delivered bytes must match
    pub file_id: String,
    /// Original filename advertised by the sender
    pub name: String,
    /// MIME hint advertised by the sender
    pub mime: Option<String>,
    /// Expected byte length (from the metadata handshake)
    pub size: u64,
    /// Bytes received so far
    pub received: u64,
    /// Temporary in-progress path (finalized via [`ReceivingFile::finalize`])
    pub path: std::path::PathBuf,
}

impl ReceivingFile {
    /// Start assembling a transfer in the store directory.
    ///
    /// # Errors
    /// Returns an error if the store directory cannot be created.
    pub fn new(
        file_id: &str,
        name: &str,
        mime: Option<String>,
        size: u64,
    ) -> color_eyre::Result<Self> {
        let path = files_dir()?.join(format!("{file_id}.part"));
        Ok(Self {
            file_id: file_id.to_string(),
            name: name.to_string(),
            mime,
            size,
            received: 0,
            path,
        })
    }

    /// Append one chunk's bytes to the in-progress blob.
    ///
    /// # Errors
    /// Returns an error if the append fails.
    pub fn write_chunk(&mut self, data: &[u8]) -> color_eyre::Result<()> {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .wrap_err("failed to open in-progress attachment")?;
        file.write_all(data)
            .wrap_err("failed to append attachment chunk")?;
        self.received = self
            .received
            .saturating_add(u64::try_from(data.len()).unwrap_or(u64::MAX));
        Ok(())
    }

    /// Whether every advertised byte has arrived.
    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.received >= self.size
    }

    /// Verify the assembled bytes hash to `file_id`, move them into the store,
    /// and record the `files` row (direction = received). On any mismatch the
    /// partial blob is removed. An empty file (size 0) needs no `.part` blob:
    /// the store entry is created directly.
    ///
    /// # Errors
    /// Returns an error if the blob is unreadable, the hash does not match, or
    /// the store/DB update fails.
    pub fn finalize(self) -> color_eyre::Result<String> {
        let store_path = store_path_for(&self.file_id)?;
        if self.size == 0 {
            if !store_path.exists() {
                std::fs::write(&store_path, []).wrap_err_with(|| {
                    format!(
                        "failed to create empty attachment blob {}",
                        store_path.display()
                    )
                })?;
            }
            insert_file_row(
                &self.file_id,
                &self.name,
                self.size,
                self.mime.as_deref(),
                0,
                &store_path,
            )?;
            return Ok(store_path.display().to_string());
        }
        let data = std::fs::read(&self.path).wrap_err_with(|| {
            format!(
                "failed to read in-progress attachment {}",
                self.path.display()
            )
        })?;
        if hash_bytes(&data) != self.file_id {
            let _ = std::fs::remove_file(&self.path);
            return Err(eyre!(
                "attachment hash mismatch (expected {}, got {})",
                self.file_id,
                hash_bytes(&data)
            ));
        }
        std::fs::rename(&self.path, &store_path).wrap_err_with(|| {
            format!(
                "failed to finalize attachment {} -> {}",
                self.path.display(),
                store_path.display()
            )
        })?;
        insert_file_row(
            &self.file_id,
            &self.name,
            self.size,
            self.mime.as_deref(),
            0,
            &store_path,
        )?;
        Ok(store_path.display().to_string())
    }

    /// Drop the in-progress blob (transfer failed or was aborted).
    pub fn discard(&self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Look up the stored attachment a received message's descriptor refers to.
///
/// Used by frontends to resolve a descriptor to a local path (and to skip the
/// transfer when the content is already on disk).
#[must_use]
pub fn attachment_path(attachment: &MessageAttachment) -> Option<String> {
    file_path(&attachment.file_id)
}

/// Minimum age a blob must reach before the GC is allowed to delete it.
///
/// Guards the brief window after `ReceivingFile::finalize` renames a blob into
/// the store but before its `files` row (and the message reference) lands, and
/// anything else that is still being written.
pub const GC_GRACE: Duration = Duration::from_mins(1);

/// What one attachment-GC pass deleted (files lane §12).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct GcStats {
    /// `.part` partials of interrupted pulls that were removed
    pub incomplete_removed: usize,
    /// Full blobs / broken `files` rows that were removed
    pub orphans_removed: usize,
}

/// Attachment garbage collection (blob lifecycle, files lane §12): deletes
/// store entries nothing can use any more.
///
/// Two classes are swept:
/// - **Incomplete**: `.part` partials from interrupted pulls (crashes, aborted
///   transfers) with no transfer in flight backing them.
/// - **Overwritten/orphaned**: full blobs no live message references (superseded
///   content after a DB reset or message loss), blobs detached from their
///   `files` row, and `files` rows whose blob is missing on disk.
///
/// `active` is the set of `file_id`s currently being pulled (never swept) and
/// `grace` the minimum blob age before removal. Invocation is the knob: the
/// feature-gated `crate::swarm_handler::gc_stale_attachments` only runs it when
/// the `attachment-gc` Cargo feature is on.
///
/// # Errors
/// Returns an error if the store directory cannot be created or a DB query
/// fails; individual unreadable/deleted blobs are skipped, not fatal.
pub fn gc<S: std::hash::BuildHasher>(
    active: &HashSet<String, S>,
    grace: Duration,
) -> color_eyre::Result<GcStats> {
    Ok(GcStats {
        incomplete_removed: sweep_incomplete_parts(active, grace)?,
        orphans_removed: sweep_orphan_blobs(grace)?,
    })
}

/// Remove every `.part` partial whose `file_id` has no transfer in flight and
/// that is older than `grace`; returns how many were deleted.
fn sweep_incomplete_parts<S: std::hash::BuildHasher>(
    active: &HashSet<String, S>,
    grace: Duration,
) -> color_eyre::Result<usize> {
    let mut removed = 0usize;
    for entry in std::fs::read_dir(files_dir()?).wrap_err("failed to scan attachment store")? {
        let entry = entry.wrap_err("failed to read attachment store entry")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(file_id) = name.strip_suffix(".part") else {
            continue;
        };
        if active.contains(file_id) || !blob_older_than(entry.path().as_path(), grace) {
            continue;
        }
        if std::fs::remove_file(entry.path()).is_ok() {
            removed = removed.saturating_add(1);
        }
    }
    Ok(removed)
}

/// Remove blobs nothing can use any more (see [`gc`]); returns how many
/// artifacts (blobs or rows) were deleted.
fn sweep_orphan_blobs(grace: Duration) -> color_eyre::Result<usize> {
    let referenced = referenced_file_ids()?;
    let mut removed = 0usize;
    for entry in std::fs::read_dir(files_dir()?).wrap_err("failed to scan attachment store")? {
        let entry = entry.wrap_err("failed to read attachment store entry")?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_part_blob(&name)
            || !entry.file_type().is_ok_and(|t| t.is_file())
            || !blob_older_than(entry.path().as_path(), grace)
        {
            continue;
        }
        let file_id = name;
        if referenced.contains(&file_id) {
            continue;
        }
        // A blob with no live message reference is orphaned: delete the blob,
        // and its `files` row too if one lingers.
        if std::fs::remove_file(entry.path()).is_ok() {
            removed = removed.saturating_add(1);
        }
        if find_file(&file_id)?.is_some() {
            delete_file_row(&file_id)?;
            removed = removed.saturating_add(1);
        }
    }
    removed = removed.saturating_add(sweep_broken_rows()?);
    Ok(removed)
}

/// Delete `files` rows whose stored blob no longer exists on disk; returns how
/// many rows were removed (the blob is gone, so such a row can never be served
/// or opened again).
fn sweep_broken_rows() -> color_eyre::Result<usize> {
    use crate::generated::schema::files::dsl::{file_id, path};
    use diesel::QueryDsl as _;
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect()?;
    let rows = files
        .select((file_id, path))
        .load::<(String, String)>(conn)
        .wrap_err("failed to list stored attachments")?;
    let mut removed = 0usize;
    for (id, stored) in rows {
        if !std::path::Path::new(&stored).exists() {
            delete_file_row(&id)?;
            removed = removed.saturating_add(1);
        }
    }
    Ok(removed)
}

/// The set of `file_id`s referenced by any persisted message
/// (`messages.attachment_json` ∪ `group_messages.attachment_json`).
fn referenced_file_ids() -> color_eyre::Result<HashSet<String>> {
    use crate::generated::schema::group_messages::dsl::{
        attachment_json as group_attachment_json, group_messages,
    };
    use crate::generated::schema::messages::dsl::{attachment_json, messages};
    use diesel::QueryDsl as _;
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect()?;
    let message_rows = messages
        .select(attachment_json)
        .load::<Option<String>>(conn)
        .wrap_err("failed to scan message attachments")?;
    let group_rows = group_messages
        .select(group_attachment_json)
        .load::<Option<String>>(conn)
        .wrap_err("failed to scan group attachments")?;
    let mut referenced = HashSet::new();
    for json in message_rows.into_iter().chain(group_rows) {
        let Some(json) = json else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&json) else {
            continue;
        };
        if let Some(file_id) = value.get("file_id").and_then(serde_json::Value::as_str) {
            referenced.insert(file_id.to_string());
        }
    }
    Ok(referenced)
}

fn delete_file_row(file_id: &str) -> color_eyre::Result<()> {
    use crate::generated::schema::files::dsl::files as files_table;
    use diesel::ExpressionMethods as _;
    use diesel::QueryDsl as _;
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect()?;
    diesel::delete(files_table.filter(crate::generated::schema::files::file_id.eq(file_id)))
        .execute(conn)
        .wrap_err("failed to remove attachment row")?;
    Ok(())
}

/// Whether a store entry name is an in-progress `.part` partial (the extension
/// comparison ignores case, like the filesystem's conventional hints).
#[must_use]
fn is_part_blob(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("part"))
}

/// Whether the blob file at `path` was last modified at least `grace` ago (`true`
/// for files we cannot stat — a conservative *skip* would be safer, but a swept
/// directory entry that vanishes mid-scan is already handled by the caller's
/// `is_ok` guard, so treat the un-statable case as stale).
#[must_use]
fn blob_older_than(path: &std::path::Path, grace: Duration) -> bool {
    let modified = match std::fs::metadata(path) {
        Ok(metadata) => metadata.modified(),
        Err(_) => return true,
    };
    modified.is_ok_and(|m| m.elapsed().is_ok_and(|age| age >= grace))
}

#[cfg(test)]
#[path = "../tests/unit/unit_files.rs"]
mod tests;
