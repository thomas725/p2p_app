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
fn test_reply_capture_keeps_multiline_body_and_collapses_preview() {
    let mut s = test_app_state();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] line one\nline two\nline three".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("wire-multi".to_string()));
    let tab = p2p_app::tui_tabs::TabContent::Chat;

    let reply = s.begin_reply(&tab).expect("newest row is replyable");
    assert_eq!(reply.msg_id, "wire-multi");
    assert_eq!(reply.sender, "Alice");
    assert_eq!(reply.preview, "line one line two line three");
    assert_eq!(s.reply_draft(&tab), Some(&reply));
}

#[test]
fn test_target_message_line_returns_last_row_per_chat_tab() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    assert_eq!(s.target_message_line(&TabContent::Chat), None);

    s.messages.push_back(p2p_app::DisplayMessage {
        text: "first".to_string(),
        sender_peer_id: None,
    });
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] second\ncontinuation".to_string(),
        sender_peer_id: None,
    });
    assert_eq!(
        s.target_message_line(&TabContent::Chat).as_deref(),
        Some("12:00 [Alice] second\ncontinuation"),
        "the full rendered line - including continuation rows - is copied"
    );

    s.dm_messages.insert(
        "peer-b".to_string(),
        std::collections::VecDeque::from(["dm old".to_string(), "dm new".to_string()]),
    );
    assert_eq!(
        s.target_message_line(&TabContent::Direct("peer-b".to_string()))
            .as_deref(),
        Some("dm new")
    );

    s.group_messages.insert(
        "group-c".to_string(),
        std::collections::VecDeque::from(["group old".to_string(), "group new".to_string()]),
    );
    assert_eq!(
        s.target_message_line(&TabContent::GroupChat("group-c".to_string()))
            .as_deref(),
        Some("group new")
    );

    // Non-chat tabs own no message queue, so they must never yield a copy.
    assert_eq!(s.target_message_line(&TabContent::Contacts), None);
    assert_eq!(s.target_message_line(&TabContent::Log), None);
    assert_eq!(
        s.target_message_line(&TabContent::Direct("missing".to_string())),
        None
    );
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

