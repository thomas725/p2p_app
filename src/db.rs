//! Database connection and identity key management
//!
//! This module manages SQLite connections. To avoid connection overhead,
//! a single connection is established and reused for the lifetime of the application.
//! While this is not suitable for high-concurrency scenarios, it's appropriate for
//! this single-threaded TUI application.
//!
//! For future multi-threaded use, consider using r2d2 connection pooling.

use crate::schema::identities::dsl::identities;
use crate::models_queryable::Identity;
use color_eyre::eyre::{eyre, Context};
use diesel::{Connection as _, QueryDsl, RunQueryDsl as _, SelectableHelper as _, SqliteConnection};
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

    let mut conn = SqliteConnection::establish(&db_path)
        .wrap_err_with(|| format!("Error connecting to {db_path}"))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| eyre!(format!("Error executing migrations on {db_path}: {e}")))?;
    Ok(conn)
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

/// Finds the first unused SQLite database in the current working directory using lock files.
/// If none is available, creates a new database with the next sequential name.
fn find_or_create_unused_db() -> color_eyre::Result<String> {
    use std::fs;
    use std::io::Write;
    use std::process::id as getpid;

    let cwd = std::env::current_dir().wrap_err("failed to get current working directory")?;
    let pid = getpid();

    // Collect all existing .db files
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

    // Check existing databases for lock files (lock indicates active use)
    // A lock file is considered active if:
    // 1. It exists and contains a numeric PID
    // 2. That process is still running
    let mut available = Vec::new();
    for db_file in &db_files {
        let lock_path = cwd.join(format!("{}.lock", db_file));
        let is_locked = if lock_path.exists() {
            match fs::read_to_string(&lock_path) {
                Ok(content) => {
                    // If PID in lock file is still running, it's locked
                    if let Ok(other_pid) = content.trim().parse::<u32>() {
                        if other_pid == 0 {
                            false // Empty/zero PID means unlocked
                        } else {
                            // Check if process with this PID exists using /proc
                            #[cfg(target_os = "linux")]
                            {
                                std::path::Path::new(&format!("/proc/{other_pid}")).exists()
                            }
                            #[cfg(not(target_os = "linux"))]
                            {
                                // On non-Linux, we can't easily check, so assume locked
                                true
                            }
                        }
                    } else {
                        true // Non-numeric content = assume locked
                    }
                }
                Err(_) => true, // Can't read lock file = assume locked
            }
        } else {
            false // No lock file = available
        };
        if !is_locked {
            available.push(db_file.clone());
        }
    }

    // Pick first available, or create new one
    let db_file = if let Some(first) = available.first() {
        first.clone()
    } else {
        // Find the next sequential number
        let next_n = db_files
            .iter()
            .filter_map(|name| {
                let stem = name.trim_start_matches("sqlite_").trim_end_matches(".db");
                stem.parse::<u32>().ok()
            })
            .max()
            .unwrap_or(0);
        format!("sqlite_{}.db", next_n + 1)
    };

    // Write lock file with our PID
    let lock_path = cwd.join(format!("{}.lock", db_file));
    let mut lock_file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)
        .wrap_err_with(|| format!("failed to create lock file for {db_file}"))?;
    write!(lock_file, "{}", pid)
        .wrap_err_with(|| "failed to write PID to lock file".to_string())?;

    Ok(cwd.join(db_file).to_string_lossy().into_owned())
}

/// Get the database URL from environment or default value.
///
/// Respects `DATABASE_URL` environment variable or `.env` file, defaulting to "sqlite.db".
#[must_use]
pub fn get_database_url() -> String {
    dotenv().ok();
    env::var("DATABASE_URL")
        .unwrap_or_else(|_| find_or_create_unused_db().unwrap_or_else(|_| "sqlite.db".to_owned()))
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
            let i = crate::models_insertable::NewIdentity {
                key,
                last_tcp_port: None,
                last_quic_port: None,
                self_nickname: None,
            };
            match diesel::insert_into(crate::schema::identities::table)
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
