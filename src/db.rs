//! Database connection and identity key management
//!
//! This module manages `SQLite` connections. To avoid connection overhead,
//! a single connection is established and reused for the lifetime of the application.
//! While this is not suitable for high-concurrency scenarios, it's appropriate for
//! this single-threaded TUI application.
//!
//! For future multi-threaded use, consider using r2d2 connection pooling.

use crate::generated::columns::SCHEMA_ENTRIES;
use crate::generated::models_queryable::Identity;
use crate::generated::schema::identities::dsl::identities;
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, QueryDsl, RunQueryDsl as _, SelectableHelper as _, SqliteConnection,
};
use diesel_migrations::MigrationHarness;
use std::cell::RefCell;
use std::sync::OnceLock;

pub use crate::MIGRATIONS;

thread_local! {
    /// Database path for the current thread.
    ///
    /// Each test (and each runtime task) runs on its own OS thread, so storing the
    /// URL here — instead of a process-global variable or a `DATABASE_URL` env var —
    /// keeps callers fully isolated: no test can observe or stomp another test's
    /// database. The FRB/mobile entry points set this once on their worker thread.
    static DB_URL: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Process-wide primary database URL.
///
/// `get_database_url` is called from many threads (the tokio swarm handler, the
/// FRB worker thread, etc.) and tokio tasks migrate between worker threads, so a
/// thread-local URL alone would force every thread to independently re-run DB
/// selection and lock its own `sqlite_N.db`. We cache the selected URL here so the
/// application reuses a single database across all threads.
#[cfg(not(any(test, feature = "test-utils")))]
static PRIMARY_DB_URL: OnceLock<std::sync::Mutex<Option<String>>> = OnceLock::new();

#[cfg(not(any(test, feature = "test-utils")))]
fn primary_db_url() -> Option<String> {
    PRIMARY_DB_URL
        .get()
        .and_then(|m| m.lock().ok().and_then(|g| g.clone()))
}

#[cfg(not(any(test, feature = "test-utils")))]
fn set_primary_db_url(url: String) {
    let slot = PRIMARY_DB_URL.get_or_init(|| std::sync::Mutex::new(None));
    if let Ok(mut g) = slot.lock() {
        *g = Some(url);
    }
}

/// Override the database path for the current thread.
///
/// Preferred over any global/env mechanism: it cannot race with other threads
/// and keeps tests independent of each other.
#[cfg(any(test, feature = "test-utils"))]
pub fn set_db_url(url: &str) {
    DB_URL.with(|u| *u.borrow_mut() = Some(url.to_string()));
    crate::nickname::clear_display_names();
}

/// Production (non-test) override: set both the thread-local and the shared
/// primary URL so every thread in the process converges on the same database.
///
/// Claims a PID lock file for the requested path (and, if another live process
/// already owns it, diverges to a process-unique variant) so two app instances
/// — e.g. the Flutter desktop app and the TUI — never open the same database,
/// share one identity/peer id, or contend on the same SQLite file.
#[cfg(not(any(test, feature = "test-utils")))]
pub fn set_db_url(url: &str) {
    let actual = claim_db_url_with_lock(url);
    DB_URL.with(|u| *u.borrow_mut() = Some(actual.clone()));
    set_primary_db_url(actual);
    crate::nickname::clear_display_names();
}

#[cfg(any(test, feature = "test-utils"))]
pub fn reset_db_url() {
    DB_URL.with(|u| *u.borrow_mut() = None);
    crate::nickname::clear_display_names();
}

/// Production (non-test) reset: drop the thread-local URL and release the
/// lock file so another instance may reclaim the database.
#[cfg(not(any(test, feature = "test-utils")))]
pub fn reset_db_url() {
    if let Some(url) = DB_URL.with(|u| u.borrow().clone()) {
        let _ = std::fs::remove_file(format!("{url}.lock"));
    }
    DB_URL.with(|u| *u.borrow_mut() = None);
    crate::nickname::clear_display_names();
}

/// Shared lock for serialising test DB setup/teardown.
#[cfg(any(test, feature = "test-utils"))]
pub fn shared_db_test_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

thread_local! {
    /// Per-thread count of test-DB auto-selections, so each selection (after a
    /// `reset_db_url`) yields a fresh file rather than reusing stale data.
    #[cfg(any(test, feature = "test-utils"))]
    static TEST_DB_SELECTION_COUNTER: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// The auto-selected test database path for the current thread.
///
/// Tests never share a database across processes or threads: the name embeds
/// the process id and this thread's unique id, so the lib, bin, and integration
/// test targets racing under `cargo test --all-features` — and any test threads
/// within them — each get their own `SQLite` file in the system temp dir. This
/// eliminates the old behaviour of scanning the shared working directory for
/// `.db`/`.lock` files, which let parallel processes delete each other's live
/// lock files and then select the same database. The per-thread counter keeps
/// each fresh selection (after `reset_db_url`) on a brand-new file, so no test
/// ever sees rows left over from an earlier test on the same thread.
#[cfg(any(test, feature = "test-utils"))]
fn unique_test_db_path() -> String {
    let pid = std::process::id();
    let tid = format!("{:?}", std::thread::current().id());
    let n = TEST_DB_SELECTION_COUNTER.with(|c| {
        let current = c.get();
        c.set(current.saturating_add(1));
        current
    });
    std::env::temp_dir()
        .join(format!("p2p-chat-test-{pid}-{tid}-{n}.db"))
        .to_string_lossy()
        .into_owned()
}

/// Establish a connection to the `SQLite` database and run pending migrations.
///
/// Uses the per-thread database URL set via [`set_db_url`] (or the default
/// auto-selected database when none is set). Because the URL is per-thread,
/// each test (and each runtime task) opens its own database file, so concurrent
/// connections never race on the same migrations table.
///
/// # Returns
/// A new `SqliteConnection` with all migrations applied, or an error if connection/migration fails
///
/// # Errors
/// - If database file cannot be found or created
/// - If migrations fail to execute
pub fn sqlite_connect() -> color_eyre::Result<SqliteConnection> {
    static PANIC_HOOK_SET: OnceLock<()> = OnceLock::new();

    let db_path = get_database_url();

    // Register cleanup on panic (to ensure the lock file is released on crash)
    let () = PANIC_HOOK_SET.get_or_init(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if let Some(db_path) = DB_URL.with(|u| u.borrow().clone()) {
                let lock_path = format!("{db_path}.lock");
                let _ = std::fs::remove_file(&lock_path);
                crate::logging::p2plog_debug(format!("[DB] released lock on panic: {lock_path}"));
            }
            prev(info);
        }));
    });

    // Ensure the parent directory exists before opening the database.
    // SQLite creates missing files but not missing directories, which breaks
    // first launch on Android where the app `databases` folder doesn't exist yet.
    if let Some(parent) = std::path::Path::new(&db_path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .wrap_err_with(|| format!("Error creating directory {}", parent.display()))?;
    }

    let mut conn = SqliteConnection::establish(&db_path)
        .wrap_err_with(|| format!("Error connecting to {db_path}"))?;

    // Set busy timeout so we retry instead of getting "database is locked"
    diesel::sql_query("PRAGMA busy_timeout = 5000")
        .execute(&mut conn)
        .ok();

    // Run migrations first to create tables, then ensure columns that may be
    // missing from older schemas.
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| eyre!(format!("Error executing migrations on {db_path}: {e}")))?;

    ensure_columns(&mut conn);

    Ok(conn)
}

