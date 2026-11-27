//! DB-backed integration coverage for device-centric location sharing.
//!
//! The unit suite owns the pure grammar/policy checks; these tests exercise the
//! real SQLite path: canonical bodies persisted as DMs are read back through
//! [`p2p_app::messages::load_location_track`] (and its FRB/web wrapper), and
//! recipient resolution reads real device-binding rows.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented
)]

mod test_utils;

use p2p_app::location::{self, LocationShareInput};
use p2p_app::messages::MessageMeta;
use serial_test::serial;
use test_utils::with_test_db;

/// 2023-11-14T22:13:20Z, matching the unit suite's canonical timestamp fixture.
const BASE_TIME: f64 = 1_700_000_000.0;

fn share_input(sequence: u32, latitude: f64) -> LocationShareInput {
    LocationShareInput {
        latitude,
        longitude: 13.405,
        accuracy: 12.0,
        mode: "once".to_string(),
        duration_secs: None,
        sensor_interval_secs: location::DEFAULT_SENSOR_INTERVAL_SECS,
        send_interval_secs: location::DEFAULT_SEND_INTERVAL_SECS,
        motion_gate: false,
        session_id: "integration-session".to_string(),
        sequence,
        is_final: false,
    }
}

/// Canonical body from Rust, with only the wall-clock marker replaced so the
/// time-window assertions can use fixed, RFC 3339 formatted bounds. The body
/// grammar still comes from `build_location_message`; no frontend parser is
/// involved in producing these rows.
fn canonical_body_at(sequence: u32, latitude: f64, timestamp_secs: f64) -> String {
    let body = p2p_app::api::build_location_message(share_input(sequence, latitude))
        .expect("canonical location body");
    let marker = body
        .split_whitespace()
        .find(|part| part.starts_with('@'))
        .expect("canonical body carries a timestamp marker");
    let body = body.replacen(marker, &format!("@{timestamp_secs:.0}"), 1);
    assert!(
        location::parse_location_message(&body).is_some(),
        "body must stay canonical: {body}"
    );
    body
}

fn save_incoming_dm(
    sender: &str,
    target: &str,
    body: &str,
    timestamp_secs: f64,
    msg_id: &str,
) -> i32 {
    let meta = MessageMeta {
        msg_id: Some(msg_id.to_string()),
        sent_at: Some(timestamp_secs),
        ..MessageMeta::default()
    };
    p2p_app::messages::save_message_with_meta(body, Some(sender), "chat", true, Some(target), meta)
        .expect("save incoming DM")
        .id
}

