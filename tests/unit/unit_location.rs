//! Unit coverage for Rust-owned location-sharing policy.

use super::*;

fn input(
    latitude: f64,
    longitude: f64,
    mode: &str,
    duration_secs: Option<u32>,
) -> LocationShareInput {
    LocationShareInput {
        latitude,
        longitude,
        accuracy: 12.5,
        mode: mode.to_string(),
        duration_secs,
        sensor_interval_secs: 10,
        send_interval_secs: 60,
        motion_gate: true,
        session_id: "session-1".to_string(),
        sequence: 1,
        is_final: false,
    }
}

#[test]
fn latitude_and_longitude_reject_out_of_range_and_non_finite() {
    assert!(Latitude::new(90.0).is_ok());
    assert!(Latitude::new(-90.0).is_ok());
    assert!(Latitude::new(90.1).is_err());
    assert!(Latitude::new(f64::NAN).is_err());
    assert!(Longitude::new(180.0).is_ok());
    assert!(Longitude::new(-180.1).is_err());
    assert!(Longitude::new(f64::INFINITY).is_err());
}

#[test]
fn mode_parses_and_round_trips() {
    for (name, mode) in [
        ("once", LocationMode::Once),
        ("limited", LocationMode::Limited),
        ("unlimited", LocationMode::Unlimited),
    ] {
        assert_eq!(LocationMode::parse(name).expect("mode"), mode);
        assert_eq!(mode.as_str(), name);
    }
    assert!(LocationMode::parse("sometimes").is_err());
}

#[test]
fn canonical_map_url_contains_both_marker_and_centre() {
    let url = location_map_url(52.52, 13.405);
    assert!(url.starts_with(MAP_URL_PREFIX), "{url}");
    assert!(url.contains("mlat=52.520000&mlon=13.405000"), "{url}");
    assert!(url.contains("#map=17/52.520000/13.405000"), "{url}");
}

#[test]
fn message_body_is_canonical_and_detectable() {
    let body = build_location_message(input(52.52, 13.405, "once", None)).expect("body");
    assert!(body.starts_with(MESSAGE_PREFIX), "{body}");
    assert!(body.contains("52.520000,13.405000"), "{body}");
    assert!(body.contains("±12 m"), "{body}");
    assert!(body.contains("(once)"), "{body}");
    assert!(is_location_message(&body));
    assert!(!is_location_message("hello there"));
}

#[test]
fn rendered_location_lines_yield_the_canonical_url_despite_display_markers() {
    let body = build_location_message(input(52.52, 13.405, "once", None)).expect("body");
    let expected = location_message_url(&body).expect("canonical map url");
    let rendered = |body: &str| format!("12:34:56  [alice] {body}");

    assert_eq!(
        location_url_from_display_line(&rendered(&body)).as_deref(),
        Some(expected.as_str())
    );
    // A sealed DM is decrypted before display but keeps the `[e2e] ` marker,
    // and an edited row keeps ` (edited)`. Both are presentation-only.
    assert_eq!(
        location_url_from_display_line(&rendered(&format!("[e2e] {body}"))).as_deref(),
        Some(expected.as_str())
    );
    assert_eq!(
        location_url_from_display_line(&rendered(&format!("{body} (edited)"))).as_deref(),
        Some(expected.as_str())
    );
    // A hand-written URL that merely looks like a fix must stay inert.
    let forged =
        "[Location] 1.0,2.0 (±1 m) @1 https://www.openstreetmap.org/?mlat=1&mlon=2#map=17/1/2";
    assert_eq!(location_url_from_display_line(&rendered(forged)), None);
    assert_eq!(
        location_url_from_display_line(&rendered("hello there")),
        None
    );
}

#[test]
fn limited_session_requires_a_bounded_duration() {
    assert!(build_location_message(input(0.0, 0.0, "limited", None)).is_err());
    assert!(build_location_message(input(0.0, 0.0, "limited", Some(30))).is_err());
    let body = build_location_message(input(0.0, 0.0, "limited", Some(900))).expect("body");
    assert!(body.contains("(limited 15m"), "{body}");
    assert!(body.contains("motion-gated"), "{body}");
}