#[test]
fn test_edit_flow_keeps_multiline_body_through_prefill_and_replacement() {
    use p2p_app::tui_tabs::TabContent;

    let mut state = test_app_state();
    let original = "first line\nsecond line\nthird line";
    state.messages.push_back(p2p_app::DisplayMessage {
        text: format!("10:00 [Alice] {original}"),
        sender_peer_id: None,
    });
    state.message_ids.push_back(Some("wire-multi".to_string()));
    // A distinctive id that has no DB row, so `replace_edited_message` uses the
    // editor content directly (the edited marker is added in memory).
    let db_id = 910_043;
    state.message_db_ids.push_back(Some(db_id));

    assert_eq!(
        state.last_sent_message_raw(&TabContent::Chat),
        Some((db_id, original.to_string())),
        "the full multiline body - not just its last line - is pre-filled"
    );

    state.begin_edit_message(db_id, false, original.to_string(), TabContent::Chat);
    assert!(state.is_editing_message());
    assert_eq!(
        state.chat_input.lines().first().map(String::as_str),
        Some(original),
        "the editor keeps every embedded newline in its single pre-filled row"
    );

    let prompt = crate::tui::state::EditMessagePrompt {
        message_id: db_id,
        is_group: false,
        tab_content: TabContent::Chat,
    };
    let edited = "replacement one\nreplacement two";
    assert!(state.replace_edited_message(&prompt, edited));
    assert_eq!(
        state.messages[0].text,
        format!("10:00 [Alice] {edited} (edited)"),
        "editing replaces the entire multiline block, not just its last line"
    );
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

/// Re-sorting the Contacts tab must keep the selected *contact*, not the row
/// index. `resort_peers` rebuilds and re-orders the table, so the id has to be
/// captured before that mutation — otherwise a sort flip silently selects a
/// different peer (the regression this guards).
#[test]
fn test_resort_peers_keeps_selected_contact_across_sort_flip() {
    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    p2p_app::db::set_db_url(db_path.to_str().unwrap());
    p2p_app::db::init_database().unwrap();

    let mut state = test_app_state();
    // Distinct last-seen values make the ascending/descending order unambiguous
    // (column 3 sorts on last_seen, parsed from the in-memory peer records).
    for (peer_id, seen) in [
        ("alice", "2024-01-03 12:00:00"),
        ("bob", "2024-01-02 12:00:00"),
        ("carol", "2024-01-01 12:00:00"),
    ] {
        state.peers.push_back(p2p_app::PeerRecord {
            peer_id: peer_id.to_string(),
            first_seen: "2024-01-01 12:00:00".to_string(),
            last_seen: seen.to_string(),
        });
    }
    state.peer_sort_column = 3; // last seen
    state.peer_sort_ascending = true; // oldest first: carol, bob, alice
    state.resort_peers();
    assert_eq!(state.contacts.len(), 3);
    assert_eq!(state.contacts[0].contact_id, "carol");

    state.peer_selection = 0; // carol
    assert_eq!(state.selected_contact_id().as_deref(), Some("carol"));

    // Flip to newest-first: alice, bob, carol. Carol moved to the last row;
    // the selection must follow her instead of staying on the first row (alice).
    state.peer_sort_ascending = false;
    state.resort_peers();
    assert_eq!(state.contacts[0].contact_id, "alice");
    assert_eq!(state.selected_contact_id().as_deref(), Some("carol"));
    assert_eq!(state.peer_selection, 2);

    p2p_app::db::reset_db_url();
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

#[test]
fn test_active_conversation_key_tracks_the_visible_chat() {
    let mut state = test_app_state();

    // Default tab is the broadcast chat.
    assert_eq!(
        state.active_conversation_key().as_deref(),
        Some("broadcast")
    );

    // Non-chat tabs never suppress a notification.
    state.active_tab = 1; // Contacts
    assert_eq!(state.active_conversation_key(), None);
    state.active_tab = 2; // Groups
    assert_eq!(state.active_conversation_key(), None);

    // A group-chat tab suppresses only that group's key.
    let group_tab = state
        .dynamic_tabs
        .add_group_tab("grp-1".into(), "Team".into());
    state.active_tab = group_tab;
    assert_eq!(
        state.active_conversation_key().as_deref(),
        Some("group:grp-1")
    );

    // A DM tab suppresses only that peer's key.
    let dm_tab = state.dynamic_tabs.add_dm_tab("peer-42".into());
    state.active_tab = dm_tab;
    assert_eq!(
        state.active_conversation_key().as_deref(),
        Some("dm:peer-42")
    );

    // Inspecting the same peer's info is not the same as reading the DM.
    let peer_info_tab = state.dynamic_tabs.add_peer_info_tab("peer-42".into());
    state.active_tab = peer_info_tab;
    assert_eq!(state.active_conversation_key(), None);

    // The Shares suffix tab is not a conversation either.
    state.active_tab = state.dynamic_tabs.total_tab_count().saturating_sub(1);
    assert_eq!(state.active_conversation_key(), None);

    // Notification focus ('broadcast') is not the draft/cursor key ('chat').
    state.active_tab = 0;
    assert_eq!(
        state.active_conversation_key().as_deref(),
        Some("broadcast")
    );
    assert_eq!(
        p2p_app::notifications::conversation_key_for_tab(
            &state.dynamic_tabs.tab_index_to_content(state.active_tab)
        )
        .as_deref(),
        state.active_conversation_key().as_deref()
    );
    assert_eq!(
        AppState::conversation_key(&p2p_app::tui_tabs::TabContent::Chat).as_deref(),
        Some("chat")
    );
}

// ── target_attachment_path (TUI received-attachment open, Flutter parity) ───

#[test]
fn test_target_attachment_path_resolves_broadcast_and_group() {
    use p2p_app::tui_tabs::TabContent;

    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let db_str = db_path.to_str().unwrap().to_string();
    p2p_app::db::set_db_url(&db_str);
    p2p_app::db::init_database().unwrap();

    // No attachment in an empty conversation.
    let mut state = test_app_state();
    assert_eq!(state.target_attachment_path(&TabContent::Chat), None);

    // Store a blob on disk and attach it to a broadcast message.
    let src = dir.path().join("chip.txt");
    std::fs::write(&src, b"attachment bytes").unwrap();
    let att = p2p_app::files::store_outgoing_file(src.to_str().unwrap()).unwrap();
    let expected = p2p_app::files::file_path(&att.file_id).unwrap();

    let meta = p2p_app::messages::MessageMeta {
        msg_id: Some("broadcast-att-1".to_string()),
        attachment: Some(att.clone()),
        ..Default::default()
    };
    p2p_app::messages::save_message_with_meta("see file", None, "broadcast", false, None, meta)
        .unwrap();

    state
        .message_ids
        .push_back(Some("broadcast-att-1".to_string()));
    assert_eq!(
        state.target_attachment_path(&TabContent::Chat).as_deref(),
        Some(expected.as_str())
    );

    // A group message resolves through the group messages table.
    let gmeta = p2p_app::groups::GroupMessageMeta {
        msg_id: Some("group-att-1".to_string()),
        attachment: Some(att),
        ..Default::default()
    };
    p2p_app::groups::save_incoming_group_message("grp-att", "peer-1", "group file", gmeta).unwrap();
    let mut gstate = test_app_state();
    gstate.group_message_ids.insert(
        "grp-att".to_string(),
        std::collections::VecDeque::from(vec![Some("group-att-1".to_string())]),
    );
    assert_eq!(
        gstate
            .target_attachment_path(&TabContent::GroupChat("grp-att".to_string()))
            .as_deref(),
        Some(expected.as_str())
    );

    // A wire id that has no persisted attachment yields nothing (the chip may
    // have been rendered from a live event whose row failed to persist).
    let mut missing = test_app_state();
    missing
        .message_ids
        .push_back(Some("never-persisted".to_string()));
    assert_eq!(missing.target_attachment_path(&TabContent::Chat), None);

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

// ── stable message selection (keyboard cursor parity) ─────────────────────

#[test]
fn test_message_selection_follows_wire_id_when_history_shifts() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    s.messages = std::collections::VecDeque::from(vec![
        p2p_app::DisplayMessage {
            text: "12:00 [Alice] old line".to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
        p2p_app::DisplayMessage {
            text: "12:01 [Alice] target line".to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
        p2p_app::DisplayMessage {
            text: "12:02 [Alice] newest line".to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
    ]);
    s.message_ids = std::collections::VecDeque::from(vec![
        Some("wire-old".to_string()),
        Some("wire-target".to_string()),
        Some("wire-new".to_string()),
    ]);
    s.message_db_ids = std::collections::VecDeque::from(vec![None, None, None]);

    assert!(s.select_message(&tab, 1));
    assert_eq!(
        s.message_selection.get("chat"),
        Some(&MessageSelection::WireId("wire-target".to_string()))
    );
    assert_eq!(s.message_selection_index(&tab), Some(1));
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-target"));
    assert_eq!(
        s.target_message_line(&tab).as_deref(),
        Some("12:01 [Alice] target line")
    );

    // A history trim removes the row before the pinned one. The wire id still
    // resolves, now at index 0, so every chat verb keeps targeting it.
    let _ = s.messages.pop_front();
    let _ = s.message_ids.pop_front();
    let _ = s.message_db_ids.pop_front();
    assert_eq!(s.message_selection_index(&tab), Some(0));
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-target"));

    assert!(s.clear_message_selection(&tab));
    assert_eq!(s.message_selection_index(&tab), None);
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-new"));
}

#[test]
fn test_message_selection_uses_db_id_when_wire_id_is_absent() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    s.messages = std::collections::VecDeque::from(vec![
        p2p_app::DisplayMessage {
            text: "12:00 [Me] first body".to_string(),
            sender_peer_id: None,
        },
        p2p_app::DisplayMessage {
            text: "12:01 [Me] second body".to_string(),
            sender_peer_id: None,
        },
    ]);
    s.message_ids = std::collections::VecDeque::from(vec![None, None]);
    s.message_db_ids = std::collections::VecDeque::from(vec![Some(10), Some(11)]);

    assert!(s.select_message(&tab, 0));
    assert_eq!(
        s.message_selection.get("chat"),
        Some(&MessageSelection::DbId(10))
    );
    assert_eq!(s.target_message_id(&tab), None);
    assert_eq!(
        s.target_sent_message_raw(&tab),
        Some((10, "first body".to_string()))
    );
    assert_eq!(
        s.target_sent_message(&tab),
        Some((10, "first body".to_string()))
    );

    s.messages.push_front(p2p_app::DisplayMessage {
        text: "11:59 [Alice] unrelated".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_front(None);
    s.message_db_ids.push_front(None);
    assert_eq!(s.message_selection_index(&tab), Some(1));
    assert_eq!(
        s.target_sent_message_raw(&tab),
        Some((10, "first body".to_string()))
    );
}

#[test]
fn test_message_selection_rendered_anchor_uses_line_occurrence() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    let duplicate = "12:00 [Alice] same line";
    s.messages = std::collections::VecDeque::from(vec![
        p2p_app::DisplayMessage {
            text: duplicate.to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
        p2p_app::DisplayMessage {
            text: duplicate.to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
    ]);
    s.message_ids = std::collections::VecDeque::from(vec![Some(String::new()), None]);
    s.message_db_ids = std::collections::VecDeque::from(vec![None, None]);

    assert!(s.select_message(&tab, 1));
    assert_eq!(
        s.message_selection.get("chat"),
        Some(&MessageSelection::Rendered {
            text: duplicate.to_string(),
            occurrence: 1,
            loaded_len: 2,
        })
    );

    // A front trim changes the loaded length, so the id-less anchor is stale:
    // verbs fall back to the newest row instead of retargeting the remaining
    // equal line at a different index.
    s.messages.push_front(p2p_app::DisplayMessage {
        text: "11:59 [Alice] distinct line".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_front(None);
    s.message_db_ids.push_front(None);
    assert_eq!(s.message_selection_index(&tab), None);
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:03 [Alice] newer distinct line".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(None);
    s.message_db_ids.push_back(None);
    assert_eq!(
        s.target_message_line(&tab).as_deref(),
        Some("12:03 [Alice] newer distinct line")
    );
}

#[test]
fn test_message_selection_is_scoped_per_conversation() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let chat = TabContent::Chat;
    let dm = TabContent::Direct("peer-1".to_string());
    let group = TabContent::GroupChat("group-1".to_string());

    s.messages.push_back(p2p_app::DisplayMessage {
        text: "chat oldest".to_string(),
        sender_peer_id: None,
    });
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "chat newest".to_string(),
        sender_peer_id: None,
    });
    s.message_ids.push_back(Some("chat-old".to_string()));
    s.message_ids.push_back(Some("chat-new".to_string()));
    s.message_db_ids.push_back(None);
    s.message_db_ids.push_back(None);

    s.dm_messages.insert(
        "peer-1".to_string(),
        std::collections::VecDeque::from(vec!["dm oldest".to_string(), "dm newest".to_string()]),
    );
    s.dm_message_ids.insert(
        "peer-1".to_string(),
        std::collections::VecDeque::from(vec![
            Some("dm-old".to_string()),
            Some("dm-new".to_string()),
        ]),
    );
    s.dm_message_db_ids.insert(
        "peer-1".to_string(),
        std::collections::VecDeque::from(vec![None, None]),
    );

    s.group_messages.insert(
        "group-1".to_string(),
        std::collections::VecDeque::from(vec![
            "group oldest".to_string(),
            "group newest".to_string(),
        ]),
    );
    s.group_message_ids.insert(
        "group-1".to_string(),
        std::collections::VecDeque::from(vec![
            Some("group-old".to_string()),
            Some("group-new".to_string()),
        ]),
    );
    s.group_message_db_ids.insert(
        "group-1".to_string(),
        std::collections::VecDeque::from(vec![None, None]),
    );

    assert!(s.select_message(&chat, 0));
    assert!(s.select_message(&dm, 0));
    assert!(s.select_message(&group, 0));
    assert_eq!(s.target_message_id(&chat).as_deref(), Some("chat-old"));
    assert_eq!(s.target_message_id(&dm).as_deref(), Some("dm-old"));
    assert_eq!(s.target_message_id(&group).as_deref(), Some("group-old"));

    assert!(s.clear_message_selection(&chat));
    assert_eq!(s.message_selection_index(&chat), None);
    assert_eq!(s.target_message_id(&chat).as_deref(), Some("chat-new"));
    assert_eq!(s.message_selection_index(&dm), Some(0));
    assert_eq!(s.message_selection_index(&group), Some(0));
}

#[test]
fn test_stale_message_selection_falls_back_to_newest_row() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] deleted row".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:01 [Alice] surviving row".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("wire-deleted".to_string()));
    s.message_ids.push_back(Some("wire-survivor".to_string()));
    s.message_db_ids.push_back(None);
    s.message_db_ids.push_back(None);

    assert!(s.select_message(&tab, 0));
    let _ = s.messages.pop_front();
    let _ = s.message_ids.pop_front();
    let _ = s.message_db_ids.pop_front();

    // The stale anchor is not reused as index 0 of unrelated content: the
    // verb-returning helpers fall back to the newest row instead.
    assert_eq!(s.message_selection_index(&tab), None);
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-survivor"));
    assert_eq!(
        s.target_message_line(&tab).as_deref(),
        Some("12:01 [Alice] surviving row")
    );
}

#[test]
fn test_blank_wire_id_is_never_replyable_when_selected() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:00 [Alice] older valid".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.messages.push_back(p2p_app::DisplayMessage {
        text: "12:01 [Alice] newest blank".to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(Some("wire-valid".to_string()));
    s.message_ids.push_back(Some("   ".to_string()));
    s.message_db_ids.push_back(None);
    s.message_db_ids.push_back(None);

    // Without a cursor, scanning backwards skips the blank newest id.
    let reply = s.begin_reply(&tab).expect("older valid row is replyable");
    assert_eq!(reply.msg_id, "wire-valid");
    s.clear_reply(&tab);

    // With the blank row explicitly selected, no older row may be substituted.
    assert!(s.select_message(&tab, 1));
    assert_eq!(s.message_selection_index(&tab), Some(1));
    assert_eq!(s.target_message_id(&tab), None);
    assert!(!s.begin_reaction(&tab));
    assert_eq!(s.begin_reply(&tab), None);
    assert_eq!(
        s.target_message_line(&tab).as_deref(),
        Some("12:01 [Alice] newest blank")
    );
}

#[test]
fn test_sent_verbs_target_selected_row_without_falling_back() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    s.messages = std::collections::VecDeque::from(vec![
        p2p_app::DisplayMessage {
            text: "12:00 [Alice] incoming".to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        },
        p2p_app::DisplayMessage {
            text: "12:01 [Me] selected body".to_string(),
            sender_peer_id: None,
        },
        p2p_app::DisplayMessage {
            text: "12:02 [Me] newest body".to_string(),
            sender_peer_id: None,
        },
    ]);
    s.message_ids = std::collections::VecDeque::from(vec![
        Some("wire-in".to_string()),
        Some("wire-selected".to_string()),
        Some("wire-new".to_string()),
    ]);
    s.message_db_ids = std::collections::VecDeque::from(vec![None, Some(7), Some(9)]);

    assert!(s.select_message(&tab, 1));
    assert_eq!(
        s.target_sent_message_raw(&tab),
        Some((7, "selected body".to_string()))
    );
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-selected"));
    assert_eq!(
        s.begin_reply(&tab).map(|reply| reply.msg_id),
        Some("wire-selected".to_string())
    );

    assert!(s.select_message(&tab, 0));
    assert_eq!(
        s.target_sent_message(&tab),
        None,
        "incoming row is not deletable"
    );

    assert!(s.clear_message_selection(&tab));
    assert_eq!(
        s.target_sent_message_raw(&tab),
        Some((9, "newest body".to_string()))
    );
    assert_eq!(s.target_message_id(&tab).as_deref(), Some("wire-new"));
}

#[test]
fn test_rendered_anchor_is_invalidated_when_history_cap_keeps_deque_length() {
    use p2p_app::tui_tabs::TabContent;

    let mut s = test_app_state();
    let tab = TabContent::Chat;
    let duplicate = "12:00 [Alice] recycled line";
    // At the history cap the deque length never changes: a trim+append evicts
    // the oldest row and pushes a new one, so the rendered anchor's loaded_len
    // guard cannot detect the mutation on its own.
    for _ in 0..MAX_MESSAGE_HISTORY {
        s.messages.push_back(p2p_app::DisplayMessage {
            text: duplicate.to_string(),
            sender_peer_id: Some("peer-a".to_string()),
        });
        s.message_ids.push_back(None);
        s.message_db_ids.push_back(None);
    }
    assert!(s.select_message(&tab, 1));
    assert_eq!(
        s.message_selection.get("chat"),
        Some(&MessageSelection::Rendered {
            text: duplicate.to_string(),
            occurrence: 1,
            loaded_len: MAX_MESSAGE_HISTORY,
        })
    );

    // A normal append path must invalidate explicitly at the cap.
    let _ = s.messages.pop_front();
    let _ = s.message_ids.pop_front();
    let _ = s.message_db_ids.pop_front();
    s.messages.push_back(p2p_app::DisplayMessage {
        text: duplicate.to_string(),
        sender_peer_id: Some("peer-a".to_string()),
    });
    s.message_ids.push_back(None);
    s.message_db_ids.push_back(None);
    assert_eq!(
        s.message_selection_index(&tab),
        Some(1),
        "the stale anchor still resolves while the length is unchanged"
    );
    s.invalidate_rendered_selection(&tab);
    assert!(!s.message_selection.contains_key("chat"));
    assert_eq!(s.message_selection_index(&tab), None);
    assert_eq!(s.target_message_line(&tab).as_deref(), Some(duplicate));
}

#[test]
#[serial_test::serial]
fn test_group_leave_drops_the_group_selection_and_reply_draft() {
    use p2p_app::tui_tabs::TabContent;

    let _guard = p2p_app::db::shared_db_test_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = tempfile::TempDir::new().expect("tempdir");
    let db_path = dir.path().join("group-leave-selection.db");
    p2p_app::db::set_db_url(db_path.to_str().expect("db path"));
    p2p_app::db::init_database().expect("init db");

    let mut s = test_app_state();
    let group_id = "g-leave-selection";
    let group_tab = s
        .dynamic_tabs
        .add_group_tab(group_id.to_string(), "Group".to_string());
    s.active_tab = group_tab;
    s.group_messages.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![
            "12:00 [Alice] group body".to_string(),
            "12:01 [Me] group newest".to_string(),
        ]),
    );
    s.group_message_ids.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![
            Some("group-old".to_string()),
            Some("group-new".to_string()),
        ]),
    );
    s.group_message_db_ids.insert(
        group_id.to_string(),
        std::collections::VecDeque::from(vec![None, None]),
    );
    let tab = TabContent::GroupChat(group_id.to_string());
    assert!(s.select_message(&tab, 0));
    assert!(s.begin_reply(&tab).is_some());

    s.apply_group_leave(group_id);

    assert!(!s.message_selection.contains_key("group:g-leave-selection"));
    assert!(s.reply_draft(&tab).is_none());
    assert!(!s.group_messages.contains_key(group_id));
    assert!(!s.group_message_ids.contains_key(group_id));
    assert_eq!(s.dynamic_tabs.group_tab_count(), 0);
    assert!(matches!(
        s.dynamic_tabs.tab_index_to_content(s.active_tab),
        TabContent::Groups
    ));

    p2p_app::db::release_db_lock();
    p2p_app::db::reset_db_url();
}

