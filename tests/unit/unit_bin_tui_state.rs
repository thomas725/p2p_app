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
        reply_to_json: None,
        sent_at,
        sender_nickname: sender_nickname.map(String::from),
        attachment_json: None,
        encrypted: 0,
        deleted_at: None,
        edited_at: None,
    }
}

#[test]
fn test_format_messages_from_db_empty() {
    let (msgs, ids, db_ids) = format_messages_from_db(&[], &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs.is_empty());
    assert!(ids.is_empty());
    assert!(db_ids.is_empty());
}

#[test]
fn test_format_messages_from_db_restores_edit_marker() {
    let plain = msg(
        "plain body",
        None,
        None,
        Some("m-plain"),
        Some(1.0),
        "2024-01-01 12:00:00",
    );
    let mut edited = msg(
        "edited body",
        None,
        None,
        Some("m-edited"),
        Some(2.0),
        "2024-01-01 12:01:00",
    );
    edited.edited_at = Some(1_700_000_000.0);

    let (msgs, _, db_ids) =
        format_messages_from_db(&[plain, edited], &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 2);
    assert!(msgs[1].text.ends_with("plain body"));
    assert!(!msgs[1].text.contains("(edited)"));
    assert!(msgs[0].text.ends_with("edited body (edited)"));
    assert!(
        db_ids[0].is_none(),
        "test rows are incoming unless explicitly marked sent"
    );
}

#[test]
fn test_reply_draft_capture_clear_and_consume() {
    let mut s = test_app_state();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] the question".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("wire-1".to_string()));
    let tab = p2p_app::tui_tabs::TabContent::Chat;

    let reply = s.begin_reply(&tab).expect("newest row is replyable");
    assert_eq!(reply.msg_id, "wire-1");
    assert_eq!(reply.sender, "Alice");
    assert_eq!(reply.preview, "the question");
    assert_eq!(s.reply_draft(&tab), Some(&reply));

    assert!(s.clear_reply(&tab));
    assert!(!s.clear_reply(&tab), "clearing twice is a no-op");
    assert!(s.reply_draft(&tab).is_none());

    let again = s.begin_reply(&tab).expect("row remains replyable");
    assert_eq!(s.take_reply(&tab), Some(again));
    assert!(s.reply_draft(&tab).is_none(), "sending consumes the draft");
}

#[test]
fn test_reply_drafts_are_scoped_to_a_conversation() {
    let mut s = test_app_state();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] broadcast question".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("bc-wire".to_string()));
    let chat = p2p_app::tui_tabs::TabContent::Chat;
    assert!(s.begin_reply(&chat).is_some());

    s.dynamic_tabs.add_dm_tab("peer-dm".to_string());
    s.dm_messages.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from(["12:01 [Bob] dm question".to_string()]),
    );
    s.dm_message_ids.insert(
        "peer-dm".to_string(),
        std::collections::VecDeque::from([Some("dm-wire".to_string())]),
    );
    let dm = p2p_app::tui_tabs::TabContent::Direct("peer-dm".to_string());

    let dm_reply = s.begin_reply(&dm).expect("dm row is replyable");
    assert_eq!(dm_reply.msg_id, "dm-wire");
    assert_eq!(
        s.reply_draft(&chat).map(|r| r.msg_id.as_str()),
        Some("bc-wire")
    );
    assert_eq!(s.reply_draft(&dm), Some(&dm_reply));

    assert!(s.clear_reply(&dm));
    assert_eq!(
        s.reply_draft(&chat).map(|r| r.msg_id.as_str()),
        Some("bc-wire")
    );
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
    let (msgs, ids, db_ids) =
        format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
    assert_eq!(msgs.len(), 1);
    assert!(msgs[0].text.contains("[Me]"));
    assert!(msgs[0].text.contains("hello"));
    assert_eq!(ids[0], Some("m1".to_string()));
    assert_eq!(db_ids[0], None); // `sent = 0` rows are never deletable
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
fn test_format_messages_skipped_when_sent_at_none() {
    let messages = [msg(
        "no sent_at",
        None,
        None,
        Some("m1"),
        None,
        "2024-01-01 12:00:00",
    )];
    let (msgs, ids, _) = format_messages_from_db(&messages, &HashMap::new(), &HashMap::new(), "Me");
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

#[test]
fn test_format_messages_records_db_ids_only_for_sent_rows() {
    let mut outgoing = msg(
        "mine",
        None,
        None,
        Some("m1"),
        Some(1.0),
        "2024-01-01 12:00:00",
    );
    outgoing.id = 7;
    outgoing.sent = 1;
    let mut incoming = msg(
        "theirs",
        Some("p1"),
        None,
        None,
        None,
        "2024-01-01 12:00:01",
    );
    incoming.id = 8;

    // DB rows are newest-first; the formatter reverses them chronologically.
    let (msgs, _, db_ids) = format_messages_from_db(
        &[incoming, outgoing],
        &HashMap::new(),
        &HashMap::new(),
        "Me",
    );
    assert_eq!(msgs.len(), 2);
    assert_eq!(db_ids, VecDeque::from([Some(7), None]));
}

// ── Local-only deletion queue helpers (US-005) ──────────────────────────

#[test]
fn test_delete_prompt_and_removal_target_newest_sent_broadcast() {
    use p2p_app::tui_tabs::TabContent;

    let mut state = test_app_state();
    for (text, msg_id, db_id) in [
        ("old", Some("m-old"), Some(7)),
        ("incoming", None, None),
        ("newest", Some("m-new"), Some(9)),
    ] {
        state.messages.push_back(p2p_app::DisplayMessage {
            text: text.to_string(),
            sender_peer_id: None,
        });
        state.message_ids.push_back(msg_id.map(str::to_string));
        state.message_db_ids.push_back(db_id);
    }

    assert_eq!(
        state.last_sent_message(&TabContent::Chat),
        Some((9, "newest".to_string()))
    );
    assert_eq!(
        state.last_sent_message(&TabContent::Direct("p1".into())),
        None
    );

    state.begin_delete_message(9, false, "newest".to_string(), TabContent::Chat);
    let prompt = state.delete_message_prompt.clone().unwrap();
    assert_eq!(prompt.message_id, 9);
    assert!(!prompt.is_group);
    assert!(state.remove_deleted_message(&prompt));
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages.back().unwrap().text, "incoming");
    assert_eq!(state.message_db_ids, VecDeque::from([Some(7), None]));
    assert!(!state.remove_deleted_message(&prompt));
    assert!(state.cancel_delete_message());
    assert!(state.delete_message_prompt.is_none());
    assert!(!state.cancel_delete_message());
}

