//! Database connection and identity key management
//!
//! This module manages SQLite connections. To avoid connection overhead,
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
use dotenvy::dotenv;
use std::env;
use std::sync::OnceLock;

pub use crate::MIGRATIONS;

/// Cache the database URL after first connection to avoid repeated lock file checks.
static DB_URL: OnceLock<String> = OnceLock::new();

/// Establish a connection to the SQLite database and run pending migrations.
///
/// If DATABASE_URL is set, uses that file directly.
/// Otherwise, finds the first unused SQLite database in the current working directory
/// using lock files (`.lock` files with our PID), or creates a new one.
/// Automatically runs all pending Diesel migrations on first call.
///
/// **Optimization:** The database path is cached after the first connection,
/// avoiding expensive lock file checks on subsequent operations.
///
/// # Returns
/// A new SqliteConnection with all migrations applied, or an error if connection/migration fails
///
/// # Errors
/// - If database file cannot be found or created
/// - If migrations fail to execute
pub fn sqlite_connect() -> color_eyre::Result<SqliteConnection> {
    dotenv().ok();

    // Try to get cached path, or determine it for the first time
    let db_path = if let Some(cached) = DB_URL.get() {
        cached.clone()
    } else {
        let path = determine_db_path()?;
        let _ = DB_URL.set(path.clone());
        path
    };

    // Register cleanup on panic after path is determined and cached (to ensure it lives for the app lifetime)
    static PANIC_HOOK_SET: OnceLock<()> = OnceLock::new();
    let _ = PANIC_HOOK_SET.get_or_init(|| {
        std::panic::set_hook(Box::new(|_info| {
            if let Some(db_path) = DB_URL.get() {
                let lock_path = format!("{}.lock", db_path);
                let _ = std::fs::remove_file(&lock_path);
                eprintln!("[DB] released lock on panic: {}", lock_path);
            }
        }));
    });

    let mut conn = SqliteConnection::establish(&db_path)
        .wrap_err_with(|| format!("Error connecting to {db_path}"))?;

    // Ensure columns that may be missing from older schemas
    ensure_columns(&mut conn);

    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| eyre!(format!("Error executing migrations on {db_path}: {e}")))?;
    Ok(conn)
}

/// Ensures all columns exist in the database schema.
/// This is needed because SQLite doesn't support "ADD COLUMN IF NOT EXISTS".
/// We check each table/column pair and add missing ones before migrations run.
///
/// This handles legacy databases created before certain columns were added.
fn ensure_columns(conn: &mut SqliteConnection) {
    use diesel::RunQueryDsl;
    use diesel::sql_query;

    for (table, column, col_type) in SCHEMA_ENTRIES {
        let sql = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, col_type);
        match sql_query(&sql).execute(conn) {
            Ok(_) => crate::logging::p2plog_debug(format!("[DB] added {} to table {}", column, table)),
            Err(e) => {
                // SQLite has no "ADD COLUMN IF NOT EXISTS". The common/expected failure mode
                // is "duplicate column name: <col>" for already-existing columns; don't spam logs.
                let msg = e.to_string();
                if msg.contains("duplicate column name") {
                    continue;
                }
                crate::logging::p2plog_debug(format!(
                    "[DB] failed to add column {} to table {}: {}",
                    column, table, msg
                ));
            }
        }
    }
}

/// Determine the database path (cached version to avoid repeated lock file checks).
fn determine_db_path() -> color_eyre::Result<String> {
    dotenv().ok();

    // If DATABASE_URL is explicitly set, use it directly (lock-file logic not needed)
    if let Ok(url) = env::var("DATABASE_URL") {
        return Ok(url);
    }

    // No DATABASE_URL set: find or create an unused database in cwd
    find_or_create_unused_db()
}

/// Checks if a database file is locked by examining its lock file.
fn is_db_locked(lock_path: &std::path::Path) -> bool {
    use std::fs;

    if !lock_path.exists() {
        return false; // No lock file = available
    }

    match fs::read_to_string(lock_path) {
        Ok(content) => {
            if let Ok(other_pid) = content.trim().parse::<u32>() {
                if other_pid == 0 {
                    return false; // Empty/zero PID = unlocked
                }
                #[cfg(target_os = "linux")]
                {
                    std::path::Path::new(&format!("/proc/{other_pid}")).exists()
                }
                #[cfg(not(target_os = "linux"))]
                {
                    true // Assume locked on non-Linux
                }
            } else {
                true // Non-numeric content = locked
            }
        }
        Err(_) => true, // Can't read = locked
    }
}

