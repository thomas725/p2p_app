//! File shares and folder sync (share lane).
//!
//! The attachments lane ([`crate::files`], [`crate::file_transfer`]) moves single
//! files. This module builds on the same content-addressed blob store to share
//! a *versioned tree*: a [`ShareManifest`] names every file by its relative path
//! and SHA-256 content address, and the bytes themselves travel over the
//! existing receiver-driven file lane.
//!
//! Invariants that everything here upholds:
//!
//! - **Verified targets only.** Private grants key by `user_id` resolved from a
//!   signed device binding ([`crate::identity::find_user_id_for_peer`]), never
//!   by an ID the requester claims in a payload. Access checks fail *closed*: a
//!   DB error or an unbound peer denies the request.
//! - **Public is opt-in.** A public share is readable by any connected peer
//!   (the swarm handler already gates the blocklist); writes require the share's
//!   explicit `public_write` flag.
//! - **No stale writes.** Each submitted manifest carries its base revision;
//!   [`accept_submitted_manifest`] refuses it unless that revision is still
//!   current, so the writer must refresh and retry instead of silently
//!   overwriting another peer's changes.
//! - **Paths stay inside the bound folder.** Every path is validated before it
//!   is stored or applied, and every resolved component is checked for symlinks,
//!   so a hostile manifest can neither escape the root nor delete files it does
//!   not own. Only files tracked by the previous manifest are ever removed.

use crate::generated::models_insertable::{NewShare, NewShareEntry, NewShareGrant};
use crate::generated::models_queryable::{ShareGrantRow, ShareRow};
use crate::generated::schema::{share_entries, share_grants, shares};
use crate::p2plog_debug;
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _,
    SelectableHelper as _,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

/// Maximum number of files in one share manifest. Bounds the wire payload and
/// the work done applying a hostile manifest.
pub const MAX_SHARE_ENTRIES: usize = 10_000;

/// Maximum length of a share display name.
pub const MAX_SHARE_NAME_LEN: usize = 512;

/// Maximum length of a validated relative path.
pub const MAX_SHARE_PATH_LEN: usize = 4096;

/// Visibility of a share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ShareKind {
    /// Only peers whose verified `user_id` holds a grant may access it.
    #[default]
    Private,
    /// Any connected peer may discover and fetch it.
    Public,
}

impl ShareKind {
    /// Wire/DB value (`0` private, `1` public).
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        match self {
            Self::Private => 0,
            Self::Public => 1,
        }
    }

    /// Parse a stored value; unknown values are treated as private (fail closed).
    #[must_use]
    pub const fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::Public,
            _ => Self::Private,
        }
    }
}

/// Permission a target user holds on a share.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ShareMode {
    /// May fetch the manifest and blobs, but not submit changes.
    #[default]
    ReadOnly,
    /// May submit a new snapshot based on the current revision.
    ReadWrite,
}

impl ShareMode {
    /// Wire/DB value (`0` read-only, `1` read-write).
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        match self {
            Self::ReadOnly => 0,
            Self::ReadWrite => 1,
        }
    }

    /// Parse a stored value; unknown values are treated as read-only (fail closed).
    #[must_use]
    pub const fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::ReadWrite,
            _ => Self::ReadOnly,
        }
    }
}

/// Canonical user-facing label for a share kind (`serde` has no `Display`).
/// Shared by the FRB/web facade and the TUI so both frontends print the same.
#[must_use]
pub const fn kind_label(kind: ShareKind) -> &'static str {
    match kind {
        ShareKind::Private => "private",
        ShareKind::Public => "public",
    }
}

/// Canonical user-facing label for a grant mode.
#[must_use]
pub const fn mode_label(mode: ShareMode) -> &'static str {
    match mode {
        ShareMode::ReadOnly => "readonly",
        ShareMode::ReadWrite => "readwrite",
    }
}

/// One file in a share manifest: a validated relative path plus its content
/// address. The bytes live in the shared blob store, never in the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ShareEntry {
    /// Relative path with forward slashes, e.g. `docs/readme.md`.
    pub path: String,
    /// SHA-256 hex of the file bytes (the `files` table's `file_id`).
    pub file_id: String,
    /// Byte length of the file.
    pub size: u64,
    /// Best-effort MIME type hint.
    pub mime: Option<String>,
}

/// A versioned snapshot of a shared tree.
///
/// `revision` is the optimistic-concurrency token: a write is only accepted
/// when the owner's current revision still equals the base this manifest was
/// built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ShareManifest {
    /// Stable share id shared with peers on the wire.
    pub share_id: String,
    /// Human-readable share name.
    pub name: String,
    /// Monotonically increasing snapshot number.
    pub revision: u64,
    /// Every regular file in the tree, sorted by path.
    pub entries: Vec<ShareEntry>,
}

/// Advertisement sent to peers when a share is published or updated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ShareOffer {
    /// Stable share id used for follow-up manifest requests.
    pub share_id: String,
    /// Human-readable share name.
    pub name: String,
    /// Private (grant-gated) or public.
    pub kind: ShareKind,
    /// Current manifest revision.
    pub revision: u64,
    /// Whether a public share accepts writes.
    pub public_write: bool,
    /// Owner's verified user id when the receiver can resolve it. Used only for
    /// local bookkeeping — the owner enforces the ACL itself.
    pub owner_user_id: Option<String>,
}