#[test]
fn share_creation_prompt_is_modal_and_cancels_on_tab_switch() {
    let mut s = test_app_state();
    assert!(!s.is_creating_share());

    s.begin_create_share();
    assert!(s.is_creating_share());
    assert!(s.is_modal_editor());
    s.chat_input.insert_str("Notes");

    s.finalize_tab_input();
    assert!(!s.is_creating_share());
    assert_eq!(s.chat_input.lines().join("\n"), "");

    s.begin_create_share();
    assert!(s.cancel_create_share());
    assert!(!s.is_creating_share());
    assert!(!s.cancel_create_share(), "cancel is idempotent");
}

#[test]
fn share_binding_prompt_captures_its_target_and_prefills_the_root() {
    let mut s = test_app_state();
    assert!(!s.is_binding_share());
    assert!(!s.is_modal_editor());

    s.begin_bind_share("share-a".to_string(), Some("/tmp/shared"));
    assert!(s.is_binding_share());
    assert!(s.is_modal_editor());
    assert_eq!(
        s.share_binding_prompt
            .as_ref()
            .map(|prompt| prompt.share_id.as_str()),
        Some("share-a")
    );
    assert_eq!(s.chat_input.lines().join("\n"), "/tmp/shared");

    s.finalize_tab_input();
    assert!(!s.is_binding_share());
    assert!(s.share_binding_prompt.is_none());
    assert_eq!(s.chat_input.lines().join("\n"), "");

    s.begin_bind_share("share-b".to_string(), None);
    assert_eq!(s.chat_input.lines().join("\n"), "");
    assert!(s.cancel_share_binding());
    assert!(!s.cancel_share_binding(), "cancel is idempotent");
}

