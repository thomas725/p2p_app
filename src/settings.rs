//! Persisted application settings (US-051).
//!
//! A small key/value store backed by `app_settings`, with **typed** accessors
//! so callers never parse or spell keys/values themselves. Settings are read
//! when the node starts (or when Settings is rendered) and are never fanned
//! out over the network.
//!
//! Reads are best-effort: a broken database falls back to the default rather
//! than aborting startup. This mirrors `network_config`'s fail-open readers —
//! a settings-table error must never make the node refuse to start.

use crate::generated::models_insertable::NewAppSetting;
use crate::generated::schema::app_settings;
use crate::theme::Accent;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _};

/// `app_settings.key` for LAN-only mode (US-051).
const KEY_LAN_ONLY: &str = "lan_only";
/// `app_settings.key` for the dark/light theme (US-025).
const KEY_THEME_DARK: &str = "theme_dark";
/// `app_settings.key` for the accent colour (US-026).
const KEY_THEME_ACCENT: &str = "theme_accent";

/// Read a raw setting value. Returns `None` when the key is absent or the DB
/// cannot be read (a broken settings table must not break the caller).
fn get(key: &str) -> Option<String> {
    let conn = &mut crate::sqlite_connect().ok()?;
    app_settings::table
        .filter(app_settings::key.eq(key))
        .select(app_settings::value)
        .first::<String>(conn)
        .optional()
        .ok()
        .flatten()
}

/// Write a raw setting value (upsert), returning a human-readable DB error.
fn set(key: &str, value: &str) -> Result<(), String> {
    let conn = &mut crate::sqlite_connect().map_err(|e| e.to_string())?;
    diesel::insert_into(app_settings::table)
        .values(NewAppSetting {
            key: key.to_string(),
            value: value.to_string(),
        })
        .on_conflict(app_settings::key)
        .do_update()
        .set(app_settings::value.eq(value))
        .execute(conn)
        .map_err(|e| format!("failed to save setting: {e}"))?;
    Ok(())
}

/// Whether LAN-only mode is enabled (US-051).
///
/// Default `false` (full internet discovery as before) when the key is absent
/// or unreadable, so an existing install keeps its current behaviour.
#[must_use]
pub fn lan_only() -> bool {
    get(KEY_LAN_ONLY).is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// Enable or disable LAN-only mode (US-051).
///
/// The value is stored as `"1"`/`"0"`; [`lan_only`] is the only reader. The
/// change applies on the **next node start** — the swarm is built once and
/// discovery/relay wiring is fixed at that point.
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_lan_only(enabled: bool) -> Result<(), String> {
    set(KEY_LAN_ONLY, if enabled { "1" } else { "0" })
}

/// Whether the dark palette is active (US-025).
///
/// Defaults to `true` when absent or unreadable: terminal installs (and the
/// project's original Flutter seed) have always been dark, so existing users
/// keep their look until they explicitly choose light.
#[must_use]
pub fn theme_dark() -> bool {
    get(KEY_THEME_DARK)
        .is_none_or(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on" | "dark"))
}

/// Set the dark/light palette choice (US-025). Stored as `"1"`/`"0"`.
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_theme_dark(dark: bool) -> Result<(), String> {
    set(KEY_THEME_DARK, if dark { "1" } else { "0" })
}

/// The validated accent colour (US-026), defaulting to teal.
#[must_use]
pub fn theme_accent() -> Accent {
    get(KEY_THEME_ACCENT)
        .and_then(|value| Accent::parse(&value))
        .unwrap_or(Accent::TEAL)
}

/// Persist a validated accent colour (US-026).
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_theme_accent(accent: Accent) -> Result<(), String> {
    set(KEY_THEME_ACCENT, accent.as_str())
}

#[cfg(test)]
#[path = "../tests/unit/unit_settings.rs"]
mod tests;