/// One share as the domain layer exposes it (a parsed view of `ShareRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    /// Stable share id shared with peers on the wire.
    pub share_id: String,
    /// Verified user id of the owner.
    pub owner_user_id: String,
    /// Human-readable share name.
    pub name: String,
    /// Private (grant-gated) or public.
    pub kind: ShareKind,
    /// Whether a public share accepts writes.
    pub public_write: bool,
    /// Real local directory bound to this share, if any.
    pub local_root: Option<String>,
    /// Current manifest revision.
    pub revision: u64,
    /// Last accepted manifest, if one has been published.
    pub manifest: Option<ShareManifest>,
}

impl From<ShareRow> for Share {
    fn from(row: ShareRow) -> Self {
        Self {
            share_id: row.share_id,
            owner_user_id: row.owner_user_id,
            name: row.name,
            kind: ShareKind::from_i32(row.kind),
            public_write: row.public_write != 0,
            local_root: row.local_root,
            revision: u64::try_from(row.revision).unwrap_or_default(),
            manifest: row.manifest_json.as_deref().and_then(manifest_from_json),
        }
    }
}

/// One target user's grant on a share.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareGrant {
    /// Verified target user id.
    pub user_id: String,
    /// Read-only or read-write.
    pub mode: ShareMode,
}

impl From<ShareGrantRow> for ShareGrant {
    fn from(row: ShareGrantRow) -> Self {
        Self {
            user_id: row.user_id,
            mode: ShareMode::from_i32(row.mode),
        }
    }
}

/// Counters from applying a manifest to a bound local folder.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ApplyStats {
    /// Files written (including replacements).
    pub written: usize,
    /// Files removed because the new manifest no longer tracks them.
    pub removed: usize,
}

/// Validate and normalise a share display name.
///
/// # Errors
/// Returns an error for empty, oversized, or control-character names.
pub fn validate_share_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("share name must not be empty".to_string());
    }
    if trimmed.chars().count() > MAX_SHARE_NAME_LEN {
        return Err(format!(
            "share name too long (max {MAX_SHARE_NAME_LEN} characters)"
        ));
    }
    if trimmed.chars().any(char::is_control) {
        return Err("share name must not contain control characters".to_string());
    }
    Ok(trimmed.to_string())
}

/// Validate a relative path from a share manifest and return its canonical
/// forward-slash form. Absolute paths, `.`/`..` components, backslashes,
/// colons, control characters, and empty segments are rejected.
///
/// # Errors
/// Returns a human-readable reason the path is unsafe.
pub fn validate_share_path(path: &str) -> Result<String, String> {
    if path.is_empty() || path.len() > MAX_SHARE_PATH_LEN {
        return Err("share path is empty or too long".to_string());
    }
    if path.contains('\\') || path.contains(':') || path.chars().any(char::is_control) {
        return Err("share path contains forbidden characters".to_string());
    }
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("share path must name a file".to_string());
    }
    let mut parts: Vec<String> = Vec::new();
    for part in trimmed.split('/') {
        match part {
            "" | "." | ".." => return Err(format!("invalid path component: {part:?}")),
            value => parts.push(value.to_string()),
        }
    }
    Ok(parts.join("/"))
}

/// Resolve a validated relative path under `root`, rejecting any symlink in
/// the traversed components. The returned path is absolute because `root` is
/// canonicalised first.
///
/// # Errors
/// Returns an error if the path is invalid, `root` cannot be canonicalised, or
/// a component is a symlink.
fn resolve_share_path(root: &Path, relative: &str) -> color_eyre::Result<PathBuf> {
    let normalized = validate_share_path(relative).map_err(|error| eyre!("{error}"))?;
    let root_canonical = root
        .canonicalize()
        .wrap_err_with(|| format!("failed to resolve share root {}", root.display()))?;
    let mut candidate = root_canonical;
    for component in Path::new(&normalized).components() {
        let Component::Normal(part) = component else {
            return Err(eyre!("invalid share path component: {normalized:?}"));
        };
        candidate.push(part);
        match std::fs::symlink_metadata(&candidate) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(eyre!(
                    "share path traverses a symlink: {}",
                    candidate.display()
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).wrap_err_with(|| {
                    format!("failed to inspect share path {}", candidate.display())
                });
            }
        }
    }
    Ok(candidate)
}

/// Serialize a manifest for the `manifest_json` column.
///
/// Canonical encoder for both the persistence path and tests; unlike the
/// fail-soft reader, a write error must be reported rather than silently
/// dropping the manifest.
///
/// # Errors
/// Returns the `serde_json` error if serialization fails.
pub fn manifest_to_json(manifest: &ShareManifest) -> serde_json::Result<String> {
    serde_json::to_string(manifest)
}

/// Deserialize a manifest from its persisted JSON form.
#[must_use]
pub fn manifest_from_json(json: &str) -> Option<ShareManifest> {
    serde_json::from_str(json).ok()
}

