use super::*;
use crate::tui::test_helpers::test_app_state;
use chrono::NaiveDateTime;
use p2p_app::generated::models_queryable::Message;

fn msg(
    content: &str,
    peer_id: Option<&str>,
    sender_nickname: Option<&str>,
    msg_id: Option<&str>,
    sent_at: Option<f64>,
    created_at: &str,
) -> Message {
    let dt = NaiveDateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S").unwrap();
    Message {
        id: 0,
        created_at: dt,
        content: content.to_string(),
        peer_id: peer_id.map(String::from),
        topic: "test".to_string(),
        sent: 0,
        is_direct: 0,
        target_peer: None,
        msg_id: msg_id.map(String::from),
        sent_at,
        sender_nickname: sender_nickname.map(String::from),
    }
}

#[test]
fn test_format_messages_from_db_empty() {
    let (msgs, ids, sent_at) = format_messages_from_db(&[], &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs.is_empty());
    assert!(ids.is_empty());
    assert!(sent_at.is_empty());
}

#[test]
fn test_format_outgoing_without_sender_nickname() {
    let messages = [msg(
        "hello",
        None,
        None,
        Some("m1"),
        Some(1.0),
        "2024-01-01 12:00:00",
    )];
    let (msgs, ids, sent_at) =
        format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].text.contains("[Me]"));
    assert!(msgs[0].text.contains("hello"));
    assert_eq!(ids[0], Some("m1".to_string()));
    assert_eq!(sent_at.get("m1"), Some(&1.0));
}

#[test]
fn test_format_outgoing_with_sender_nickname() {
    let messages = [msg(
        "hello",
        None,
        Some("OldNick"),
        Some("m1"),
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, _, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs[0].text.contains("[OldNick]"));
    assert!(msgs[0].text.contains("hello"));
}

#[test]
fn test_format_incoming_without_sender_nickname_falls_back_to_display_name() {
    let messages = [msg(
        "hi there",
        Some("peer-abc"),
        None,
        None,
        None,
        "2024-01-01 12:00:00",
    )];
    let local = HashMap::from([("peer-abc".to_string(), "Alice".to_string())]);
    let (msgs, _, _) = format_messages_from_db(&messages, &local, &HashMap::new(), "Me");
    assert!(msgs[0].text.contains("[Alice]"));
    assert!(msgs[0].text.contains("hi there"));
}

#[test]
fn test_format_incoming_with_sender_nickname() {
    let messages = [msg(
        "hey",
        Some("peer-abc"),
        Some("Bob"),
        None,
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, _, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs[0].text.contains("[Bob]"));
    assert!(msgs[0].text.contains("hey"));
}

#[test]
fn test_format_messages_reverses_newest_first_to_oldest_first() {
    let messages = vec![
        msg(
            "second",
            Some("p1"),
            None,
            None,
            None,
            "2024-01-01 12:00:01",
        ),
        msg("first", Some("p1"), None, None, None, "2024-01-01 12:00:00"),
    ];
    let (msgs, _, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 2);
    assert!(
        msgs[0].text.contains("first"),
        "first msg should be oldest after rev"
    );
    assert!(
        msgs[1].text.contains("second"),
        "second msg should be newest after rev"
    );
}

#[test]
fn test_format_messages_sent_at_skipped_when_none() {
    let messages = [msg(
        "no sent_at",
        None,
        None,
        Some("m1"),
        None,
        "2024-01-01 12:00:00",
    )];
    let (_, _, sent_at) =
        format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert!(sent_at.is_empty());
}

#[test]
fn test_format_messages_peer_id_maps_correctly() {
    let messages = [msg(
        "incoming",
        Some("p1"),
        None,
        None,
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, _, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs[0].sender_peer_id, Some("p1".to_string()));
}

#[test]
fn test_format_messages_outgoing_peer_id_is_none() {
    let messages = [msg(
        "outgoing",
        None,
        None,
        None,
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, _, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs[0].sender_peer_id, None);
}

// ── AppState ────────────────────────────────────────────────────────────

#[test]
fn test_cancel_nickname_edit_when_active() {
    let mut state = test_app_state();
    state.editing_nickname = true;
    state.editing_nickname_peer = Some("peer-1".to_string());
    let mut ta = TextArea::default();
    ta.insert_str("something");
    state.chat_input = ta;

    state.cancel_nickname_edit();

    assert!(!state.editing_nickname);
    assert_eq!(state.editing_nickname_peer, None);
    assert!(state.chat_input.lines().join("").is_empty());
}

#[test]
fn test_cancel_nickname_edit_when_inactive_is_noop() {
    let mut state = test_app_state();
    assert!(!state.editing_nickname);
    state.cancel_nickname_edit();
    assert!(!state.editing_nickname);
    assert_eq!(state.editing_nickname_peer, None);
}

#[test]
fn test_app_state_new_defaults() {
    let state = test_app_state();
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.peer_selection, 0);
    assert!(state.mouse_capture);
    assert!(state.chat_auto_scroll);
    assert!(state.log_auto_scroll);
    assert_eq!(state.topic_str, "test-net");
    assert_eq!(state.own_nickname, "TestUser");
    assert!(!state.editing_nickname);
    assert_eq!(state.popup, None);
    assert_eq!(state.broadcast_selection, None);
}