#[test]
fn non_continued_modes_drop_the_duration() {
    let share = LocationShare::validate(input(0.0, 0.0, "once", Some(999))).expect("share");
    assert_eq!(share.session.duration_secs, None);
    let share = LocationShare::validate(input(0.0, 0.0, "unlimited", Some(999))).expect("share");
    assert_eq!(share.session.duration_secs, None);
}

#[test]
fn validation_rejects_bad_input() {
    let mut bad = input(91.0, 0.0, "once", None);
    assert!(build_location_message(bad.clone()).is_err());
    bad = input(0.0, 0.0, "once", None);
    bad.accuracy = -1.0;
    assert!(build_location_message(bad.clone()).is_err());
    bad = input(0.0, 0.0, "once", None);
    bad.sensor_interval_secs = 0;
    assert!(build_location_message(bad.clone()).is_err());
    bad = input(0.0, 0.0, "once", None);
    bad.send_interval_secs = MAX_SEND_INTERVAL_SECS + 1;
    assert!(build_location_message(bad.clone()).is_err());
    bad = input(0.0, 0.0, "once", None);
    bad.sequence = 0;
    assert!(build_location_message(bad.clone()).is_err());
    bad = input(0.0, 0.0, "once", None);
    bad.session_id.clear();
    assert!(build_location_message(bad.clone()).is_err());
}

#[test]
fn sequence_and_final_markers_are_rendered() {
    let mut share = input(1.0, 2.0, "unlimited", None);
    share.sequence = 4;
    share.is_final = true;
    let body = build_location_message(share).expect("body");
    assert!(body.contains("#4"), "{body}");
    assert!(body.ends_with("(final)"), "{body}");
}

#[test]
fn expiry_policy_matches_mode() {
    let once = LocationSessionSettings {
        mode: LocationMode::Once,
        duration_secs: None,
        sensor_interval_secs: 10,
        send_interval_secs: 60,
        motion_gate: false,
    };
    assert!(location_session_expired(0.0, &once, 0.0));

    let unlimited = LocationSessionSettings {
        mode: LocationMode::Unlimited,
        ..once
    };
    assert!(!location_session_expired(0.0, &unlimited, 100_000.0));

    let limited = LocationSessionSettings {
        mode: LocationMode::Limited,
        duration_secs: Some(900),
        ..once
    };
    assert!(!location_session_expired(100.0, &limited, 999.0));
    assert!(location_session_expired(100.0, &limited, 1_000.0));
}

#[test]
fn session_active_helper_uses_the_same_limited_duration_validation() {
    assert!(location_session_active(100.0, "limited", None, 110.0).is_err());
    assert!(location_session_active(100.0, "limited", Some(30), 110.0).is_err());
    assert!(location_session_active(100.0, "limited", Some(900), 110.0).expect("still active"));
    assert!(
        !location_session_active(100.0, "limited", Some(900), 1_000.0).expect("expired session")
    );
    assert!(location_session_active(100.0, "sometimes", None, 110.0).is_err());
}

#[test]
fn cadence_helpers_are_due_only_at_or_after_the_interval() {
    assert!(location_send_due(None, 0.0, 60));
    assert!(!location_send_due(Some(100.0), 159.9, 60));
    assert!(location_send_due(Some(100.0), 160.0, 60));
    assert!(location_query_due(Some(100.0), 110.0, 10));
}