/// Tries to acquire the lock file for a database. Returns Ok if successful.
fn try_acquire_lock(lock_path: &std::path::Path, pid: u32) -> Result<(), ()> {
    use std::fs;
    use std::io::Write;

    match fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(lock_path)
    {
        Ok(mut f) => {
            let _ = f.write_all(pid.to_string().as_bytes());
            Ok(())
        }
        Err(_) => Err(()),
    }
}

/// Finds the first unused SQLite database in the current working directory using lock files.
/// If none is available, creates a new database with the next sequential name.
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

    // Check each db file in order, return first available
    for db_file in &db_files {
        let lock_path = cwd.join(format!("{}.lock", db_file));
        p2plog_debug(format!("[DB] checking {}", db_file));

        if is_db_locked(&lock_path) {
            p2plog_debug(format!("[DB]   {} has active lock", db_file));
            continue;
        }

        // Try to acquire lock (may fail if race)
        match try_acquire_lock(&lock_path, pid) {
            Ok(()) => {
                return Ok(cwd.join(db_file.clone()).to_string_lossy().into_owned());
            }
            Err(()) => {
                p2plog_debug(format!(
                    "[DB] lock for {} already exists, trying next",
                    db_file
                ));
                continue; // Lost race, try next
            }
        }
    }

    // All existing dbs locked or taken, create new one
    Ok(create_new_db(&db_files, &cwd, pid))
}

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
    let mut candidate = format!("sqlite_{}.db", max_n + 1);
    let mut attempts = 0;

    loop {
        if attempts > 1000 {
            return "sqlite.db".to_string(); // Give up
        }
        let lock_path = cwd.join(format!("{}.lock", candidate));
        match fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(mut f) => {
                let _ = f.write_all(pid.to_string().as_bytes());
                return candidate;
            }
            Err(_) => {
                attempts += 1;
                candidate = format!("sqlite_{}.db", max_n + attempts);
            }
        }
    }
}

/// Get the database URL from environment or default value.
///
/// Respects `DATABASE_URL` environment variable or `.env` file, defaulting to "sqlite.db".
/// Get the database URL from environment or default value.
/// Caches result in DB_URL so subsequent calls (like from sqlite_connect) use same db.
#[must_use]
pub fn get_database_url() -> String {
    if let Some(cached) = DB_URL.get() {
        return cached.clone();
    }
    dotenv().ok();
    let url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| find_or_create_unused_db().unwrap_or_else(|_| "sqlite.db".to_owned()));
    let _ = DB_URL.set(url.clone());
    url
}

/// Release the database lock file by deleting the .lock file.
/// Called on normal exit to clean up the lock file.
pub fn release_db_lock() {
    if let Some(db_path) = DB_URL.get() {
        let lock_path = format!("{}.lock", db_path);
        if std::path::Path::new(&lock_path).exists() && std::fs::remove_file(&lock_path).is_ok() {
            eprintln!("[DB] released lock on exit: {}", lock_path);
        }
    }
}

/// Load or generate the libp2p identity keypair.
///
/// Checks the database for an existing identity. If found, deserializes and returns it.
/// If no valid identity exists, generates a new Ed25519 keypair, stores it, and returns it.
pub fn get_libp2p_identity() -> color_eyre::Result<libp2p_identity::Keypair> {
    dotenv().ok();
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
    match keypair.to_protobuf_encoding() {
        Ok(key) => {
            let i = crate::generated::models_insertable::NewIdentity {
                key,
                last_tcp_port: None,
                last_quic_port: None,
                self_nickname: None,
            };
            match diesel::insert_into(crate::generated::schema::identities::table)
                .values(&i)
                .returning(Identity::as_returning())
                .get_result(conn)
            {
                Ok(i) => {
                    #[cfg(feature = "tracing")]
                    tracing::info!("inserted new identity: {i:?}");
                }
                Err(e) => {
                    #[cfg(feature = "tracing")]
                    tracing::error!("failed to insert identity {i:?}: {e}");
                }
            }
        }
        Err(e) => {
            #[cfg(feature = "tracing")]
            tracing::error!("failed to encode identity: {e}");
        }
    }
    Ok(keypair)
}
