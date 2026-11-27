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

use chrono::{DateTime, NaiveDate, NaiveDateTime, SecondsFormat, Utc};
use num_traits::ToPrimitive as _;
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

/// Default sensor-query interval (seconds) offered in the sharing dialog.
pub const DEFAULT_SENSOR_INTERVAL_SECS: u32 = 10;
/// Default network-send interval (seconds) offered in the sharing dialog.
pub const DEFAULT_SEND_INTERVAL_SECS: u32 = 60;

/// Maximum waypoints accepted by the public OSM directions service.
///
/// The track URL builder keeps the newest points up to this cap so it stays
/// within the service's documented limit; GPX exports are unbounded.
pub const MAX_TRACK_URL_POINTS: usize = 50;

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
            "{} {:.6},{:.6} (±{:.0} m) @{:.0} {}",
            MESSAGE_PREFIX,
            self.latitude.value(),
            self.longitude.value(),
            self.accuracy,
            crate::fmt::current_timestamp(),
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
///
/// Exposed so persisted UI defaults use exactly the same bounds and mode rules
/// as a live share; no frontend holds a second copy of the policy.
///
/// # Errors
/// Returns a human-readable error for an unknown mode or an out-of-range value.
pub fn validate_location_settings(
    settings: &LocationSessionSettings,
) -> Result<LocationSessionSettings, String> {
    validate_session(settings)
}

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

/// Extract the canonical map URL from a location message body.
///
/// Frontends use this to offer an "open map" action without parsing or
/// rebuilding URLs themselves. A non-location body has no URL, even if it
/// happens to contain one.
#[must_use]
pub fn location_message_url(content: &str) -> Option<String> {
    // Only a canonical body may become an actionable map link. A chat message
    // that merely starts with the marker (or carries a hand-written URL) must
    // never masquerade as a device fix.
    parse_location_message(content)?;
    content
        .split_whitespace()
        .find(|part| part.starts_with(MAP_URL_PREFIX))
        .map(str::to_owned)
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
    let settings = validate_location_settings(&LocationSessionSettings {
        mode: LocationMode::parse(mode)?,
        duration_secs,
        sensor_interval_secs: MIN_SENSOR_INTERVAL_SECS,
        send_interval_secs: MIN_SEND_INTERVAL_SECS,
        motion_gate: false,
    })?;
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

/// One point parsed back from a canonical location DM body.
///
/// Flat and primitive-only so FRB/web can hand a whole track to a frontend
/// without duplicating the body grammar there. `timestamp_secs` is the
/// *sender's* sampling time embedded in the body, not the local row's
/// `created_at`, so a GPX track keeps the device's clock even after forwarding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationTrackPoint {
    /// Latitude in decimal degrees.
    pub latitude: f64,
    /// Longitude in decimal degrees.
    pub longitude: f64,
    /// Estimated horizontal accuracy in metres.
    pub accuracy: f64,
    /// Unix seconds at which the sending device sampled this point.
    pub timestamp_secs: f64,
    /// Monotonic sample number within the sending session.
    pub sequence: u32,
    /// Whether this was the final point of the session.
    pub is_final: bool,
}

/// Parse one canonical location body back into a track point.
///
/// Returns `None` for ordinary chat text and for a malformed body (a message
/// that merely starts with the marker is not enough). The grammar is shared
/// with [`LocationShare::message_body`], so Rust is the only owner of both
/// halves and the frontends never implement a second parser.
#[must_use]
pub fn parse_location_message(content: &str) -> Option<LocationTrackPoint> {
    let content = content.trim_start();
    let rest = content.strip_prefix(MESSAGE_PREFIX)?.trim_start();
    let coords = rest.split_whitespace().next()?;
    let (lat, lon) = coords.split_once(',')?;
    let latitude = Latitude::new(lat.parse().ok()?).ok()?.value();
    let longitude = Longitude::new(lon.parse().ok()?).ok()?.value();

    // The accuracy span contains a space (`(±12 m)`), so it must be read from
    // the original string rather than from `split_whitespace()` tokens.
    let after_coords = rest.get(coords.len()..)?.trim_start();
    let accuracy_span = after_coords.split(" m)").next()?;
    let accuracy = accuracy_span.strip_prefix("(±")?.parse::<f64>().ok()?;
    if !accuracy.is_finite() || accuracy < 0.0 {
        return None;
    }
    let after_accuracy = after_coords
        .get(accuracy_span.len().saturating_add(" m)".len())..)?
        .trim_start();
    let mut trailing = after_accuracy.split_whitespace();
    let timestamp_token = trailing.next()?;
    let timestamp_secs = timestamp_token.strip_prefix('@')?.parse::<f64>().ok()?;
    if !timestamp_secs.is_finite() {
        return None;
    }
    // A body built by `build_location_message` always carries its canonical
    // map URL immediately after the timestamp. Requiring it here keeps the
    // sender boundary honest: a hand-written prefix plus coordinates is not a
    // location update and must not enter the track store.
    let map_url = trailing.next()?;
    if map_url != location_map_url(latitude, longitude) {
        return None;
    }

    let mut sequence = 1;
    let mut is_final = false;
    for part in after_accuracy.split_whitespace().skip(1) {
        // OSM URLs carry their fragment (`#map=...`) inside a single token, so
        // only a standalone numeric `#N` token is a sequence number.
        if let Some(value) = part.strip_prefix('#') {
            sequence = value.parse().ok()?;
        }
        if part == "(final)" {
            is_final = true;
        }
    }
    Some(LocationTrackPoint {
        latitude,
        longitude,
        accuracy,
        timestamp_secs,
        sequence,
        is_final,
    })
}