/// Validate every path, size, content address and the entry cap of a manifest
/// received from a peer before it touches the DB or the filesystem.
///
/// # Errors
/// Returns an error on the first invalid field.
pub fn validate_manifest(manifest: &ShareManifest) -> color_eyre::Result<()> {
    if manifest.share_id.is_empty() {
        return Err(eyre!("share manifest has no id"));
    }
    validate_share_name(&manifest.name).map_err(|error| eyre!("{error}"))?;
    if manifest.entries.len() > MAX_SHARE_ENTRIES {
        return Err(eyre!(
            "share manifest has {} entries (max {MAX_SHARE_ENTRIES})",
            manifest.entries.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for entry in &manifest.entries {
        let path = validate_share_path(&entry.path).map_err(|error| eyre!("{error}"))?;
        if !seen.insert(path.clone()) {
            return Err(eyre!("duplicate share path: {path:?}"));
        }
        if !crate::files::is_valid_file_id(&entry.file_id) {
            return Err(eyre!("invalid content address in share entry {path:?}"));
        }
        if entry.size > crate::files::MAX_ATTACHMENT_SIZE {
            return Err(eyre!("share entry {path:?} exceeds the file size cap"));
        }
    }
    Ok(())
}

/// Convert a share into its advertised offer.
///
/// The share row is the authority for id/name/revision even when no manifest has
/// been published yet, so an ownerless remote stub can still be advertised.
#[must_use]
pub fn offer_for(
    share: &Share,
    kind: ShareKind,
    public_write: bool,
    owner_user_id: Option<&str>,
) -> ShareOffer {
    ShareOffer {
        share_id: share.share_id.clone(),
        name: share.name.clone(),
        kind,
        revision: share.revision,
        public_write,
        owner_user_id: owner_user_id.map(str::to_string),
    }
}

/// Prefix of the local-only marker for a share with no verifiable owner.
const UNKNOWN_OWNER_PREFIX: &str = "unknown:share:";

/// Owner id recorded when an offer or manifest has no verifiable owner.
///
/// Real account ids are `PeerId` strings (`user_id_from_account_pubkey`), so the
/// namespaced sentinel can never collide with one. It is local bookkeeping
/// only — the owner enforces its own ACL.
#[must_use]
pub fn unknown_owner_id(share_id: &str) -> String {
    format!("{UNKNOWN_OWNER_PREFIX}{share_id}")
}

/// Whether an owner id is the local-only "unknown owner" marker.
///
/// The marker must never leave this install: it identifies no real user, so
/// advertising it would make a peer store a meaningless owner id. Ownership
/// claims on the wire stay `None` until they can be verified.
#[must_use]
pub fn is_unknown_owner_id(owner_user_id: &str) -> bool {
    owner_user_id.starts_with(UNKNOWN_OWNER_PREFIX)
}

/// `public_write` is meaningful only for public shares. Normalising it at the
/// write boundary keeps a private share from ever granting anonymous writes.
#[must_use]
pub const fn effective_public_write(kind: ShareKind, public_write: bool) -> bool {
    matches!(kind, ShareKind::Public) && public_write
}

/// Create a new share row and return its domain view.
///
/// # Errors
/// Returns an error for an invalid name or a DB failure.
pub fn create_share(
    name: &str,
    owner_user_id: &str,
    kind: ShareKind,
    public_write: bool,
    local_root: Option<&str>,
) -> color_eyre::Result<Share> {
    create_share_with_id(
        &crate::fmt::gen_msg_id(),
        name,
        owner_user_id,
        kind,
        public_write,
        local_root,
    )
}

/// Insert a share with an explicit id (used when materialising a remote offer,
/// whose id is chosen by the owner and must survive verbatim).
fn create_share_with_id(
    share_id: &str,
    name: &str,
    owner_user_id: &str,
    kind: ShareKind,
    public_write: bool,
    local_root: Option<&str>,
) -> color_eyre::Result<Share> {
    let name = validate_share_name(name).map_err(|error| eyre!("{error}"))?;
    let row = NewShare {
        share_id: share_id.to_string(),
        owner_user_id: owner_user_id.to_string(),
        name,
        kind: kind.as_i32(),
        public_write: i32::from(effective_public_write(kind, public_write)),
        local_root: local_root.map(str::to_string),
        revision: 0,
        manifest_json: None,
    };
    let conn = &mut crate::sqlite_connect()?;
    diesel::insert_into(shares::table)
        .values(&row)
        .execute(conn)
        .wrap_err("failed to create share")?;
    get_share(share_id)?.ok_or_else(|| eyre!("share vanished after insert"))
}

/// Look up one share by id.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn get_share(share_id: &str) -> color_eyre::Result<Option<Share>> {
    let conn = &mut crate::sqlite_connect()?;
    get_share_conn(conn, share_id)
}

/// Look up one share using an existing connection.
///
/// Keeping the read on the caller's connection lets permission checks inside a
/// write transaction observe the same committed state as the write itself.
fn get_share_conn(
    conn: &mut diesel::SqliteConnection,
    share_id: &str,
) -> color_eyre::Result<Option<Share>> {
    let row = shares::table
        .filter(shares::share_id.eq(share_id))
        .select(ShareRow::as_select())
        .first::<ShareRow>(conn)
        .optional()
        .wrap_err("failed to load share")?;
    Ok(row.map(Share::from))
}

/// List every share known to this install (owned and accepted).
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn list_shares() -> color_eyre::Result<Vec<Share>> {
    let conn = &mut crate::sqlite_connect()?;
    let rows = shares::table
        .select(ShareRow::as_select())
        .load::<ShareRow>(conn)
        .wrap_err("failed to list shares")?;
    Ok(rows.into_iter().map(Share::from).collect())
}

/// Bind (or clear) the real local directory backing a share.
///
/// # Errors
/// Returns an error if the share is unknown or the DB update fails.
pub fn set_local_root(share_id: &str, local_root: Option<&str>) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    let changed = diesel::update(shares::table.filter(shares::share_id.eq(share_id)))
        .set(shares::local_root.eq(local_root))
        .execute(conn)
        .wrap_err("failed to set share root")?;
    if changed == 0 {
        return Err(eyre!("unknown share: {share_id}"));
    }
    Ok(())
}

/// Change a locally owned share's visibility and public-write flag.
///
/// `expected_owner_user_id` is supplied by the caller's resolved local user so
/// accepted peer shares cannot be reclassified by this install. `public_write`
/// is normalised through [`effective_public_write`], making anonymous writes
/// impossible on a private share.
///
/// # Errors
/// Returns an error if the share is unknown, not owned by the caller, or the
/// DB write fails.
pub fn set_visibility(
    share_id: &str,
    expected_owner_user_id: &str,
    kind: ShareKind,
    public_write: bool,
) -> color_eyre::Result<Share> {
    let mut share = get_share(share_id)?.ok_or_else(|| eyre!("unknown share: {share_id}"))?;
    if share.owner_user_id != expected_owner_user_id {
        return Err(eyre!("share is not owned by the local user"));
    }
    let public_write = effective_public_write(kind, public_write);
    let conn = &mut crate::sqlite_connect()?;
    diesel::update(shares::table.filter(shares::share_id.eq(share_id)))
        .set((
            shares::kind.eq(kind.as_i32()),
            shares::public_write.eq(i32::from(public_write)),
        ))
        .execute(conn)
        .wrap_err("failed to set share visibility")?;
    share.kind = kind;
    share.public_write = public_write;
    Ok(share)
}

/// Set a target user's grant on a private share (upsert).
///
/// # Errors
/// Returns an error if the share is unknown or the DB write fails.
pub fn set_grant(share_id: &str, user_id: &str, mode: ShareMode) -> color_eyre::Result<()> {
    if get_share(share_id)?.is_none() {
        return Err(eyre!("unknown share: {share_id}"));
    }
    let grant = NewShareGrant {
        share_id: share_id.to_string(),
        user_id: user_id.to_string(),
        mode: mode.as_i32(),
    };
    let conn = &mut crate::sqlite_connect()?;
    diesel::insert_into(share_grants::table)
        .values(&grant)
        .on_conflict((share_grants::share_id, share_grants::user_id))
        .do_update()
        .set(share_grants::mode.eq(mode.as_i32()))
        .execute(conn)
        .wrap_err("failed to set share grant")?;
    Ok(())
}

/// Remove a target user's grant (a no-op when it does not exist).
///
/// # Errors
/// Returns an error if the DB delete fails.
pub fn remove_grant(share_id: &str, user_id: &str) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    diesel::delete(
        share_grants::table
            .filter(share_grants::share_id.eq(share_id))
            .filter(share_grants::user_id.eq(user_id)),
    )
    .execute(conn)
    .wrap_err("failed to remove share grant")?;
    Ok(())
}

