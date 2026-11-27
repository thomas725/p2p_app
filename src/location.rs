//! Location sharing: validated samples, canonical map URLs and session policy.
//!
//! Every policy decision lives here: coordinate validation, sharing-mode
//! semantics, sensor/send frequency bounds, expiry and the canonical message
//! text. Frontends only sample their sensors and pass raw numbers across this
//! boundary; no latitude maths, URL assembly or expiry arithmetic is
//! duplicated in Dart or Kotlin.
//!
//! Location rides the ordinary (sealed) DM path. [`build_location_message`]
//! produces a canonical body that is sent like any other DM, so the receive
//! side needs no new wire type or database column: both frontends already
//! render canonical message text.

use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

/// Prefix that marks a canonical location message body.
pub const MESSAGE_PREFIX: &str = "[Location]";

/// Base of the canonical, free, public OpenStreetMap URL.
///
/// OSM is used deliberately: it needs no API key, no account and no tracking
/// cookie. Coordinates are embedded in the URL fragment so the map centres on
/// the reported point with a marker.
pub const MAP_URL_PREFIX: &str = "https://www.openstreetmap.org/";

/// Shortest permitted sensor-query interval, in seconds.
pub const MIN_SENSOR_INTERVAL_SECS: u32 = 1;
/// Longest permitted sensor-query interval, in seconds (one hour).
pub const MAX_SENSOR_INTERVAL_SECS: u32 = 3_600;
/// Shortest permitted network-send interval, in seconds.
pub const MIN_SEND_INTERVAL_SECS: u32 = 1;
/// Longest permitted network-send interval, in seconds (one day).
pub const MAX_SEND_INTERVAL_SECS: u32 = 86_400;
/// Shortest permitted limited-sharing window, in seconds (one minute).
pub const MIN_DURATION_SECS: u32 = 60;
/// Longest permitted limited-sharing window, in seconds (seven days).
pub const MAX_DURATION_SECS: u32 = 604_800;

/// A validated latitude in decimal degrees.
///
/// The constructor is the only way to build one, so an out-of-range or
/// non-finite latitude cannot enter the rest of the system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Latitude(f64);

impl Latitude {
    /// Validate and wrap a latitude.
    ///
    /// # Errors
    /// Returns a human-readable error when `value` is not finite or is outside
    /// `[-90, 90]`.
    pub fn new(value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("latitude must be a finite number".to_string());
        }
        if !(-90.0..=90.0).contains(&value) {
            return Err("latitude must be between -90 and 90".to_string());
        }
        Ok(Self(value))
    }

    /// The validated value.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Latitude {
    type Error = String;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Latitude> for f64 {
    fn from(value: Latitude) -> Self {
        value.0
    }
}

/// A validated longitude in decimal degrees.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct Longitude(f64);

impl Longitude {
    /// Validate and wrap a longitude.
    ///
    /// # Errors
    /// Returns a human-readable error when `value` is not finite or is outside
    /// `[-180, 180]`.
    pub fn new(value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("longitude must be a finite number".to_string());
        }
        if !(-180.0..=180.0).contains(&value) {
            return Err("longitude must be between -180 and 180".to_string());
        }
        Ok(Self(value))
    }

    /// The validated value.
    #[must_use]
    pub const fn value(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for Longitude {
    type Error = String;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<Longitude> for f64 {
    fn from(value: Longitude) -> Self {
        value.0
    }
}

/// How long a location-sharing session stays active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocationMode {
    /// One sample, then stop.
    Once,
    /// Keep sharing until a wall-clock deadline.
    Limited,
    /// Keep sharing until the user explicitly stops.
    Unlimited,
}

impl LocationMode {
    /// Parse the canonical lowercase wire/UI name.
    ///
    /// # Errors
    /// Returns an error listing the accepted names for anything else.
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "once" => Ok(Self::Once),
            "limited" => Ok(Self::Limited),
            "unlimited" => Ok(Self::Unlimited),
            other => Err(format!(
                "unknown location mode '{other}' (expected once, limited or unlimited)"
            )),
        }
    }

    /// The canonical lowercase name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Once => "once",
            Self::Limited => "limited",
            Self::Unlimited => "unlimited",
        }
    }
}

/// One location sample as supplied by a device adapter.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LocationSample {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Estimated horizontal accuracy in metres (negative values are rejected).
    pub accuracy: f64,
}