/// Initialize the database once at startup.
/// Logs the database path and local peer ID.
/// Returns the connection for use by the application.
///
/// This should be called once at application startup before any other DB operations.
///
/// # Errors
/// Returns an error if the database cannot be connected to or migrations fail.
pub fn init_database() -> color_eyre::Result<SqliteConnection> {
    let db_path = get_database_url();
    let conn = sqlite_connect()?;

    // Log startup info once
    crate::logging::p2plog_info(format!("[Startup] Database: {db_path}"));
    if let Ok(id) = get_local_peer_id() {
        crate::logging::p2plog_info(format!("[Startup] Local peer ID: {id}"));
    }

    Ok(conn)
}

/// Ensures all columns exist in the database schema.
/// This is needed because `SQLite` doesn't support "ADD COLUMN IF NOT EXISTS".
/// We check each table/column pair and add missing ones.
///
/// This handles legacy databases created before certain columns were added.
fn ensure_columns(conn: &mut SqliteConnection) {
    use diesel::RunQueryDsl;
    use diesel::sql_query;

    for (table, column, col_type) in SCHEMA_ENTRIES {
        let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {col_type}");
        match sql_query(&sql).execute(conn) {
            Ok(_) => {
                crate::logging::p2plog_debug(format!("[DB] added {column} to table {table}"));
            }
            Err(e) => {
                // SQLite has no "ADD COLUMN IF NOT EXISTS". The common/expected failure modes
                // are "duplicate column name" (already exists) or "no such table" (fresh DB).
                // Don't spam logs for expected cases.
                let msg = e.to_string();
                if msg.contains("duplicate column name") || msg.contains("no such table") {
                    continue;
                }
                crate::logging::p2plog_debug(format!(
                    "[DB] failed to add column {column} to table {table}: {msg}"
                ));
            }
        }
    }
}

