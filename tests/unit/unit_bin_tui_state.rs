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
    let (msgs, ids) = format_messages_from_db(&[], &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs.is_empty());
    assert!(ids.is_empty());
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
    let (msgs, ids) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].text.contains("[Me]"));
    assert!(msgs[0].text.contains("hello"));
    assert_eq!(ids[0], Some("m1".to_string()));
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
    let (msgs, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
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
    let (msgs, _) = format_messages_from_db(&messages, &local, &HashMap::new(), "Me");
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
    let (msgs, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
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
    let (msgs, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
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
fn test_format_messages_skipped_when_sent_at_none() {
    let messages = [msg(
        "no sent_at",
        None,
        None,
        Some("m1"),
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, ids) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 1);
    assert_eq!(ids[0], Some("m1".to_string()));
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
    let (msgs, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
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
    let (msgs, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs[0].sender_peer_id, None);
}

// ── AppState ────────────────────────────────────────────────────────────

#[test]
fn test_cancel_nickname_edit_when_active() {
    let mut state = test_app_state();
    state.input_mode = p2p_app::InputMode::EditingNickname;
    state.editing_nickname_peer = Some("peer-1".to_string());
    let mut ta = TextArea::default();
    ta.insert_str("something");
    state.chat_input = ta;

    state.cancel_nickname_edit();

    assert!(!state.is_editing_nickname());
    assert_eq!(state.editing_nickname_peer, None);
    assert!(state.chat_input.lines().join("").is_empty());
}

#[test]
fn test_cancel_nickname_edit_when_inactive_is_noop() {
    let mut state = test_app_state();
    assert!(!state.is_editing_nickname());
    state.cancel_nickname_edit();
    assert!(!state.is_editing_nickname());
    assert_eq!(state.editing_nickname_peer, None);
}

#[test]
fn test_app_state_new_defaults() {
    let state = test_app_state();
    assert_eq!(state.active_tab, 0);
    assert_eq!(state.peer_selection, 0);
    assert!(state.env.mouse_capture);
    assert!(state.auto_scroll.chat);
    assert!(state.auto_scroll.log);
    assert_eq!(state.topic_str, "test-net");
    assert_eq!(state.own_nickname, "TestUser");
    assert!(!state.is_editing_nickname());
    assert_eq!(state.popup, None);
}

// ── Contact expand / selection helpers ─────────────────────────────────

fn multi_device_contact(
    contact_id: &str,
    name: &str,
    devices: &[&str],
) -> p2p_app::contacts::ContactRow {
    let device_rows = devices
        .iter()
        .map(|d| p2p_app::contacts::ContactDevice {
            peer_id: format!("{contact_id}-{d}"),
            display_name: d.to_string(),
            first_seen: "2024-01-01 12:00:00".to_string(),
            last_seen: "2024-01-01 12:00:00".to_string(),
            received_nickname: None,
            local_nickname: None,
        })
        .collect();
    p2p_app::contacts::ContactRow {
        contact_id: contact_id.to_string(),
        is_user: false,
        display_name: name.to_string(),
        device_count: devices.len(),
        device_rows,
        first_seen: "2024-01-01 12:00:00".to_string(),
        last_seen: "2024-01-01 12:00:00".to_string(),
        dm_count: 3,
        broadcast_sent_to_peer: 2,
    }
}

#[test]
fn toggle_expand_contact_adds_and_removes_device_rows() {
    let mut state = test_app_state();
    state.contacts = vec![multi_device_contact("bob", "Bob", &["Laptop", "Phone"])];
    state.peer_selection = 0;

    // Single expansion: the contact row gains its two device sub-rows.
    state.toggle_expand_selected_contact();
    assert!(state.expanded_contact_ids.contains("bob"));
    assert_eq!(
        p2p_app::tui_helpers::contact_table_plan_len(&state.contacts, &state.expanded_contact_ids,),
        3
    );

    // Toggling again collapses back to the contact row.
    state.toggle_expand_selected_contact();
    assert!(state.expanded_contact_ids.is_empty());
    assert_eq!(
        p2p_app::tui_helpers::contact_table_plan_len(&state.contacts, &state.expanded_contact_ids,),
        1
    );
}

#[test]
fn toggle_expand_on_device_row_or_single_device_is_noop() {
    let mut state = test_app_state();
    state.contacts = vec![
        multi_device_contact("alice", "Alice", &["Phone"]),
        multi_device_contact("bob", "Bob", &["Laptop", "Phone"]),
    ];
    state.expanded_contact_ids.insert("bob".to_string());

    // Selecting bob's first device row must not toggle the contact.
    state.peer_selection = 2;
    state.toggle_expand_selected_contact();
    assert!(state.expanded_contact_ids.contains("bob"));

    // A single-device contact can never expand.
    state.peer_selection = 0;
    state.toggle_expand_selected_contact();
    assert!(!state.expanded_contact_ids.contains("alice"));
    assert!(state.expanded_contact_ids.contains("bob"));
}

#[test]
fn selected_contact_helpers_resolve_device_rows() {
    let mut state = test_app_state();
    state.contacts = vec![
        multi_device_contact("alice", "Alice", &["Phone"]),
        multi_device_contact("bob", "Bob", &["Laptop", "Phone"]),
    ];
    state.expanded_contact_ids.insert("bob".to_string());

    // Contact row -> best device.
    state.peer_selection = 1;
    assert_eq!(
        state.selected_contact_peer_id(),
        Some("bob-Laptop".to_string())
    );
    assert_eq!(state.selected_contact_id(), Some("bob".to_string()));

    // Device row -> that exact device (plan index 2 = Laptop).
    state.peer_selection = 2;
    assert_eq!(
        state.selected_contact_peer_id(),
        Some("bob-Laptop".to_string())
    );
    assert_eq!(state.selected_contact_id(), Some("bob".to_string()));

    // Second device row (plan index 3 = Phone).
    state.peer_selection = 3;
    assert_eq!(
        state.selected_contact_peer_id(),
        Some("bob-Phone".to_string())
    );
    assert_eq!(state.selected_contact_id(), Some("bob".to_string()));

    // Out-of-range selection resolves to nothing.
    state.peer_selection = 99;
    assert_eq!(state.selected_contact_peer_id(), None);
}

// ── load_and_format_messages ───────────────────────────────────────────

#[test]
fn test_load_and_format_messages_db_failure_returns_empty() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap().to_string();
    p2p_app::db::set_db_url(&db_str);
    p2p_app::db::init_database().unwrap();

    let (msgs, ids) = load_and_format_messages("test", 10, &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs.is_empty());
    assert!(ids.is_empty());

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
    drop(dir);
}