/// List all grants on a share.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn list_grants(share_id: &str) -> color_eyre::Result<Vec<ShareGrant>> {
    let conn = &mut crate::sqlite_connect()?;
    let rows = share_grants::table
        .filter(share_grants::share_id.eq(share_id))
        .select(ShareGrantRow::as_select())
        .load::<ShareGrantRow>(conn)
        .wrap_err("failed to list share grants")?;
    Ok(rows.into_iter().map(ShareGrant::from).collect())
}

/// All grants grouped by share id in one query. Read-only frontends render a
/// grant summary for every share, so this avoids one `list_grants` call per
/// share (and its per-call connection).
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn list_all_grants() -> color_eyre::Result<HashMap<String, Vec<ShareGrant>>> {
    let conn = &mut crate::sqlite_connect()?;
    let rows = share_grants::table
        .select(ShareGrantRow::as_select())
        .load::<ShareGrantRow>(conn)
        .wrap_err("failed to list share grants")?;
    let mut by_share: HashMap<String, Vec<ShareGrant>> = HashMap::new();
    for row in rows {
        by_share
            .entry(row.share_id.clone())
            .or_default()
            .push(ShareGrant::from(row));
    }
    Ok(by_share)
}

/// Delete a share and its grants/entries.
///
/// The bound local folder and the content-addressed blobs are left untouched:
/// deleting a share never deletes a user's files. Any in-flight apply for the
/// share is dropped too, so a late blob cannot materialise files for a share
/// that no longer exists.
///
/// # Errors
/// Returns an error if the DB delete fails.
pub fn delete_share(share_id: &str) -> color_eyre::Result<()> {
    let conn = &mut crate::sqlite_connect()?;
    conn.transaction::<(), color_eyre::Report, _>(|conn| {
        diesel::delete(share_entries::table.filter(share_entries::share_id.eq(share_id)))
            .execute(conn)
            .wrap_err("failed to delete share entries")?;
        diesel::delete(share_grants::table.filter(share_grants::share_id.eq(share_id)))
            .execute(conn)
            .wrap_err("failed to delete share grants")?;
        diesel::delete(shares::table.filter(shares::share_id.eq(share_id)))
            .execute(conn)
            .wrap_err("failed to delete share")?;
        Ok(())
    })?;
    abandon_pending_for_share(share_id);
    Ok(())
}

/// Encode a validated manifest into the rows and cached JSON used by both
/// [`save_manifest`] and [`accept_submitted_manifest`].
fn prepare_manifest_rows(
    share_id: &str,
    manifest: &ShareManifest,
) -> color_eyre::Result<(Vec<NewShareEntry>, String, i64)> {
    validate_manifest(manifest)?;
    if manifest.share_id != share_id {
        return Err(eyre!("manifest id does not match the share"));
    }
    let json = manifest_to_json(manifest).wrap_err("failed to encode share manifest")?;
    let revision =
        i64::try_from(manifest.revision).map_err(|_| eyre!("share revision out of range"))?;
    let entries = manifest
        .entries
        .iter()
        .map(|entry| NewShareEntry {
            share_id: share_id.to_string(),
            path: entry.path.clone(),
            file_id: entry.file_id.clone(),
            size: i64::try_from(entry.size).unwrap_or(i64::MAX),
            mime: entry.mime.clone(),
        })
        .collect();
    Ok((entries, json, revision))
}