#[test]
fn canonical_body_parses_back_with_sequence_and_final_marker() {
    let mut share = input(52.52, 13.405, "unlimited", None);
    share.accuracy = 12.0;
    share.sequence = 7;
    share.is_final = true;
    let body = build_location_message(share).expect("body");
    let point = parse_location_message(&body).expect("canonical point");
    assert!((point.latitude - 52.52).abs() < 0.000_001);
    assert!((point.longitude - 13.405).abs() < 0.000_001);
    // The body rounds accuracy to whole metres; the parser must read the
    // `(±12 m)` span as one token rather than splitting on its space.
    assert!((point.accuracy - 12.0).abs() < f64::EPSILON);
    assert!(point.timestamp_secs > 0.0);
    assert_eq!(point.sequence, 7);
    assert!(point.is_final);
    assert_eq!(
        location_message_url(&body).as_deref(),
        Some(
            "https://www.openstreetmap.org/?mlat=52.520000&mlon=13.405000#map=17/52.520000/13.405000"
        )
    );
    assert!(parse_location_message("just a chat message").is_none());
    assert!(parse_location_message("[Location] nonsense").is_none());
    assert!(
        parse_location_message("[Location] 52.520000,13.405000 (±12 m) @1700000000").is_none(),
        "a hand-written prefix without the canonical map URL is not a track point"
    );
    assert_eq!(location_message_url("see https://example.test"), None);
    assert_eq!(
        location_message_url(
            "[Location] 52.520000,13.405000 (±12 m) @1700000000 https://example.test"
        ),
        None,
        "a hand-written URL must not become an actionable map link"
    );
}

#[test]
fn time_filter_accepts_relative_absolute_and_epoch_forms() {
    let now = 1_700_000_000.0;
    assert_eq!(parse_location_time_filter("now", now).expect("now"), now);
    assert_eq!(
        parse_location_time_filter("24 hours ago", now).expect("relative"),
        now - 86_400.0
    );
    assert_eq!(
        parse_location_time_filter("2 weeks ago", now).expect("weeks"),
        now - 1_209_600.0
    );
    assert_eq!(
        parse_location_time_filter("2023-11-14T22:13:20Z", now).expect("rfc3339"),
        1_700_000_000.0
    );
    assert_eq!(
        parse_location_time_filter("1700000000", now).expect("epoch"),
        now
    );
    assert!(parse_location_time_filter("tomorrow", now).is_err());
    assert!(parse_location_time_filter("5 fortnights ago", now).is_err());
}

#[test]
fn gpx_escapes_label_and_emits_rfc3339_times() {
    let points = vec![
        LocationTrackPoint {
            latitude: 52.52,
            longitude: 13.405,
            accuracy: 12.5,
            timestamp_secs: 1_700_000_000.0,
            sequence: 1,
            is_final: false,
        },
        LocationTrackPoint {
            latitude: -1.0,
            longitude: 2.0,
            accuracy: 3.0,
            timestamp_secs: 1_700_000_060.0,
            sequence: 2,
            is_final: true,
        },
    ];
    let gpx = location_gpx(&points, "Laptop & <phone>");
    assert!(
        gpx.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
        "{gpx}"
    );
    assert!(
        gpx.contains("<name>Laptop &amp; &lt;phone&gt;</name>"),
        "{gpx}"
    );
    assert!(gpx.contains("<time>2023-11-14T22:13:20Z</time>"), "{gpx}");
    assert!(gpx.contains("<accuracy>12.5</accuracy>"), "{gpx}");
    assert_eq!(gpx.matches("<trkpt").count(), 2);
    assert!(gpx.trim_end().ends_with("</gpx>"));
}

#[test]
fn track_url_caps_waypoints_and_single_point_uses_marker() {
    assert_eq!(location_track_url(&[]), None);
    let one = vec![LocationTrackPoint {
        latitude: 1.0,
        longitude: 2.0,
        accuracy: 5.0,
        timestamp_secs: 1.0,
        sequence: 1,
        is_final: false,
    }];
    assert_eq!(location_track_url(&one), Some(location_map_url(1.0, 2.0)));
    let points: Vec<LocationTrackPoint> = (0..60)
        .map(|index| LocationTrackPoint {
            latitude: f64::from(index),
            longitude: 0.0,
            accuracy: 5.0,
            timestamp_secs: f64::from(index),
            sequence: index + 1,
            is_final: false,
        })
        .collect();
    let url = location_track_url(&points).expect("route URL");
    assert!(url.contains("engine=fossgis_osrm_foot"), "{url}");
    let route = url.split_once("route=").map_or("", |(_, route)| route);
    assert_eq!(route.split(';').count(), MAX_TRACK_URL_POINTS);
    assert!(route.starts_with("10.000000,0.000000"), "{route}");
    assert!(route.ends_with("59.000000,0.000000"), "{route}");
}