/// Settings for a (possibly continued) sharing session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocationSessionSettings {
    /// Sharing mode.
    pub mode: LocationMode,
    /// Window length for [`LocationMode::Limited`], ignored for other modes.
    pub duration_secs: Option<u32>,
    /// How often the device queries its location sensor, in seconds.
    pub sensor_interval_secs: u32,
    /// How often a sample is sent over the network, in seconds.
    pub send_interval_secs: u32,
    /// Whether a low-power motion sensor gates location re-querying.
    pub motion_gate: bool,
}

/// Everything a frontend needs to build one canonical location message.
///
/// Deliberately flat and primitive-only: this is the FRB/web boundary DTO, so
/// Dart and the JSON transport can construct it without knowing about the
/// internal newtypes or enum. Rust validates all of it in [`LocationShare::validate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationShareInput {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Estimated horizontal accuracy in metres.
    pub accuracy: f64,
    /// Canonical mode name: `once`, `limited` or `unlimited`.
    pub mode: String,
    /// Window length for `limited` sharing; ignored otherwise.
    pub duration_secs: Option<u32>,
    /// How often the sensor is queried, in seconds.
    pub sensor_interval_secs: u32,
    /// How often a sample is sent, in seconds.
    pub send_interval_secs: u32,
    /// Whether a low-power motion gate controls sensor queries.
    pub motion_gate: bool,
    /// Sender-minted session id; all updates of one continued share reuse it.
    pub session_id: String,
    /// Monotonic sample number within the session (starts at 1).
    pub sequence: u32,
    /// True for the last sample of a limited/unlimited session.
    pub is_final: bool,
}

/// A fully validated location share, ready for canonical formatting.
#[derive(Debug, Clone, PartialEq)]
pub struct LocationShare {
    latitude: Latitude,
    longitude: Longitude,
    accuracy: f64,
    session: LocationSessionSettings,
    session_id: String,
    sequence: u32,
    is_final: bool,
}

impl LocationShare {
    /// Validate `input` into the typed share.
    ///
    /// # Errors
    /// Returns a human-readable error for a bad coordinate, accuracy,
    /// frequency, duration, sequence number or session id.
    pub fn validate(input: LocationShareInput) -> Result<Self, String> {
        let latitude = Latitude::new(input.latitude)?;
        let longitude = Longitude::new(input.longitude)?;
        let accuracy = input.accuracy;
        if !accuracy.is_finite() || accuracy < 0.0 {
            return Err("location accuracy must be a non-negative number".to_string());
        }
        let session = validate_session(&LocationSessionSettings {
            mode: LocationMode::parse(&input.mode)?,
            duration_secs: input.duration_secs,
            sensor_interval_secs: input.sensor_interval_secs,
            send_interval_secs: input.send_interval_secs,
            motion_gate: input.motion_gate,
        })?;
        if input.sequence == 0 {
            return Err("location sequence must start at 1".to_string());
        }
        if input.session_id.trim().is_empty() {
            return Err("location session id must not be empty".to_string());
        }
        Ok(Self {
            latitude,
            longitude,
            accuracy,
            session,
            session_id: input.session_id,
            sequence: input.sequence,
            is_final: input.is_final,
        })
    }

    /// The canonical, free map URL for this sample.
    #[must_use]
    pub fn map_url(&self) -> String {
        location_map_url(self.latitude.value(), self.longitude.value())
    }

    /// Render the canonical message body carried inside the (sealed) DM.
    #[must_use]
    pub fn message_body(&self) -> String {
        let mut body = String::new();
        // Writing to a String cannot fail.
        let _ = write!(
            body,
            "{} {:.6},{:.6} (±{:.0} m) {}",
            MESSAGE_PREFIX,
            self.latitude.value(),
            self.longitude.value(),
            self.accuracy,
            self.map_url(),
        );
        let _ = write!(body, " ({})", self.session_summary());
        if self.sequence > 1 {
            let _ = write!(body, " #{}", self.sequence);
        }
        if self.is_final {
            let _ = write!(body, " (final)");
        }
        body
    }

    fn session_summary(&self) -> String {
        let base = match self.session.mode {
            LocationMode::Once => "once".to_string(),
            LocationMode::Limited => format!(
                "limited {}m",
                self.session.duration_secs.unwrap_or_default() / 60
            ),
            LocationMode::Unlimited => "unlimited".to_string(),
        };
        let mut summary = base;
        if self.session.motion_gate && self.session.mode != LocationMode::Once {
            summary.push_str(", motion-gated");
        }
        summary
    }
}