/// Replace a share's entry rows, revision and cached JSON inside an open
/// transaction. The share row must already exist (the FK requires it).
fn write_manifest_rows(
    conn: &mut diesel::SqliteConnection,
    share_id: &str,
    entries: &[NewShareEntry],
    json: &str,
    revision: i64,
) -> color_eyre::Result<()> {
    diesel::delete(share_entries::table.filter(share_entries::share_id.eq(share_id)))
        .execute(conn)
        .wrap_err("failed to clear old share entries")?;
    for entry in entries {
        diesel::insert_into(share_entries::table)
            .values(entry)
            .execute(conn)
            .wrap_err("failed to insert share entry")?;
    }
    diesel::update(shares::table.filter(shares::share_id.eq(share_id)))
        .set((
            shares::revision.eq(revision),
            shares::manifest_json.eq(Some(json)),
            shares::updated_at.eq(chrono::Utc::now().naive_utc()),
        ))
        .execute(conn)
        .wrap_err("failed to update share manifest")?;
    Ok(())
}

/// Persist a received manifest: replace the entry rows, bump the revision and
/// refresh the cached JSON.
///
/// The share must exist and the manifest revision must not move backwards — a
/// replayed or stale peer response must never roll local state back. An equal
/// revision is accepted only as an idempotent re-fetch of the cached manifest:
/// when cached JSON exists it must be byte-identical to this manifest.
///
/// # Errors
/// Returns an error for an invalid/mismatched manifest, an unknown share, a
/// rollback or same-revision conflict, or a DB failure.
pub fn save_manifest(share_id: &str, manifest: &ShareManifest) -> color_eyre::Result<()> {
    let (entries, json, revision) = prepare_manifest_rows(share_id, manifest)?;
    let conn = &mut crate::sqlite_connect()?;
    conn.transaction::<(), color_eyre::Report, _>(|conn| {
        let current: Option<(i64, Option<String>)> = shares::table
            .filter(shares::share_id.eq(share_id))
            .select((shares::revision, shares::manifest_json))
            .first(conn)
            .optional()
            .wrap_err("failed to check share revision")?;
        let (current, cached_json) = current.ok_or_else(|| eyre!("unknown share: {share_id}"))?;
        if revision < current {
            return Err(eyre!(
                "manifest revision {revision} would roll back share {share_id} (current {current})"
            ));
        }
        if revision == current
            && cached_json
                .as_deref()
                .is_some_and(|cached| cached != json.as_str())
        {
            return Err(eyre!(
                "manifest revision {revision} conflicts with cached content for share {share_id}"
            ));
        }
        write_manifest_rows(conn, share_id, &entries, &json, revision)
    })
}

/// Load a share's current manifest from its cached JSON.
///
/// # Errors
/// Returns an error if the share is unknown or its cached JSON is corrupt.
pub fn load_manifest(share_id: &str) -> color_eyre::Result<ShareManifest> {
    let share = get_share(share_id)?.ok_or_else(|| eyre!("unknown share: {share_id}"))?;
    share
        .manifest
        .ok_or_else(|| eyre!("share {share_id} has no published manifest"))
}

/// Content addresses referenced by any live share. Used by attachment GC so a
/// manifest's blobs are never swept as orphans.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn referenced_file_ids() -> color_eyre::Result<std::collections::HashSet<String>> {
    use diesel::QueryDsl as _;
    use diesel::RunQueryDsl as _;
    let conn = &mut crate::sqlite_connect()?;
    let ids = share_entries::table
        .select(share_entries::file_id)
        .load::<String>(conn)
        .wrap_err("failed to scan share entries")?;
    Ok(ids.into_iter().collect())
}

/// Effective permission for a requester peer on a loaded share, resolved on
/// `conn`, or `None` when denied.
///
/// Fail closed everywhere: an unbound peer, unknown enum value, or any DB error
/// yields `None`. Passing the caller's connection lets a write transaction see
/// the same grant state it is about to commit alongside.
fn permission_for_share(
    conn: &mut diesel::SqliteConnection,
    share: &Share,
    peer_id: &str,
) -> Option<ShareMode> {
    let requester = crate::identity::find_user_id_for_peer_conn(conn, peer_id).ok()?;
    if requester.as_deref() == Some(share.owner_user_id.as_str()) {
        return Some(ShareMode::ReadWrite);
    }
    if share.kind == ShareKind::Public {
        return Some(if share.public_write {
            ShareMode::ReadWrite
        } else {
            ShareMode::ReadOnly
        });
    }
    let mode = share_grants::table
        .filter(share_grants::share_id.eq(&share.share_id))
        .filter(share_grants::user_id.eq(requester?))
        .select(share_grants::mode)
        .first::<i32>(conn)
        .optional()
        .ok()??;
    Some(ShareMode::from_i32(mode))
}

/// Effective permission for a requester peer on a share, or `None` when denied.
fn permission_for(share_id: &str, peer_id: &str) -> Option<ShareMode> {
    let conn = &mut crate::sqlite_connect().ok()?;
    let share = get_share_conn(conn, share_id).ok()??;
    permission_for_share(conn, &share, peer_id)
}

/// Whether `peer_id` may fetch this share (manifest plus blobs).
#[must_use]
pub fn can_read(share_id: &str, peer_id: &str) -> bool {
    permission_for(share_id, peer_id).is_some()
}