/// Checks if a database file is locked by examining its lock file.
///
/// Production selection below scans the working directory for available
/// `SQLite` files; tests instead auto-select per-thread temp databases. With
/// `test-utils` enabled the scanner is compiled out entirely (its unit tests
/// are equally gated); otherwise it stays compiled so the unit tests can
/// exercise it directly.
#[cfg(not(feature = "test-utils"))]
fn is_db_locked(lock_path: &std::path::Path) -> bool {
    use std::fs;

    if !lock_path.exists() {
        return false; // No lock file = available
    }

    if let Ok(content) = fs::read_to_string(lock_path) {
        if let Ok(other_pid) = content.trim().parse::<u32>() {
            if other_pid == 0 {
                let _ = fs::remove_file(lock_path);
                return false; // Empty/zero PID = unlocked/stale
            }
            #[cfg(target_os = "linux")]
            {
                let alive = std::path::Path::new(&format!("/proc/{other_pid}")).exists();
                if !alive {
                    let _ = fs::remove_file(lock_path);
                }
                alive
            }
            #[cfg(not(target_os = "linux"))]
            {
                true // Assume locked on non-Linux
            }
        } else {
            let _ = fs::remove_file(lock_path);
            false // Non-numeric content = stale/invalid lock
        }
    } else {
        let _ = fs::remove_file(lock_path);
        false // Unreadable lock is treated as stale and removed
    }
}

/// Tries to acquire the lock file for a database. Returns Ok if successful.
#[cfg(not(feature = "test-utils"))]
fn try_acquire_lock(lock_path: &std::path::Path, pid: u32) -> Result<(), ()> {
    use std::fs;
    use std::io::Write;

    fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(lock_path)
        .map(|mut f| {
            let _ = f.write_all(pid.to_string().as_bytes());
        })
        .map_err(|_| ())
}

/// Returns `true` if this process may use `url` as-is: there is no lock file,
/// the lock is stale (dead/zero/garbage PID), or the lock is already held by
/// this same process. Acquires the lock (or reclaims a stale one) as a side
/// effect when it returns `true`.
#[cfg(not(any(test, feature = "test-utils")))]
fn can_own_lock(lock_path: &std::path::Path, pid: u32) -> bool {
    if !lock_path.exists() {
        return try_acquire_lock(lock_path, pid).is_ok();
    }
    if let Ok(content) = std::fs::read_to_string(lock_path) {
        if let Ok(other) = content.trim().parse::<u32>() {
            if other == pid {
                return true; // already ours
            }
            if other == 0 {
                let _ = std::fs::remove_file(lock_path);
                return try_acquire_lock(lock_path, pid).is_ok();
            }
            if !is_pid_alive(other) {
                let _ = std::fs::remove_file(lock_path);
                return try_acquire_lock(lock_path, pid).is_ok();
            }
            return false; // live foreign PID
        }
        // non-numeric content = stale/invalid lock
        let _ = std::fs::remove_file(lock_path);
        return try_acquire_lock(lock_path, pid).is_ok();
    }
    let _ = std::fs::remove_file(lock_path);
    try_acquire_lock(lock_path, pid).is_ok()
}

/// Best-effort check for whether a PID is still running.
#[cfg(not(any(test, feature = "test-utils")))]
fn is_pid_alive(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        // No portable liveness check; treat a numeric foreign PID as live so we
        // don't clobber another process's database.
        let _ = pid;
        true
    }
}