/// Validate and normalise session settings (frequency bounds and duration).
fn validate_session(settings: &LocationSessionSettings) -> Result<LocationSessionSettings, String> {
    if !(MIN_SENSOR_INTERVAL_SECS..=MAX_SENSOR_INTERVAL_SECS)
        .contains(&settings.sensor_interval_secs)
    {
        return Err(format!(
            "sensor interval must be between {MIN_SENSOR_INTERVAL_SECS} and {MAX_SENSOR_INTERVAL_SECS} seconds"
        ));
    }
    if !(MIN_SEND_INTERVAL_SECS..=MAX_SEND_INTERVAL_SECS).contains(&settings.send_interval_secs) {
        return Err(format!(
            "send interval must be between {MIN_SEND_INTERVAL_SECS} and {MAX_SEND_INTERVAL_SECS} seconds"
        ));
    }
    let mut normalised = *settings;
    match settings.mode {
        LocationMode::Limited => {
            let Some(duration) = settings.duration_secs else {
                return Err("limited sharing needs a duration".to_string());
            };
            if !(MIN_DURATION_SECS..=MAX_DURATION_SECS).contains(&duration) {
                return Err(format!(
                    "sharing duration must be between {MIN_DURATION_SECS} and {MAX_DURATION_SECS} seconds"
                ));
            }
        }
        LocationMode::Once | LocationMode::Unlimited => normalised.duration_secs = None,
    }
    Ok(normalised)
}

/// Build the canonical OSM URL for a validated coordinate pair.
///
/// The public OSM site needs no API key or account. Coordinates appear in both
/// the query (`mlat`/`mlon`, which places a marker) and the fragment (`#map`,
/// which sets zoom/centre), so opening the URL shows the exact point.
#[must_use]
pub fn location_map_url(latitude: f64, longitude: f64) -> String {
    let lat = Latitude::new(latitude).map_or(0.0, Latitude::value);
    let lon = Longitude::new(longitude).map_or(0.0, Longitude::value);
    format!("{MAP_URL_PREFIX}?mlat={lat:.6}&mlon={lon:.6}#map=17/{lat:.6}/{lon:.6}")
}

/// Whether `content` is a canonical location message.
#[must_use]
pub fn is_location_message(content: &str) -> bool {
    content.trim_start().starts_with(MESSAGE_PREFIX)
}

/// Validate and render one canonical location message body.
///
/// # Errors
/// Returns a human-readable validation error (see [`LocationShare::validate`]).
pub fn build_location_message(input: LocationShareInput) -> Result<String, String> {
    Ok(LocationShare::validate(input)?.message_body())
}

/// Whether a limited session has passed its deadline.
///
/// Uses wall-clock seconds so it works identically on every frontend.
#[must_use]
pub fn location_session_expired(
    started_at: f64,
    settings: &LocationSessionSettings,
    now: f64,
) -> bool {
    match settings.mode {
        LocationMode::Once => true,
        LocationMode::Unlimited => false,
        LocationMode::Limited => settings
            .duration_secs
            .is_some_and(|duration| now >= started_at + f64::from(duration)),
    }
}

/// Whether a sharing session is still active at `now`, given its Rust-validated
/// mode and optional duration. Exposed to the frontends so their timers never
/// implement expiry policy themselves.
///
/// # Errors
/// Returns an error for an unknown mode name.
pub fn location_session_active(
    started_at: f64,
    mode: &str,
    duration_secs: Option<u32>,
    now: f64,
) -> Result<bool, String> {
    let settings = LocationSessionSettings {
        mode: LocationMode::parse(mode)?,
        duration_secs,
        sensor_interval_secs: MIN_SENSOR_INTERVAL_SECS,
        send_interval_secs: MIN_SEND_INTERVAL_SECS,
        motion_gate: false,
    };
    Ok(!location_session_expired(started_at, &settings, now))
}

/// Whether a new sample should be sent given the last send time.
///
/// `None` means nothing has been sent yet, so the first sample is always due.
#[must_use]
pub fn location_send_due(last_sent_at: Option<f64>, now: f64, interval_secs: u32) -> bool {
    last_sent_at.is_none_or(|last| now >= last + f64::from(interval_secs))
}

/// Whether the sensor should be re-queried given the last query time.
#[must_use]
pub fn location_query_due(last_query_at: Option<f64>, now: f64, interval_secs: u32) -> bool {
    location_send_due(last_query_at, now, interval_secs)
}

#[cfg(test)]
#[path = "../tests/unit/unit_location.rs"]
mod tests;