#[test]
fn test_delete_prompt_removal_is_scoped_to_dm_and_group() {
    use p2p_app::tui_tabs::TabContent;

    let mut state = test_app_state();
    state
        .dm_messages
        .insert("p1".into(), VecDeque::from(["dm".to_string()]));
    state
        .dm_message_ids
        .insert("p1".into(), VecDeque::from([Some("dm-1".to_string())]));
    state
        .dm_message_db_ids
        .insert("p1".into(), VecDeque::from([Some(11)]));
    state.group_messages.insert(
        "g1".into(),
        VecDeque::from(["group".to_string(), "later".to_string()]),
    );
    state
        .group_message_ids
        .insert("g1".into(), VecDeque::from([None, None]));
    state
        .group_message_db_ids
        .insert("g1".into(), VecDeque::from([Some(13), None]));
    state
        .group_message_peer_ids
        .insert("g1".into(), VecDeque::from([None, Some("p2".into())]));

    assert_eq!(
        state.last_sent_message(&TabContent::Direct("p1".into())),
        Some((11, "dm".to_string()))
    );
    assert_eq!(
        state.last_sent_message(&TabContent::GroupChat("g1".into())),
        Some((13, "group".to_string()))
    );
    assert_eq!(state.last_sent_message(&TabContent::Chat), None);

    let group_prompt = DeleteMessagePrompt {
        message_id: 13,
        is_group: true,
        preview: "group".into(),
        tab_content: TabContent::GroupChat("g1".into()),
    };
    assert!(state.remove_deleted_message(&group_prompt));
    assert_eq!(
        state.group_messages["g1"],
        VecDeque::from(["later".to_string()])
    );
    assert_eq!(state.group_message_db_ids["g1"], VecDeque::from([None]));
    assert_eq!(
        state.group_message_peer_ids["g1"],
        VecDeque::from([Some("p2".into())])
    );
    assert_eq!(state.dm_message_db_ids["p1"], VecDeque::from([Some(11)]));

    let dm_prompt = DeleteMessagePrompt {
        message_id: 11,
        is_group: false,
        preview: "dm".into(),
        tab_content: TabContent::Direct("p1".into()),
    };
    assert!(state.remove_deleted_message(&dm_prompt));
    assert!(state.dm_messages["p1"].is_empty());
    assert!(state.dm_message_db_ids["p1"].is_empty());
}