/// Derive a process-unique database filename next to `url`, e.g.
/// `/dir/p2p.db` -> `/dir/p2p-<pid>.db`. Used when the requested database is
/// already owned by another live process so the two instances keep distinct
/// identities (peer ids) and never contend on the same `SQLite` file.
#[cfg(not(any(test, feature = "test-utils")))]
fn unique_db_variant(url: &str) -> String {
    let path = std::path::Path::new(url);
    let parent = path.parent().map(|p| p.to_string_lossy().into_owned());
    let stem = path.file_stem().map_or_else(
        || "sqlite".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );
    let ext = path
        .extension()
        .map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
    let file = format!("{stem}-{}{ext}", std::process::id());
    match parent {
        Some(p) if !p.is_empty() => format!("{p}/{file}"),
        _ => file,
    }
}

/// Claim `url` for this process, returning the path it should actually open.
///
/// Creates a PID lock file so a concurrently auto-selecting process (such as
/// the TUI scanning the working directory) skips this database. If another
/// live process already owns `url`, returns a process-unique variant instead so
/// the two instances never share the same identity/peer id.
#[cfg(not(any(test, feature = "test-utils")))]
fn claim_db_url_with_lock(url: &str) -> String {
    let pid = std::process::id();
    let lock_path = format!("{url}.lock");
    if can_own_lock(std::path::Path::new(&lock_path), pid) {
        return url.to_string();
    }
    let variant = unique_db_variant(url);
    let _ = try_acquire_lock(std::path::Path::new(&format!("{variant}.lock")), pid);
    variant
}