#[test]
fn share_peer_selection_uses_canonical_order_and_clamps() {
    use crossterm::event::KeyCode;

    let mut s = test_app_state();
    assert_eq!(s.share_peer_ids(), Vec::<String>::new());
    assert!(s.selected_share_peer_id().is_none());

    for peer in ["peer-c", "peer-a", "peer-b"] {
        s.connected.on_peer_connected(peer.to_string());
    }
    assert_eq!(s.share_peer_ids(), vec!["peer-a", "peer-b", "peer-c"]);
    assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-a"));

    s.move_share_peer_selection(KeyCode::Right);
    assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-b"));
    s.move_share_peer_selection(KeyCode::Left);
    assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-a"));

    s.share_peer_selection = usize::MAX;
    s.clamp_share_peer_selection();
    assert_eq!(s.selected_share_peer_id().as_deref(), Some("peer-c"));

    for peer in ["peer-a", "peer-b", "peer-c"] {
        s.connected.on_peer_disconnected(peer, 1.0);
    }
    s.clamp_share_peer_selection();
    assert_eq!(s.share_peer_selection, 0);
    assert!(s.selected_share_peer_id().is_none());
}

// ── discovered peer shares ────────────────────────────────────────────────

fn discovered_offer(share_id: &str, name: &str, revision: u64) -> p2p_app::shares::ShareOffer {
    p2p_app::shares::ShareOffer {
        share_id: share_id.to_string(),
        name: name.to_string(),
        kind: p2p_app::shares::ShareKind::Private,
        revision,
        public_write: false,
        owner_user_id: None,
    }
}