/// Whether `peer_id` may submit a new snapshot for this share.
#[must_use]
pub fn can_write(share_id: &str, peer_id: &str) -> bool {
    permission_for(share_id, peer_id) == Some(ShareMode::ReadWrite)
}

/// Build an offer for a share, or `None` if the share is unknown.
///
/// # Errors
/// Returns an error if the DB query fails.
pub fn offer_for_share(share_id: &str) -> color_eyre::Result<Option<ShareOffer>> {
    let Some(share) = get_share(share_id)? else {
        return Ok(None);
    };
    let owner =
        (!is_unknown_owner_id(&share.owner_user_id)).then_some(share.owner_user_id.as_str());
    Ok(Some(offer_for(
        &share,
        share.kind,
        share.public_write,
        owner,
    )))
}

/// Materialise a received offer as a local share row.
///
/// The row gives a later manifest request somewhere to land. Calls are
/// idempotent by `share_id`. An offer is untrusted (it came from a peer): for a
/// share we own, have bound to a folder, or already hold a manifest for, it is
/// a no-op — it can never rewrite our ACL, root or revision. For a bare remote
/// stub, only the display name is refreshed.
///
/// # Errors
/// Returns an error if the offer is invalid or the DB write fails.
pub fn accept_offer(offer: &ShareOffer, local_root: Option<&str>) -> color_eyre::Result<Share> {
    let name = validate_share_name(&offer.name).map_err(|error| eyre!("{error}"))?;
    if offer.share_id.is_empty() {
        return Err(eyre!("share offer has no id"));
    }
    if let Some(existing) = get_share(&offer.share_id)? {
        let locally_owned = crate::identity::my_user_id()
            .ok()
            .flatten()
            .is_some_and(|id| id == existing.owner_user_id);
        if existing.local_root.is_some() || existing.manifest.is_some() || locally_owned {
            return Ok(existing);
        }
        // A pure remote stub: refresh only the cosmetic name, never `kind`,
        // `public_write`, `revision`, root or owner (all untrusted in an offer).
        let conn = &mut crate::sqlite_connect()?;
        diesel::update(shares::table.filter(shares::share_id.eq(&offer.share_id)))
            .set((
                shares::name.eq(&name),
                shares::updated_at.eq(chrono::Utc::now().naive_utc()),
            ))
            .execute(conn)
            .wrap_err("failed to refresh share offer")?;
        return Ok(Share { name, ..existing });
    }
    let owner_user_id = offer
        .owner_user_id
        .clone()
        .unwrap_or_else(|| unknown_owner_id(&offer.share_id));
    create_share_with_id(
        &offer.share_id,
        &name,
        &owner_user_id,
        offer.kind,
        offer.public_write,
        local_root,
    )
}

/// Snapshot a local directory into the blob store and return its entries.
///
/// Symlinks are skipped (never followed), directories are walked recursively,
/// and every regular file is stored content-addressed via
/// [`crate::files::store_outgoing_bytes`]. The result is sorted by path so two
/// identical trees always produce byte-identical manifests.
///
/// # Errors
/// Returns an error if the root is unreadable, a path is unsafe, the file cap
/// is exceeded, or a blob cannot be stored.
pub fn snapshot_tree(root: &Path) -> color_eyre::Result<Vec<ShareEntry>> {
    let root_canonical = root
        .canonicalize()
        .wrap_err_with(|| format!("failed to resolve share root {}", root.display()))?;
    let mut entries = Vec::new();
    let mut stack = vec![root_canonical.clone()];
    while let Some(dir) = stack.pop() {
        let mut children = Vec::new();
        for item in std::fs::read_dir(&dir)
            .wrap_err_with(|| format!("failed to read share directory {}", dir.display()))?
        {
            let item = item.wrap_err("failed to read share directory entry")?;
            let file_type = item.file_type().wrap_err("failed to inspect share file")?;
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                children.push(item.path());
            } else if file_type.is_file() {
                let path = item.path();
                let relative = path
                    .strip_prefix(&root_canonical)
                    .wrap_err("share file escaped its root")?
                    .to_string_lossy()
                    .into_owned();
                let relative = validate_share_path(&relative).map_err(|error| eyre!("{error}"))?;
                let bytes = std::fs::read(&path)
                    .wrap_err_with(|| format!("failed to read share file {}", path.display()))?;
                let name = Path::new(&relative).file_name().map_or_else(
                    || relative.clone(),
                    |part| part.to_string_lossy().into_owned(),
                );
                let attachment = crate::files::store_outgoing_bytes(&name, &bytes)?;
                entries.push(ShareEntry {
                    path: relative,
                    file_id: attachment.file_id,
                    size: attachment.size,
                    mime: attachment.mime,
                });
            }
        }
        stack.extend(children);
        if entries.len() > MAX_SHARE_ENTRIES {
            return Err(eyre!("share tree exceeds {MAX_SHARE_ENTRIES} files"));
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

/// Install one blob at `target` through a staged temporary file, so an
/// interrupted write never leaves a half-written tracked file. The staging file
/// is removed on any failure — otherwise a failed rename would strand a `.part`
/// file inside the user's synced folder forever (blob GC only sweeps the blob
/// store, not share roots).
fn install_share_file(target: &Path, bytes: &[u8]) -> color_eyre::Result<()> {
    let temp = target.with_extension(format!("{}.part", crate::fmt::gen_msg_id()));
    std::fs::write(&temp, bytes)
        .wrap_err_with(|| format!("failed to stage share file {}", temp.display()))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp);
        })?;
    std::fs::rename(&temp, target)
        .wrap_err_with(|| format!("failed to install share file {}", target.display()))
        .inspect_err(|_| {
            let _ = std::fs::remove_file(&temp);
        })
}