/// Parse a user-facing time filter into Unix seconds.
///
/// Supported forms: `now`, `<n> <unit> ago` (seconds/minutes/hours/days/weeks),
/// RFC 3339 (`2026-11-27T10:00:00Z`), `YYYY-MM-DD[T ]HH:MM:SS`, `YYYY-MM-DD`,
/// and a bare epoch-seconds number. Rust owns this grammar so "24 hours ago"
/// means the same in the TUI and the browser.
///
/// # Errors
/// Returns a human-readable error for an unrecognised filter.
pub fn parse_location_time_filter(text: &str, now: f64) -> Result<f64, String> {
    let value = text.trim();
    if value.eq_ignore_ascii_case("now") {
        return Ok(now);
    }
    let lower = value.to_ascii_lowercase();
    let words: Vec<&str> = lower.split_whitespace().collect();
    if let [amount, unit, "ago"] = words.as_slice() {
        let amount: u64 = amount
            .parse()
            .map_err(|_| format!("invalid relative time '{value}'"))?;
        let unit_secs = match *unit {
            "second" | "seconds" => 1,
            "minute" | "minutes" => 60,
            "hour" | "hours" => 3_600,
            "day" | "days" => 86_400,
            "week" | "weeks" => 604_800,
            other => {
                return Err(format!(
                    "unknown time unit '{other}' (use seconds, minutes, hours, days or weeks)"
                ));
            }
        };
        let delta = amount
            .saturating_mul(unit_secs)
            .to_f64()
            .ok_or_else(|| "relative time is too large".to_string())?;
        return Ok(now - delta);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return dt
            .timestamp()
            .to_f64()
            .ok_or_else(|| "timestamp is out of range".to_string());
    }
    for format in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(dt) = NaiveDateTime::parse_from_str(value, format) {
            return dt
                .and_utc()
                .timestamp()
                .to_f64()
                .ok_or_else(|| "timestamp is out of range".to_string());
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        && let Some(dt) = date.and_hms_opt(0, 0, 0)
    {
        return dt
            .and_utc()
            .timestamp()
            .to_f64()
            .ok_or_else(|| "timestamp is out of range".to_string());
    }
    value
        .parse::<f64>()
        .ok()
        .filter(|epoch| epoch.is_finite())
        .ok_or_else(|| {
            format!("unrecognised time '{value}' (use 'now', '24 hours ago', or an ISO date)")
        })
}

/// Escape text for inclusion in an XML element or attribute.
fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Render a device's points as a GPX 1.1 track.
///
/// The device label is escaped, timestamps use RFC 3339 UTC, and accuracy is
/// carried in a GPX `<extensions>` element (GPX has no accuracy field). Frontends
/// write the returned string verbatim: no frontend builds GPX itself.
#[must_use]
pub fn location_gpx(
    points: impl AsRef<[LocationTrackPoint]>,
    device_label: impl AsRef<str>,
) -> String {
    let points = points.as_ref();
    let label = xml_escape(device_label.as_ref());
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <gpx version=\"1.1\" creator=\"p2p-chat\" xmlns=\"http://www.topografix.com/GPX/1/1\">\n",
    );
    let _ = writeln!(out, "  <metadata><name>{label}</name></metadata>");
    let _ = writeln!(out, "  <trk><name>{label}</name><trkseg>");
    for point in points {
        let secs = point.timestamp_secs.to_i64().unwrap_or_default();
        let time = DateTime::<Utc>::from_timestamp(secs, 0).map_or_else(String::new, |dt| {
            dt.to_rfc3339_opts(SecondsFormat::Secs, true)
        });
        let _ = writeln!(
            out,
            "    <trkpt lat=\"{:.6}\" lon=\"{:.6}\">\
             <time>{time}</time>\
             <extensions><accuracy>{:.1}</accuracy></extensions>\
             </trkpt>",
            point.latitude, point.longitude, point.accuracy
        );
    }
    out.push_str("  </trkseg></trk>\n</gpx>\n");
    out
}