/// Finds the first unused `SQLite` database in the current working directory using lock files.
/// If none is available, creates a new database with the next sequential name.
#[cfg(not(feature = "test-utils"))]
fn find_or_create_unused_db() -> color_eyre::Result<String> {
    use crate::logging::p2plog_debug;
    use std::fs;
    use std::process::id as getpid;

    let cwd = std::env::current_dir().wrap_err("failed to get current working directory")?;
    let pid = getpid();
    p2plog_debug(format!("[DB] cwd={} pid={}", cwd.display(), pid));

    // Collect existing .db files and check each immediately
    let mut db_files: Vec<_> = fs::read_dir(&cwd)
        .wrap_err("failed to read current directory")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("db") {
                path.file_name().and_then(|n| n.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();
    db_files.sort();

    // Cleanup pass: remove stale/invalid lock files across all known DB files.
    // This prevents long-term lock-file accumulation from prior crashed test runs.
    for db_file in &db_files {
        let lock_path = cwd.join(format!("{db_file}.lock"));
        let _ = is_db_locked(&lock_path);
    }

    // Check each db file in order, return first available
    for db_file in &db_files {
        let lock_path = cwd.join(format!("{db_file}.lock"));
        p2plog_debug(format!("[DB] checking {db_file}"));

        if is_db_locked(&lock_path) {
            p2plog_debug(format!("[DB]   {db_file} has active lock"));
            continue;
        }

        // Try to acquire lock (may fail if race)
        match try_acquire_lock(&lock_path, pid) {
            Ok(()) => {
                return Ok(cwd.join(db_file.clone()).to_string_lossy().into_owned());
            }
            Err(()) => {
                p2plog_debug(format!(
                    "[DB] lock for {db_file} already exists, trying next"
                ));
            }
        }
    }

    // All existing dbs locked or taken, create new one
    Ok(create_new_db(&db_files, &cwd, pid))
}

#[cfg(not(feature = "test-utils"))]
fn create_new_db(db_files: &[String], cwd: &std::path::Path, pid: u32) -> String {
    use std::fs;
    use std::io::Write;

    let max_n = db_files
        .iter()
        .filter_map(|name| {
            let stem = name.trim_start_matches("sqlite_").trim_end_matches(".db");
            stem.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    let mut candidate = format!("sqlite_{}.db", max_n.saturating_add(1));
    let mut attempts: u32 = 0;

    loop {
        if attempts > 1000 {
            return "sqlite.db".to_string(); // Give up
        }
        let lock_path = cwd.join(format!("{candidate}.lock"));
        if let Ok(mut f) = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            let _ = f.write_all(pid.to_string().as_bytes());
            return candidate;
        }
        attempts = attempts.saturating_add(1);
        candidate = format!("sqlite_{}.db", max_n.saturating_add(attempts));
    }
}

/// Get the database URL for the current thread.
///
/// Returns the URL set via [`set_db_url`], or auto-selects a unique database
/// when none has been set. Because the URL is per-thread, this is isolated
/// across tests and runtime tasks.
#[must_use]
pub fn get_database_url() -> String {
    if let Some(url) = DB_URL.with(|u| u.borrow().clone()) {
        return url;
    }

    // Tests auto-select a per-thread database in the system temp dir (see
    // `unique_test_db_path`) and cache it in the thread-local, so repeated calls
    // within a test resolve to the same file. Test processes never scan the
    // working directory or race on shared lock files, so the parallel test
    // targets under `cargo test --all-features` stay independent. Production
    // reuses the single primary database shared across all threads (set via
    // `set_db_url` or on first selection).
    #[cfg(any(test, feature = "test-utils"))]
    {
        let url = unique_test_db_path();
        DB_URL.with(|u| *u.borrow_mut() = Some(url.clone()));
        url
    }

    #[cfg(not(any(test, feature = "test-utils")))]
    {
        if let Some(url) = primary_db_url() {
            return url;
        }
        let slot = PRIMARY_DB_URL.get_or_init(|| std::sync::Mutex::new(None));
        let mut guard = slot.lock().unwrap();
        if let Some(url) = guard.clone() {
            return url;
        }
        let url = find_or_create_unused_db().unwrap_or_else(|_| "sqlite.db".to_owned());
        *guard = Some(url.clone());
        url
    }
}

/// Release the database lock file by deleting the .lock file.
/// Called on normal exit to clean up the lock file.
pub fn release_db_lock() {
    if let Some(db_path) = DB_URL.with(|u| u.borrow().clone()) {
        let lock_path = format!("{db_path}.lock");
        if std::path::Path::new(&lock_path).exists() && std::fs::remove_file(&lock_path).is_ok() {
            crate::logging::p2plog_debug(format!("[DB] released lock on exit: {lock_path}"));
        }
    }
}

/// Load or generate the libp2p identity keypair.
///
/// Checks the database for an existing identity. If found, deserializes and returns it.
/// If no valid identity exists, generates a new Ed25519 keypair, persists it, and returns it.
///
/// A freshly generated keypair is only returned after it is durably stored, so the
/// derived peer ID is stable across restarts even if the DB write is slow or fails.
///
/// # Errors
/// Returns an error if the database connection fails or the identity cannot be
/// generated and persisted.
pub fn get_libp2p_identity() -> color_eyre::Result<libp2p_identity::Keypair> {
    let conn = &mut sqlite_connect()?;
    if let Ok(rows) = identities.select(Identity::as_select()).load(conn) {
        for row in rows {
            match libp2p_identity::Keypair::from_protobuf_encoding(&row.key) {
                Ok(i) => {
                    return Ok(i);
                }
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("invalid identity stored: {row:?} - {e}");
                }
            }
        }
    }
    #[cfg(feature = "tracing")]
    tracing::warn!("no valid identity found in database, generating and storing new one");
    let keypair = libp2p_identity::Keypair::generate_ed25519();
    let key = keypair
        .to_protobuf_encoding()
        .map_err(|e| color_eyre::eyre::eyre!("failed to encode new identity: {e}"))?;
    let new_identity = crate::generated::models_insertable::NewIdentity {
        key,
        last_tcp_port: None,
        last_quic_port: None,
        self_nickname: None,
    };
    let identity = diesel::insert_into(crate::generated::schema::identities::table)
        .values(&new_identity)
        .returning(Identity::as_returning())
        .get_result(conn)
        .map_err(|e| color_eyre::eyre::eyre!("failed to persist new identity: {e}"))?;
    #[cfg(feature = "tracing")]
    tracing::info!("inserted new identity: {identity:?}");
    Ok(keypair)
}

/// Get the local peer ID from the stored identity.
///
/// # Errors
/// Returns an error if the identity cannot be loaded or generated.
pub fn get_local_peer_id() -> color_eyre::Result<libp2p::PeerId> {
    let keypair = get_libp2p_identity()?;
    Ok(keypair.public().to_peer_id())
}

#[cfg(test)]
#[path = "../tests/unit/unit_db.rs"]
mod tests;