/// Apply a manifest to the bound local folder, writing all blobs and removing
/// files that the previous manifest tracked but the new one no longer does.
///
/// This is the only path that mutates a synced local folder. Every write goes
/// through the canonical path validator (no absolute paths, no `..`, no symlinks), and
/// a blob's bytes are re-hashed before they are written.
///
/// # Errors
/// Returns an error if the manifest is invalid, the root is unusable, a blob is
/// missing/corrupt, or a filesystem operation fails.
pub fn apply_manifest(
    share_id: &str,
    root: &Path,
    previous: Option<&ShareManifest>,
    next: &ShareManifest,
) -> color_eyre::Result<ApplyStats> {
    validate_manifest(next)?;
    if next.share_id != share_id {
        return Err(eyre!("manifest id does not match the share"));
    }
    std::fs::create_dir_all(root)
        .wrap_err_with(|| format!("failed to create share root {}", root.display()))?;

    let next_paths: BTreeSet<&str> = next
        .entries
        .iter()
        .map(|entry| entry.path.as_str())
        .collect();
    let mut stats = ApplyStats::default();
    if let Some(previous) = previous {
        let mut removed_paths: BTreeSet<String> = BTreeSet::new();
        for entry in &previous.entries {
            if let Ok(path) = validate_share_path(&entry.path)
                && !next_paths.contains(path.as_str())
            {
                removed_paths.insert(path);
            }
        }
        for path in removed_paths {
            let target = resolve_share_path(root, &path)?;
            match std::fs::symlink_metadata(&target) {
                Ok(metadata) if metadata.file_type().is_file() => {
                    std::fs::remove_file(&target).wrap_err_with(|| {
                        format!("failed to remove share file {}", target.display())
                    })?;
                    stats.removed = stats.removed.saturating_add(1);
                }
                Ok(_) => {
                    return Err(eyre!(
                        "refusing to remove non-regular share file {}",
                        target.display()
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).wrap_err("failed to inspect share file"),
            }
        }
    }

    for entry in &next.entries {
        let target = resolve_share_path(root, &entry.path)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).wrap_err_with(|| {
                format!("failed to create share directory {}", parent.display())
            })?;
        }
        let blob = crate::files::store_path_for(&entry.file_id)?;
        let bytes = std::fs::read(&blob)
            .wrap_err_with(|| format!("failed to read share blob {}", blob.display()))?;
        if crate::files::hash_bytes(&bytes) != entry.file_id {
            return Err(eyre!("share blob {} failed its hash check", entry.file_id));
        }
        install_share_file(&target, &bytes)?;
        stats.written = stats.written.saturating_add(1);
    }
    Ok(stats)
}

/// Accept a submitted snapshot after checking permission and the base revision.
/// On success the manifest is persisted and applied to the share's bound local
/// folder (when one is set).
///
/// The permission check and the base-revision check plus the write happen in a
/// single transaction, so a concurrent submit cannot slip between them and two
/// writers cannot both claim the same base. The folder is applied only after
/// the DB commit succeeds — never before a potentially failing write.
///
/// # Errors
/// Returns an error when the ACL denies the write, the revision is stale, the
/// manifest is invalid, or persistence/application fails.
pub fn accept_submitted_manifest(
    share_id: &str,
    peer_id: &str,
    base_revision: u64,
    manifest: &ShareManifest,
) -> color_eyre::Result<ApplyStats> {
    if manifest.share_id != share_id {
        return Err(eyre!("manifest id does not match the share"));
    }
    let next_revision = base_revision.saturating_add(1);
    let mut next = manifest.clone();
    next.revision = next_revision;
    let (entries, json, revision) = prepare_manifest_rows(share_id, &next)?;

    // Atomic check-and-write: resolve the ACL and verify the base revision on
    // the same connection that persists the new manifest. A grant revoked
    // concurrently with this submission is therefore visible before the write.
    let (previous, local_root) = {
        let conn = &mut crate::sqlite_connect()?;
        conn.transaction::<(Option<ShareManifest>, Option<String>), color_eyre::Report, _>(
            |conn| {
                let share = shares::table
                    .filter(shares::share_id.eq(share_id))
                    .select(ShareRow::as_select())
                    .first::<ShareRow>(conn)
                    .optional()
                    .wrap_err("failed to load share")?
                    .ok_or_else(|| eyre!("unknown share: {share_id}"))?;
                let share = Share::from(share);
                if permission_for_share(conn, &share, peer_id) != Some(ShareMode::ReadWrite) {
                    return Err(eyre!("peer {peer_id} may not write share {share_id}"));
                }
                let expected = share.revision;
                if base_revision != expected {
                    return Err(eyre!(
                        "stale share revision {base_revision} (current {expected})"
                    ));
                }
                write_manifest_rows(conn, share_id, &entries, &json, revision)?;
                Ok((share.manifest, share.local_root))
            },
        )?
    };

    // The manifest is durable now; apply it to the bound folder if there is one.
    local_root.map_or_else(
        || Ok(ApplyStats::default()),
        |root| apply_manifest(share_id, Path::new(&root), previous.as_ref(), &next),
    )
}

/// A manifest waiting for the blobs it references to finish downloading.
struct PendingApply {
    /// Peer that served the manifest (and is serving its blobs).
    peer_id: String,
    /// Manifest we were on before this one, for safe deletion tracking.
    previous: Option<ShareManifest>,
    /// Manifest to apply once every blob is present.
    manifest: ShareManifest,
    /// Content addresses still missing from the local store.
    missing: BTreeSet<String>,
}

