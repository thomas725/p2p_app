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
fn cadence_helpers_are_due_only_at_or_after_the_interval() {
    assert!(location_send_due(None, 0.0, 60));
    assert!(!location_send_due(Some(100.0), 159.9, 60));
    assert!(location_send_due(Some(100.0), 160.0, 60));
    assert!(location_query_due(Some(100.0), 110.0, 10));
}