#[test]
fn test_edit_prompt_prefill_cancel_and_in_place_replacement() {
    use p2p_app::tui_tabs::TabContent;

    let mut state = test_app_state();
    state.messages.push_back(p2p_app::DisplayMessage {
        text: "10:00 [Alice] before".to_string(),
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some("wire-1".to_string()));
    state.message_db_ids.push_back(Some(42));

    assert_eq!(
        state.last_sent_message_raw(&TabContent::Chat),
        Some((42, "before".to_string()))
    );
    assert_eq!(
        state.last_sent_message(&TabContent::Chat),
        Some((42, "before".to_string()))
    );

    state.begin_edit_message(42, false, "before".to_string(), TabContent::Chat);
    assert!(state.is_editing_message());
    assert_eq!(state.input_mode, p2p_app::InputMode::EditingMessage);
    assert_eq!(state.chat_input.lines(), ["before"]);

    let prompt = crate::tui::state::EditMessagePrompt {
        message_id: 42,
        is_group: false,
        tab_content: TabContent::Chat,
    };
    assert!(state.replace_edited_message(&prompt, "after"));
    assert_eq!(state.messages[0].text, "10:00 [Alice] after (edited)");
    assert!(
        state.replace_edited_message(&prompt, "again"),
        "repeat edits refresh the same row"
    );
    assert_eq!(
        state.messages[0].text, "10:00 [Alice] again (edited)",
        "a repeat edit must not stack the marker"
    );
    let missing = crate::tui::state::EditMessagePrompt {
        message_id: 99,
        ..prompt
    };
    assert!(!state.replace_edited_message(&missing, "nowhere"));

    state.begin_edit_message(42, false, "again".to_string(), TabContent::Chat);
    assert!(state.cancel_edit_message());
    assert!(!state.is_editing_message());
    assert_eq!(state.input_mode, p2p_app::InputMode::Chat);
    assert!(!state.cancel_edit_message());
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
    assert_eq!(state.chat_input.lines().join(""), "");
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

// ── Contacts search filter ─────────────────────────────────────────────

#[test]
fn test_contact_matches_name_id_and_device() {
    let contact = multi_device_contact("bob", "Bob", &["Laptop", "Phone"]);
    // Contact display name, contact id, device display name, device peer id.
    assert!(contact_matches(&contact, "bob"));
    assert!(contact_matches(&contact, "laptop"));
    assert!(contact_matches(&contact, "bob-phone"));
    assert!(!contact_matches(&contact, "carol"));
}

#[test]
fn test_apply_contact_search_filters_and_restores() {
    let mut state = test_app_state();
    state.contacts_all = vec![
        multi_device_contact("alice", "Alice", &["Phone"]),
        multi_device_contact("bob", "Bob", &["Laptop"]),
        multi_device_contact("carol", "Carol", &["Tablet"]),
    ];
    state.contacts.clone_from(&state.contacts_all);

    state.set_contact_search("bo");
    assert_eq!(state.contact_search, "bo");
    assert_eq!(state.contacts.len(), 1);
    assert_eq!(state.contacts[0].contact_id, "bob");

    // A device-name match keeps the owning contact row.
    state.set_contact_search("tablet");
    assert_eq!(state.contacts.len(), 1);
    assert_eq!(state.contacts[0].contact_id, "carol");

    // Clearing restores every row.
    state.set_contact_search("");
    assert_eq!(state.contacts.len(), 3);
}

#[test]
fn test_apply_contact_search_keeps_selection_when_it_survives() {
    let mut state = test_app_state();
    state.contacts_all = vec![
        multi_device_contact("alice", "Alice", &["Phone"]),
        multi_device_contact("bob", "Bob", &["Laptop"]),
    ];
    state.contacts.clone_from(&state.contacts_all);
    state.peer_selection = 1; // bob

    state.set_contact_search("o"); // matches both bob and …alice? -> only "bob"/"Phone"
    assert_eq!(state.selected_contact_id().as_deref(), Some("bob"));

    // Filtering to nothing clamps the selection to an empty table.
    state.set_contact_search("zzz");
    assert!(state.contacts.is_empty());
    assert_eq!(state.peer_selection, 0);
}

#[test]
fn test_contact_search_mode_begin_and_end() {
    let mut state = test_app_state();
    assert!(!state.is_searching_contacts());

    state.set_contact_search("bob");
    state.begin_contact_search();
    assert!(state.is_searching_contacts());
    // The box is pre-filled with the active query so it can be edited.
    assert_eq!(state.chat_input.lines().join("\n"), "bob");

    // Enter keeps the query.
    state.end_contact_search(true);
    assert!(!state.is_searching_contacts());
    assert_eq!(state.contact_search, "bob");

    // Esc clears it and restores the full list.
    state.begin_contact_search();
    state.end_contact_search(false);
    assert!(!state.is_searching_contacts());
    assert_eq!(state.contact_search, "");
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

    let (msgs, ids, db_ids) =
        load_and_format_messages("test", 10, &HashMap::new(), &HashMap::new(), "Me");
    assert!(msgs.is_empty());
    assert!(ids.is_empty());
    assert!(db_ids.is_empty());

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
    drop(dir);
}

// ── persisted network peers (US-050 follow-up 1) ─────────────────────────

fn network_peer(id: i32, kind: &str, addr: &str) -> p2p_app::network_config::NetworkPeer {
    p2p_app::network_config::NetworkPeer {
        id,
        kind: kind.to_string(),
        multiaddr: addr.to_string(),
        created_at: "2026-11-08 12:00:00".to_string(),
    }
}

#[test]
fn test_network_peer_add_prompt_transitions() {
    let mut state = test_app_state();
    assert!(!state.is_adding_network_peer());
    state.chat_input.insert_str("leftover");

    state.begin_add_network_peer(p2p_app::network_config::NetworkPeerKind::Relay);
    assert!(state.is_adding_network_peer());
    assert_eq!(
        state.network_peer_kind,
        Some(p2p_app::network_config::NetworkPeerKind::Relay)
    );
    assert_eq!(
        state.chat_input.lines().join(""),
        "",
        "the prompt must start empty"
    );

    state.cancel_add_network_peer();
    assert!(!state.is_adding_network_peer());
    assert_eq!(state.network_peer_kind, None);

    // Cancelling when no prompt is open is a no-op.
    state.cancel_add_network_peer();
    assert!(!state.is_adding_network_peer());
}

#[test]
fn test_move_network_peer_selection_clamps() {
    use crossterm::event::KeyCode;

    let mut state = test_app_state();
    state.move_network_peer_selection(KeyCode::Down);
    assert_eq!(state.network_peer_selection, 0, "empty list stays at 0");

    state.network_peers = vec![
        network_peer(1, "bootstrap", "/ip4/1.1.1.1/tcp/4001"),
        network_peer(2, "relay", "/ip4/2.2.2.2/tcp/4001/p2p/relay"),
        network_peer(3, "bootstrap", "/ip4/3.3.3.3/tcp/4001"),
    ];

    state.move_network_peer_selection(KeyCode::Down);
    assert_eq!(state.network_peer_selection, 1);
    state.move_network_peer_selection(KeyCode::End);
    assert_eq!(state.network_peer_selection, 2);
    state.move_network_peer_selection(KeyCode::Down);
    assert_eq!(state.network_peer_selection, 2, "clamped at the last row");
    state.move_network_peer_selection(KeyCode::Up);
    assert_eq!(state.network_peer_selection, 1);
    state.move_network_peer_selection(KeyCode::Home);
    assert_eq!(state.network_peer_selection, 0);
    state.move_network_peer_selection(KeyCode::Up);
    assert_eq!(state.network_peer_selection, 0, "clamped at the first row");
    state.move_network_peer_selection(KeyCode::PageDown);
    assert_eq!(state.network_peer_selection, 2);
    state.move_network_peer_selection(KeyCode::PageUp);
    assert_eq!(state.network_peer_selection, 0);
}

// ── transient DM typing indicators (US-009) ─────────────────────────────

#[test]
fn test_receive_and_expire_typing() {
    use std::time::{Duration, Instant};

    let mut state = test_app_state();
    assert!(state.typing_peers.is_empty());

    assert!(state.receive_typing("peer-a", true));
    assert!(state.typing_peers.contains_key("peer-a"));
    assert!(!state.expire_typing(), "a fresh signal stays visible");

    // Force expiry without sleeping for the whole TTL.
    state.typing_peers.insert(
        "peer-a".to_string(),
        Instant::now()
            .checked_sub(Duration::from_secs(1))
            .expect("current time is after one second"),
    );
    assert!(state.expire_typing());
    assert!(state.typing_peers.is_empty());
    assert!(!state.expire_typing());

    assert!(!state.receive_typing("peer-a", false), "already absent");
    assert!(state.receive_typing("peer-a", true));
    assert!(state.receive_typing("peer-a", false));
    assert!(state.typing_peers.is_empty());
}

#[test]
fn test_outgoing_typing_is_throttled_but_stops_are_immediate() {
    let mut state = test_app_state();

    assert!(state.note_typing_sent("peer-a", true));
    assert!(
        !state.note_typing_sent("peer-a", true),
        "key repeats are throttled"
    );
    assert!(state.note_typing_sent("peer-a", false));
    assert!(
        state.note_typing_sent("peer-a", true),
        "a fresh edit after stopping starts a new signal"
    );
    assert!(
        state.note_typing_sent("peer-b", true),
        "throttling is per conversation"
    );
}

#[test]
fn test_clear_typing_forgets_both_directions() {
    let mut state = test_app_state();
    state.receive_typing("peer-a", true);
    state.note_typing_sent("peer-a", true);

    assert!(state.clear_typing("peer-a"));
    assert!(state.typing_peers.is_empty());
    assert!(state.typing_sent_at.is_empty());
    assert!(!state.clear_typing("peer-a"));
}