#[test]
#[serial]
fn load_location_track_filters_by_device_direction_and_time_window() {
    with_test_db(|| {
        p2p_app::identity::provision_my_user(Some("alice"), Some("Alice")).expect("provision");
        let local = p2p_app::get_local_peer_id()
            .expect("local peer id")
            .to_string();
        let sender = "12D3KooSenderDevice";
        let sibling = "12D3KooSiblingDevice";
        // Both sending devices belong to the same account: the track query
        // must still key on the exact peer id, never on the user.
        p2p_app::identity::mint_device_binding(sender, [0x66u8; 32]).expect("bind sender");
        p2p_app::identity::mint_device_binding(sibling, [0x77u8; 32]).expect("bind sibling");

        // Inserted out of order on purpose: the loader sorts by embedded
        // timestamp then sequence, not by insertion order.
        save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(3, 52.53, BASE_TIME + 2.0),
            BASE_TIME + 2.0,
            "loc-third",
        );
        save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(1, 52.51, BASE_TIME),
            BASE_TIME,
            "loc-first",
        );
        save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(2, 52.52, BASE_TIME + 1.0),
            BASE_TIME + 1.0,
            "loc-second",
        );
        // Both endpoints of the Rust-parsed ISO window are exclusive of these.
        save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(9, 1.0, BASE_TIME - 100.0),
            BASE_TIME - 100.0,
            "loc-too-old",
        );
        save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(10, 2.0, BASE_TIME + 100.0),
            BASE_TIME + 100.0,
            "loc-too-new",
        );
        // Another device of the *same* user must never leak into the track.
        save_incoming_dm(
            sibling,
            &local,
            &canonical_body_at(1, 42.0, BASE_TIME + 1.0),
            BASE_TIME + 1.0,
            "loc-sibling",
        );
        // Plain chat DMs are not location points even though they carry a peer.
        save_incoming_dm(
            sender,
            &local,
            "just a chat message",
            BASE_TIME + 1.0,
            "chat-not-location",
        );

        let outgoing = save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(4, 60.0, BASE_TIME + 1.0),
            BASE_TIME + 1.0,
            "loc-outgoing",
        );
        p2p_app::messages::mark_message_sent(outgoing).expect("mark outgoing sent");
        let tombstoned = save_incoming_dm(
            sender,
            &local,
            &canonical_body_at(5, 61.0, BASE_TIME + 1.0),
            BASE_TIME + 1.0,
            "loc-tombstoned",
        );
        p2p_app::messages::mark_message_sent(tombstoned).expect("mark tombstone sent");
        assert!(
            p2p_app::messages::mark_message_deleted(tombstoned).expect("tombstone outgoing"),
            "a sent row must be deletable"
        );

        let meta = MessageMeta {
            msg_id: Some("loc-broadcast".to_string()),
            sent_at: Some(BASE_TIME + 1.0),
            ..MessageMeta::default()
        };
        p2p_app::messages::save_message_with_meta(
            &canonical_body_at(6, 62.0, BASE_TIME + 1.0),
            Some(sender),
            "chat",
            false,
            None,
            meta,
        )
        .expect("save broadcast");

        let from = "2023-11-14T22:13:19Z";
        let to = "2023-11-14T22:13:22Z";
        let points = p2p_app::messages::load_location_track(
            sender.to_string(),
            Some(from.to_string()),
            Some(to.to_string()),
        )
        .expect("track from messages loader");

        assert_eq!(
            points
                .iter()
                .map(|point| point.sequence)
                .collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        assert_eq!(
            points
                .iter()
                .map(|point| point.timestamp_secs)
                .collect::<Vec<_>>(),
            vec![BASE_TIME, BASE_TIME + 1.0, BASE_TIME + 2.0]
        );
        assert_eq!(
            points
                .iter()
                .map(|point| point.latitude)
                .collect::<Vec<_>>(),
            vec![52.51, 52.52, 52.53]
        );
        assert!(!points.iter().any(|point| point.is_final));

        // The FRB/web facade takes owned strings and must be the same query.
        let via_api = p2p_app::api::load_location_track(
            sender.to_string(),
            Some(from.to_string()),
            Some(to.to_string()),
        )
        .expect("track from api wrapper");
        assert_eq!(via_api, points);
    });
}

#[test]
#[serial]
fn location_recipients_are_other_devices_of_the_local_user_only() {
    with_test_db(|| {
        assert!(
            location::list_location_recipients()
                .expect("recipients before provisioning")
                .is_empty(),
            "no identity means no recipients"
        );

        p2p_app::identity::provision_my_user(Some("alice"), Some("Alice")).expect("provision");
        let local = p2p_app::get_local_peer_id()
            .expect("local peer id")
            .to_string();

        assert!(
            location::list_location_recipients()
                .expect("recipients with only the local device")
                .is_empty(),
            "the local device is never a recipient"
        );
        assert!(
            location::location_recipient_peer_ids(&[], true).is_err(),
            "all_devices must fail while no other device is bound"
        );
        assert!(
            location::location_recipient_peer_ids(std::slice::from_ref(&local), false).is_err(),
            "the local device is not a valid explicit target"
        );
        let unbound = "12D3KooUnboundPeer".to_string();
        assert!(
            location::location_recipient_peer_ids(std::slice::from_ref(&unbound), false).is_err(),
            "an unbound peer must be rejected"
        );

        let second_device = "12D3KooSecondDevice";
        p2p_app::identity::mint_device_binding(second_device, [0x99u8; 32]).expect("bind device");

        let recipients = location::list_location_recipients().expect("recipients after binding");
        assert_eq!(
            recipients
                .iter()
                .map(|recipient| recipient.peer_id.as_str())
                .collect::<Vec<_>>(),
            vec![second_device]
        );
        assert!(
            recipients
                .iter()
                .all(|recipient| !recipient.display_name.is_empty()),
            "labels are resolved in Rust, not left to the frontend"
        );

        assert_eq!(
            location::location_recipient_peer_ids(&[], true).expect("all devices"),
            vec![second_device.to_string()]
        );
        assert_eq!(
            location::location_recipient_peer_ids(&[second_device.to_string()], false)
                .expect("explicit device"),
            vec![second_device.to_string()]
        );
        assert!(
            location::location_recipient_peer_ids(&[unbound], false).is_err(),
            "an explicitly named unbound peer stays rejected"
        );
        assert!(
            location::location_recipient_peer_ids(&[local], false).is_err(),
            "the local peer stays excluded"
        );
    });
}
