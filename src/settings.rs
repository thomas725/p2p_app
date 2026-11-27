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
use crate::location::{self, LocationMode};
use crate::theme::Accent;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _};
use serde::{Deserialize, Serialize};

/// `app_settings.key` for LAN-only mode (US-051).
const KEY_LAN_ONLY: &str = "lan_only";
/// `app_settings.key` for the dark/light theme (US-025).
const KEY_THEME_DARK: &str = "theme_dark";
/// `app_settings.key` for the accent colour (US-026).
const KEY_THEME_ACCENT: &str = "theme_accent";
/// `app_settings.key` for the notification sound channel (US-027).
const KEY_NOTIFICATION_SOUND: &str = "notification_sound";
/// `app_settings.key` for the desktop notification channel (US-028).
const KEY_NOTIFICATION_DESKTOP: &str = "notification_desktop";
/// `app_settings.key` for the broadcast viewed watermark (highest row id read).
const KEY_BROADCAST_LAST_VIEWED_ID: &str = "broadcast_last_viewed_id";
/// `app_settings.key` for the location-sharing dialog defaults (JSON).
const KEY_LOCATION_DEFAULTS: &str = "location_defaults";

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

/// Whether the notification sound is enabled (US-027). Defaults to `true`:
/// a sound needs no permission and preserves the expected chat behaviour.
#[must_use]
pub fn notification_sound() -> bool {
    get(KEY_NOTIFICATION_SOUND).is_none_or(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// Persist the notification-sound choice (US-027). Stored as `"1"`/`"0"`.
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_notification_sound(enabled: bool) -> Result<(), String> {
    set(KEY_NOTIFICATION_SOUND, if enabled { "1" } else { "0" })
}

/// Whether desktop notifications are enabled (US-028). Defaults to `false`:
/// browsers and Android 13+ require an explicit permission grant first, so a
/// fresh install stays quiet until the user opts in.
#[must_use]
pub fn notification_desktop() -> bool {
    get(KEY_NOTIFICATION_DESKTOP).is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

/// Persist the desktop-notification choice (US-028). Stored as `"1"`/`"0"`.
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_notification_desktop(enabled: bool) -> Result<(), String> {
    set(KEY_NOTIFICATION_DESKTOP, if enabled { "1" } else { "0" })
}

/// Flat, primitive-only defaults for the location-sharing dialog.
///
/// Both frontends use the canonical lowercase mode name, so the FRB/web
/// boundary stays free of Rust enums; [`set_location_defaults`] validates the
/// values with the same policy a live share uses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationDefaults {
    /// Canonical mode name: `once`, `limited` or `unlimited`.
    pub mode: String,
    /// Window length for `limited` sharing; ignored for other modes.
    pub duration_secs: Option<u32>,
    /// How often the device queries its sensor, in seconds.
    pub sensor_interval_secs: u32,
    /// How often a sample is sent over the network, in seconds.
    pub send_interval_secs: u32,
    /// Whether the low-power motion gate is on (enabled by default).
    pub motion_gate: bool,
}

impl Default for LocationDefaults {
    fn default() -> Self {
        Self {
            mode: LocationMode::Once.as_str().to_string(),
            duration_secs: None,
            sensor_interval_secs: location::DEFAULT_SENSOR_INTERVAL_SECS,
            send_interval_secs: location::DEFAULT_SEND_INTERVAL_SECS,
            motion_gate: true,
        }
    }
}

impl LocationDefaults {
    /// Validate with the canonical session policy and return the normalised
    /// value (e.g. no duration for a one-shot share).
    ///
    /// # Errors
    /// Returns a human-readable validation error from [`location::validate_location_settings`].
    pub fn validated(self) -> Result<Self, String> {
        let mode = LocationMode::parse(&self.mode)?;
        let normalised =
            location::validate_location_settings(&location::LocationSessionSettings {
                mode,
                duration_secs: self.duration_secs,
                sensor_interval_secs: self.sensor_interval_secs,
                send_interval_secs: self.send_interval_secs,
                motion_gate: self.motion_gate,
            })?;
        Ok(Self {
            mode: normalised.mode.as_str().to_string(),
            duration_secs: normalised.duration_secs,
            sensor_interval_secs: normalised.sensor_interval_secs,
            send_interval_secs: normalised.send_interval_secs,
            motion_gate: normalised.motion_gate,
        })
    }
}

/// Persisted location-sharing defaults. A missing, corrupt or invalid row falls
/// back to [`LocationDefaults::default`] so a broken settings table never stops
/// the sharing dialog from opening.
#[must_use]
pub fn location_defaults() -> LocationDefaults {
    get(KEY_LOCATION_DEFAULTS)
        .and_then(|raw| serde_json::from_str::<LocationDefaults>(&raw).ok())
        .and_then(|defaults| defaults.validated().ok())
        .unwrap_or_default()
}

/// Persist validated location-sharing defaults.
///
/// # Errors
/// Returns a validation error for bad input, or a human-readable DB error.
pub fn set_location_defaults(input: LocationDefaults) -> Result<(), String> {
    let validated = input.validated()?;
    let raw = serde_json::to_string(&validated).map_err(|e| e.to_string())?;
    set(KEY_LOCATION_DEFAULTS, &raw)
}

/// Highest broadcast row id the user has viewed, or `None` on a fresh install.
///
/// [`crate::messages::broadcast_unread_count`] counts non-deleted rows above
/// this id. The marker is an *id*, not a loaded-list length: history caps,
/// tombstones and trimming cannot shift it. `None` means "never viewed", which
/// deliberately reports **zero** unread so a first launch does not flag all of
/// history as new; writing the marker (even `0`) switches to id-based counting.
#[must_use]
pub fn broadcast_last_viewed_id() -> Option<i32> {
    get(KEY_BROADCAST_LAST_VIEWED_ID).and_then(|value| value.parse().ok())
}

/// Persist the broadcast viewed watermark (the highest viewed row id).
///
/// # Errors
/// Returns a human-readable error if the database write fails.
pub fn set_broadcast_last_viewed_id(id: i32) -> Result<(), String> {
    set(KEY_BROADCAST_LAST_VIEWED_ID, &id.to_string())
}

#[cfg(test)]
#[path = "../tests/unit/unit_settings.rs"]
mod tests;