/// Build an OpenStreetMap directions URL showing the newest points of a track.
///
/// OSM directions accepts a semicolon-separated `route` of `lat,lon` pairs but
/// caps the number of waypoints, so only the newest [`MAX_TRACK_URL_POINTS`]
/// points are kept (oldest of that window first). A single point falls back to
/// the ordinary marker URL. GPX remains the lossless export; this URL is the
/// "show the last N positions" convenience.
#[must_use]
pub fn location_track_url(points: impl AsRef<[LocationTrackPoint]>) -> Option<String> {
    let points = points.as_ref();
    let newest = points
        .iter()
        .rev()
        .take(MAX_TRACK_URL_POINTS)
        .rev()
        .collect::<Vec<_>>();
    match newest.as_slice() {
        [] => None,
        [only] => Some(location_map_url(only.latitude, only.longitude)),
        many => {
            let route = many
                .iter()
                .map(|point| format!("{:.6},{:.6}", point.latitude, point.longitude))
                .collect::<Vec<_>>()
                .join(";");
            Some(format!(
                "https://www.openstreetmap.org/directions?engine=fossgis_osrm_foot&route={route}"
            ))
        }
    }
}

/// One selectable destination device for a location share.
///
/// Flat and primitive-only for FRB/web: the account/device filtering stays in
/// Rust, while the frontends only render the label and pass peer ids back.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationRecipient {
    /// Peer ID of another device bound into this install's user.
    pub peer_id: String,
    /// Canonical display label resolved through the same name cache as Contacts.
    pub display_name: String,
}

/// Every *other* device bound into this install's user, sorted by peer id.
///
/// Uses [`crate::db::existing_local_peer_id`] rather than generating an identity
/// as a side effect of a read path. Database errors fail closed (no recipients)
/// so a broken store cannot silently share with a peer outside the account.
fn other_user_device_ids() -> Result<Vec<String>, String> {
    let Some(local) = crate::db::existing_local_peer_id() else {
        return Ok(Vec::new());
    };
    let local = local.to_string();
    let Some(user_id) =
        crate::identity::find_user_id_for_peer(&local).map_err(|e| e.to_string())?
    else {
        return Ok(Vec::new());
    };
    let mut devices: Vec<String> = crate::identity::list_user_devices(&user_id)
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|device| device.peer_id)
        .filter(|peer_id| *peer_id != local)
        .collect();
    devices.sort();
    devices.dedup();
    Ok(devices)
}

/// List the devices a location share may target, with Rust-resolved labels.
///
/// # Errors
/// Returns a human-readable error if the device/identity tables cannot be read.
pub fn list_location_recipients() -> Result<Vec<LocationRecipient>, String> {
    other_user_device_ids().map(|devices| {
        devices
            .into_iter()
            .map(|peer_id| LocationRecipient {
                display_name: crate::nickname::get_peer_display_name_or_short(&peer_id),
                peer_id,
            })
            .collect()
    })
}

/// Resolve the devices a location share may target: every *other* device bound
/// to this install's user, or a validated subset of those devices.
///
/// This is the device-centric counterpart of the user-level contact roster:
/// only a device bound to the local account is eligible, and the local device
/// is always excluded (sharing to yourself is pointless). The caller passes
/// `all_devices` for "all", or explicit peer ids for specific devices.
///
/// # Errors
/// Returns an error when there is no bound user, when no other device exists,
/// or when a selected peer id is not another device of this user.
pub fn location_recipient_peer_ids(
    selected: impl AsRef<[String]>,
    all_devices: bool,
) -> Result<Vec<String>, String> {
    let selected = selected.as_ref();
    let devices = other_user_device_ids()?;
    if all_devices {
        return if devices.is_empty() {
            Err("no other device is bound to your user".to_string())
        } else {
            Ok(devices)
        };
    }
    if selected.is_empty() {
        return Err("select at least one device (or share with all)".to_string());
    }
    selected
        .iter()
        .map(|peer_id| {
            devices
                .iter()
                .find(|device| *device == peer_id)
                .cloned()
                .ok_or_else(|| format!("{peer_id} is not another device of your user"))
        })
        .collect()
}

#[cfg(test)]
#[path = "../tests/unit/unit_location.rs"]
mod tests;