#[test]
fn discovered_share_selection_follows_rows_across_merge_and_removal() {
    let mut s = test_app_state();
    assert_eq!(s.discovered_shares, Vec::new());
    assert!(s.selected_discovered_share().is_none());

    s.record_discovered_share("peer-a".to_string(), discovered_offer("a", "Alpha", 1));
    s.record_discovered_share("peer-b".to_string(), discovered_offer("b", "Bravo", 2));
    s.record_discovered_share("peer-c".to_string(), discovered_offer("c", "Charlie", 3));
    assert_eq!(s.discovered_shares.len(), 3);
    assert_eq!(
        s.selected_discovered_share()
            .expect("first offer selected")
            .peer_id,
        "peer-a"
    );

    s.move_share_discovery_selection(1);
    s.move_share_discovery_selection(1);
    assert_eq!(s.share_discovery_selection, 2);
    s.move_share_discovery_selection(1);
    assert_eq!(s.share_discovery_selection, 2);

    // A retransmit replaces the row but must not retarget the highlighted offer.
    s.record_discovered_share("peer-c".to_string(), discovered_offer("c", "Charlie v2", 4));
    assert_eq!(s.discovered_shares.len(), 3);
    assert_eq!(s.share_discovery_selection, 2);
    assert_eq!(
        s.selected_discovered_share()
            .expect("selected offer survives")
            .offer
            .revision,
        4
    );

    s.remove_discovered_share("peer-a", "a");
    assert_eq!(s.share_discovery_selection, 1);
    assert_eq!(
        s.selected_discovered_share()
            .expect("selected offer follows its key")
            .peer_id,
        "peer-c"
    );

    s.remove_discovered_share("peer-c", "c");
    assert_eq!(s.share_discovery_selection, 0);
    assert_eq!(
        s.selected_discovered_share()
            .expect("clamped to remaining offer")
            .peer_id,
        "peer-b"
    );

    s.move_share_discovery_selection(-1);
    assert_eq!(s.share_discovery_selection, 0);
    s.share_discovery_selection = usize::MAX;
    s.clamp_share_discovery_selection();
    assert_eq!(s.share_discovery_selection, 0);
}