fn pending_applies() -> &'static Mutex<BTreeMap<String, PendingApply>> {
    static PENDING: OnceLock<Mutex<BTreeMap<String, PendingApply>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Summary of a deferred manifest application, surfaced as a swarm event so
/// frontends can refresh the share's local folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShareApplyResult {
    /// Peer that supplied the last missing blob.
    pub peer_id: String,
    /// Share that was applied.
    pub share_id: String,
    /// Revision now installed in the local folder.
    pub revision: u64,
    /// Files written and removed by the apply.
    pub stats: ApplyStats,
}

/// Outcome of registering a received manifest for application.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct QueueApplyResult {
    /// Content addresses the caller must fetch before the manifest can apply.
    pub missing: Vec<String>,
    /// Manifests applied immediately because every blob was already local.
    pub applied: Vec<ShareApplyResult>,
}

/// Apply one fully-downloaded manifest to its bound folder, if the share still
/// exists, is still bound, and is at exactly the revision being applied.
///
/// A newer manifest may have been saved while we were downloading; never apply
/// an older snapshot over it.
fn apply_ready(apply: PendingApply) -> Option<ShareApplyResult> {
    let (root, current_revision) = get_share(&apply.manifest.share_id)
        .ok()
        .flatten()
        .and_then(|share| share.local_root.map(|root| (root, share.revision)))?;
    if apply.manifest.revision != current_revision {
        return None;
    }
    match apply_manifest(
        &apply.manifest.share_id,
        Path::new(&root),
        apply.previous.as_ref(),
        &apply.manifest,
    ) {
        Ok(stats) => Some(ShareApplyResult {
            peer_id: apply.peer_id,
            share_id: apply.manifest.share_id,
            revision: apply.manifest.revision,
            stats,
        }),
        Err(error) => {
            p2plog_debug(format!(
                "failed to apply share {}: {error}",
                apply.manifest.share_id
            ));
            None
        }
    }
}

/// Register a received manifest for application once all of its blobs are
/// locally available.
///
/// The manifest should already be persisted (so GC sees its blob references);
/// `previous` is the manifest that was stored before it, captured by the caller
/// ahead of that write. A later manifest for the same share supersedes an
/// earlier pending one, so the folder always converges on the newest revision.
///
/// When every blob is already local (a rename/delete-only snapshot or a repeat
/// fetch) the manifest is applied immediately rather than waiting forever for
/// a blob-arrival notification that will never come.
#[must_use]
pub fn queue_apply(
    peer_id: String,
    previous: Option<ShareManifest>,
    manifest: ShareManifest,
) -> QueueApplyResult {
    let missing: BTreeSet<String> = manifest
        .entries
        .iter()
        .filter(|entry| crate::files::file_path(&entry.file_id).is_none())
        .map(|entry| entry.file_id.clone())
        .collect();
    let mut pending = pending_applies()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    // A later manifest supersedes any earlier pending one for this share.
    pending.remove(&manifest.share_id);
    if missing.is_empty() {
        drop(pending);
        let apply = PendingApply {
            peer_id,
            previous,
            manifest,
            missing,
        };
        return QueueApplyResult {
            missing: Vec::new(),
            applied: apply_ready(apply).into_iter().collect(),
        };
    }
    let ids = missing.iter().cloned().collect();
    pending.insert(
        manifest.share_id.clone(),
        PendingApply {
            peer_id,
            previous,
            manifest,
            missing,
        },
    );
    drop(pending);
    QueueApplyResult {
        missing: ids,
        applied: Vec::new(),
    }
}

/// Note that a blob arrived and apply every share that was waiting on it.
/// `peer_id` is the peer that delivered the blob, attributed to any apply it
/// completes.
///
/// A share whose local root is not bound yet is dropped from the pending set:
/// there is nowhere to apply it, and the manifest is already persisted for a
/// later explicit refresh.
#[must_use]
pub fn note_blob_available(file_id: &str, peer_id: &str) -> Vec<ShareApplyResult> {
    let completed: Vec<PendingApply> = {
        let mut pending = pending_applies()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let mut ready = Vec::new();
        for apply in pending.values_mut() {
            apply.missing.remove(file_id);
            if apply.missing.is_empty() {
                ready.push(apply.manifest.share_id.clone());
            }
        }
        let completed: Vec<PendingApply> = ready
            .into_iter()
            .filter_map(|share_id| pending.remove(&share_id))
            .collect();
        drop(pending);
        completed
    };

    completed
        .into_iter()
        .map(|apply| PendingApply {
            peer_id: peer_id.to_string(),
            ..apply
        })
        .filter_map(apply_ready)
        .collect()
}

/// Drop pending applications whose serving peer disconnected. Their blobs can
/// no longer arrive, so leaving them queued would wedge future revisions.
///
/// This is deliberately coarse: it drops every pending apply from that peer,
/// not just ones tied to a specific blob. Pending applies are per-share and
/// short-lived, so a reconnect re-queues them on the next manifest.
pub fn abandon_pending_from(peer_id: &str) {
    let mut pending = pending_applies()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    pending.retain(|_, apply| apply.peer_id != peer_id);
}

/// Drop a single share's pending apply (used when the share is deleted).
fn abandon_pending_for_share(share_id: &str) {
    let mut pending = pending_applies()
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    pending.remove(share_id);
}

#[cfg(test)]
#[path = "../tests/unit/unit_shares.rs"]
mod tests;
