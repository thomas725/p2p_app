//! Integration tests for TUI rendering using ratatui `TestBackend`
//! These tests verify the TUI rendering logic using the library module

#[cfg(feature = "tui")]
mod render_tests {
    use p2p_app::tui_render_state::get_tab_content;
    use p2p_app::{
        TuiRenderState,
        tui_render::{
            render_chat_content, render_dm_content, render_frame, render_input_section,
            render_log_content, render_peer_info_content, render_peers_content, render_popup,
            render_settings_content, render_shortcuts, render_status_bar, render_tabs,
            shortcuts_text,
        },
        tui_tabs::TabContent,
    };
    use ratatui::layout::Rect;
    use ratatui::{Terminal, backend::TestBackend};

    fn create_test_terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    #[test]
    fn test_render_frame_with_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

        terminal.draw(|f| render_frame(f, &mut state)).unwrap();
    }

    #[test]
    fn test_render_tabs_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal.draw(|f| render_tabs(f, f.area(), &state)).unwrap();
    }

    #[test]
    fn test_render_chat_content_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_chat_content(f, f.area(), &mut state))
            .unwrap();
    }

    #[test]
    fn test_render_chat_shows_unread_banner() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.chat_unread_count = 2;

        terminal
            .draw(|f| render_chat_content(f, f.area(), &mut state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("2 new messages"), "buffer: {text}");
    }

    #[test]
    fn test_render_chat_no_unread_banner_when_zero() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.chat_unread_count = 0;

        terminal
            .draw(|f| render_chat_content(f, f.area(), &mut state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(!text.contains("new message"), "buffer: {text}");
    }

    #[test]
    fn test_render_peers_content_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
    }

    #[test]
    fn test_render_peers_shows_total_count_in_title() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        for i in 0..6 {
            state.add_peer(
                format!("12D3KooWpeer{i}"),
                "2026-08-01 10:00:00",
                "2026-08-28 15:26:41",
            );
        }

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Contacts (6)"), "got: {text:?}");
    }

    #[test]
    fn test_render_peers_expands_device_subrows() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        state.contacts = vec![p2p_app::contacts::ContactRow {
            contact_id: "bob".to_string(),
            is_user: false,
            display_name: "Bob".to_string(),
            device_count: 2,
            device_rows: vec![
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d1".to_string(),
                    display_name: "Laptop".to_string(),
                    first_seen: "2026-08-01 10:00:00".to_string(),
                    last_seen: "2026-08-28 15:26:41".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                },
                p2p_app::contacts::ContactDevice {
                    peer_id: "bob-d2".to_string(),
                    display_name: "Phone".to_string(),
                    first_seen: "2026-08-01 10:00:00".to_string(),
                    last_seen: "2026-08-28 15:26:42".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                },
            ],
            first_seen: "2026-08-01 10:00:00".to_string(),
            last_seen: "2026-08-28 15:26:42".to_string(),
            dm_count: 3,
            broadcast_sent_to_peer: 2,
        }];
        state.expanded_contact_ids.insert("bob".to_string());
        // Select the first device sub-row (plan index 1) so the viewport math
        // exercises the flattened plan rather than the contact list.
        state.peer_selection = 1;

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        // The contact row keeps the contacts title count and shows the expanded
        // marker; both device sub-rows render with their indented names.
        assert!(text.contains("Contacts (1)"), "got: {text:?}");
        assert!(text.contains("\u{25be} Bob \u{00d7}2"), "got: {text:?}");
        assert!(text.contains("  Laptop"), "got: {text:?}");
        assert!(text.contains("  Phone"), "got: {text:?}");
        assert!(text.contains("15:26:41"), "got: {text:?}");
        assert!(text.contains("15:26:42"), "got: {text:?}");
    }

    #[test]
    fn test_render_peers_scrolls_viewport_but_keeps_selection_visible() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        // 30 peers in a tall terminal: the last row is far past the first page.
        // Distinct per-peer `last_seen` seconds let us spot which rows are on
        // screen, since display-name cells are truncated to the column width.
        for i in 0..30 {
            state.add_peer(
                format!("peer-{i:02}"),
                "2026-08-01 10:00:00",
                format!("2026-08-28 15:26:{i:02}"),
            );
        }
        state.peer_selection = 29; // the last row in seen-descending order

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        // The selection (the least-recently-seen contact, injected last by
        // `build_contact_rows`' descending seen sort) sits on the last visible
        // data row...
        assert!(text.contains("15:26:00"), "got: {text:?}");
        // ...so the most recent rows are scrolled out (the ascending-seen
        // tail, `start=10`, paged past the top of the viewport).
        assert!(!text.contains("15:26:29"), "got: {text:?}");
        // Only one page of rows is drawn (20 of the 30 peers).
        let visible_seconds = (0..20)
            .filter(|s| text.contains(&format!("15:26:{s:02}")))
            .count();
        assert_eq!(visible_seconds, 20, "got: {text:?}");
        // The total is still reported in the title.
        assert!(text.contains("Contacts (30)"), "got: {text:?}");
    }

    #[test]
    fn test_render_dm_content_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.add_dm_message("Alice", "<Alice> Hello!");

        terminal
            .draw(|f| {
                let area = f.area();
                let block = ratatui::widgets::Block::default()
                    .title("DM: Alice")
                    .borders(ratatui::widgets::Borders::ALL);
                let inner = block.inner(area);
                render_dm_content(f, inner, "Alice", &mut state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_log_content_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_log_content(f, f.area(), &state))
            .unwrap();
    }

    #[test]
    fn test_render_input_section_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 80, 5);
                let tab = get_tab_content(&state);
                render_input_section(f, area, &state, &tab);
            })
            .unwrap();
    }

    /// The input-section title names the nickname flavor being edited, so the
    /// per-peer local edit reads differently from the own-nickname one.
    #[test]
    fn test_render_input_section_titles_nickname_flavors() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.nickname_peer_id = "peer-abcd1234".to_string();
        let tab = TabContent::Chat;

        let buffer_text = |terminal: &Terminal<TestBackend>| -> String {
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect()
        };

        state.input_mode = p2p_app::InputMode::EditingNickname;
        terminal
            .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Edit Nickname"), "buffer: {text}");
        assert!(!text.contains("Local Nickname"), "buffer: {text}");

        state.input_mode = p2p_app::InputMode::EditingLocalNickname;
        terminal
            .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("Local Nickname"), "buffer: {text}");

        // Back to plain chat input: neither flavor is named.
        state.input_mode = p2p_app::InputMode::Chat;
        terminal
            .draw(|f| render_input_section(f, Rect::new(0, 0, 80, 5), &state, &tab))
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(!text.contains("Nickname"), "buffer: {text}");
    }

    /// Opening the Contacts search box relabels the shared input section and
    /// echoes the query, even though the Contacts tab has no chat input.
    #[test]
    fn test_render_input_section_titles_contacts_search() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.input_text = "bob".to_string();
        state.input_mode = p2p_app::InputMode::SearchingContacts;

        terminal
            .draw(|f| {
                render_input_section(f, Rect::new(0, 0, 80, 5), &state, &TabContent::Contacts);
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Search Contacts"), "buffer: {text}");
        // The query is shown inside the box (the Contacts tab has no chat input).
        assert!(text.contains("bob"), "buffer: {text}");
    }

    #[test]
    fn test_render_shortcuts_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();
        let tab = get_tab_content(&state);

        terminal
            .draw(|f| render_shortcuts(f, f.area(), &tab, false))
            .unwrap();
    }

    #[test]
    fn test_shortcuts_peers_tab_documents_dm_and_info() {
        let text = shortcuts_text(&TabContent::Contacts, false);
        assert!(text.contains("Enter: open DM"));
        assert!(text.contains("i: Peer Info"));
        assert!(text.contains("c: call"));
        assert!(text.contains("/: search"));
        assert!(!text.contains("Ctrl+I: Peer Info"));
    }

    #[test]
    fn test_shortcuts_peer_info_tab_documents_call() {
        let text = shortcuts_text(&TabContent::PeerInfo("peer-1".to_string()), false);
        assert!(text.contains("Enter: open DM"));
        assert!(text.contains("c: call"));
        assert!(text.contains("n: set local nickname"));
        assert!(!text.contains("Peer Info"));
    }

    #[test]
    fn test_shortcuts_direct_tab_documents_ctrl_i_and_ctrl_p_on_kitty() {
        let text = shortcuts_text(&TabContent::Direct("peer-1".to_string()), true);
        assert!(text.contains("Ctrl+I/Ctrl+P: Peer Info"));
        assert!(!text.contains("| i: Peer Info"));
        assert!(text.contains("Ctrl+I"));
        assert!(text.contains("Ctrl+P"));
    }

    #[test]
    fn test_shortcuts_direct_tab_documents_ctrl_p_on_nonkitty() {
        let text = shortcuts_text(&TabContent::Direct("peer-1".to_string()), false);
        assert!(text.contains("Ctrl+P: Peer Info"));
        assert!(!text.contains("Ctrl+I: Peer Info"));
    }

    #[test]
    fn test_shortcuts_plain_tabs_have_no_peer_info_hint() {
        for tab in [
            TabContent::Chat,
            TabContent::Log,
            TabContent::Settings,
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(!shortcuts_text(&tab, false).contains("Peer Info"));
            assert!(!shortcuts_text(&tab, true).contains("Peer Info"));
        }
    }

    #[test]
    fn test_render_status_bar_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_status_bar(f, f.area(), &state))
            .unwrap();
    }

    #[test]
    fn test_render_popup_library() {
        let mut terminal = create_test_terminal();

        terminal
            .draw(|f| render_popup(f, "Test popup message".to_string()))
            .unwrap();
    }

    #[test]
    fn test_full_frame_all_tabs() {
        let mut terminal = create_test_terminal();

        // Test Chat tab
        let mut state = TuiRenderState::with_sample_data();
        state.active_tab = 0;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Test Peers tab
        state.active_tab = 1;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Test Log tab
        state.active_tab = 2;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();
    }

    #[test]
    fn test_render_settings_content_no_connections_shows_last_connected() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        // No peer currently connected: only the last-seen peer is remembered.
        state.add_peer(
            "12D3KooWH123456",
            "2026-08-01 10:00:00",
            "2026-08-28 15:26:41",
        );

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Connected peers: 0"), "got: {text:?}");
        assert!(
            text.contains("Last connected 2026-08-28 15:26:41 to peer"),
            "got: {text:?}"
        );
        assert!(!text.contains("last seen:"), "got: {text:?}");
    }

    #[test]
    fn test_render_settings_content_connected_peers_shown() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        state
            .connected_peer_ids
            .insert("12D3KooWH123456".to_string());
        state.peer_count = 1;
        state.add_peer(
            "12D3KooWH123456",
            "2026-08-01 10:00:00",
            "2026-08-28 15:26:41",
        );

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Connected peers: 1"), "got: {text:?}");
        assert!(!text.contains("Last connected"), "got: {text:?}");
        assert!(!text.contains("last seen:"), "got: {text:?}");
    }

    #[test]
    fn test_render_settings_content_known_but_not_connected_is_not_listed() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        // A known peer that is NOT in connected_peer_ids must not appear as a
        // currently connected peer.
        state.add_peer(
            "12D3KooWH123456",
            "2026-08-01 10:00:00",
            "2026-08-28 15:26:41",
        );

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Connected peers: 0"), "got: {text:?}");
        assert!(!text.contains("Connected peers: 1"), "got: {text:?}");
    }

    #[test]
    fn test_render_settings_content_single_fallback_uses_recent_peer() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        // Try a state actually produced by the render-loop mapping: the
        // tracker's last disconnection feeds these two fields.
        state.last_connection_peer = Some("12D3KooWH789012".to_string());
        state.last_connection_lost = Some(1_756_481_201.0);

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Connected peers: 0"), "got: {text:?}");
        assert!(text.contains("Last connected"), "got: {text:?}");
    }

    #[test]
    fn test_interactive_state_changes() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();

        // Add messages
        state.add_message("[You] msg1");
        state.add_message("[Peer] msg2");
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Add more messages
        state.add_message("[You] msg3");
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Add peers
        state.add_peer("id1", "Alice", "Online");
        state.add_peer("id2", "Bob", "Away");
        state.active_tab = 1;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();
    }

    #[test]
    fn test_popup_rendering() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        state.popup = Some("Popup content".to_string());

        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Clear popup
        state.popup = None;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();
    }

    #[test]
    fn test_render_settings_content_own_user_identity() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        state.own_user_id = Some("user-abc".to_string());
        state.own_user_devices = vec![
            (
                "12D3KooWH123456".to_string(),
                "2026-09-25 10:00:00".to_string(),
            ),
            (
                "12D3KooWH789012".to_string(),
                "2026-09-25 11:30:00".to_string(),
            ),
        ];

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("User ID: user-abc"), "got: {text:?}");
        assert!(text.contains("Bound devices: 2"), "got: {text:?}");
        assert!(
            text.contains("(bound 2026-09-25 10:00:00)"),
            "got: {text:?}"
        );
        assert!(
            text.contains("(bound 2026-09-25 11:30:00)"),
            "got: {text:?}"
        );
    }

    #[test]
    fn test_render_settings_content_own_user_unprovisioned() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::new();

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("User ID: —"), "got: {text:?}");
    }

    #[test]
    fn test_render_peer_info_shows_contact_message_stats() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        // The stats come from the contact row that owns the device, so they
        // always match the number the Contacts tab shows for that row.
        let peer_id = state.contacts[0].device_rows[0].peer_id.clone();
        state.contacts[0].dm_count = 3;
        state.contacts[0].broadcast_sent_to_peer = 7;

        terminal
            .draw(|f| render_peer_info_content(f, f.area(), &peer_id, &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Direct messages: 3"), "got: {text:?}");
        assert!(text.contains("Broadcasts sent: 7"), "got: {text:?}");
    }

    #[test]
    fn test_render_peer_info_omits_stats_for_peer_without_a_contact_row() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| {
                render_peer_info_content(f, f.area(), "peer-not-in-roster", &state);
            })
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("Peer ID: peer-not-in-roster"),
            "got: {text:?}"
        );
        assert!(!text.contains("Direct messages"), "got: {text:?}");
        assert!(!text.contains("Broadcasts sent"), "got: {text:?}");
    }
}

// ── TuiRenderState builder methods + get_tab_content ─────────────────────────

#[test]
fn test_add_message() {
    use p2p_app::tui_render_state::TuiRenderState;
    let mut state = TuiRenderState::new();
    state.add_message("hello");
    state.add_message("world");
    assert_eq!(state.messages.len(), 2);
    assert_eq!(state.messages[0], "hello");
}

#[test]
fn test_add_peer() {
    use p2p_app::tui_render_state::TuiRenderState;
    let mut state = TuiRenderState::new();
    state.add_peer("peer-1", "Alice", "Online");
    assert_eq!(state.peers.len(), 1);
    assert_eq!(state.peers[0].peer_id, "peer-1");
    assert_eq!(state.peers[0].first_seen, "Alice");
}

#[test]
fn test_add_dm_message() {
    use p2p_app::tui_render_state::TuiRenderState;
    let mut state = TuiRenderState::new();
    state.add_dm_message("peer-dm", "hello dm");
    state.add_dm_message("peer-dm", "second dm");
    let msgs = state.dm_messages.get("peer-dm").unwrap();
    assert_eq!(msgs.len(), 2);
    assert_eq!(msgs[0], "hello dm");
}

#[test]
fn test_get_tab_content_chat() {
    use p2p_app::tui_render_state::{TuiRenderState, get_tab_content};
    use p2p_app::tui_tabs::TabContent;
    let mut state = TuiRenderState::new();
    state.active_tab = 0; // "Chat"
    assert!(matches!(get_tab_content(&state), TabContent::Chat));
}

#[test]
fn test_get_tab_content_peers() {
    use p2p_app::tui_render_state::{TuiRenderState, get_tab_content};
    use p2p_app::tui_tabs::TabContent;
    let mut state = TuiRenderState::new();
    state.active_tab = 1; // "Contacts"
    assert!(matches!(get_tab_content(&state), TabContent::Contacts));
}

#[test]
fn test_get_tab_content_log() {
    use p2p_app::tui_render_state::{TuiRenderState, get_tab_content};
    use p2p_app::tui_tabs::TabContent;
    let mut state = TuiRenderState::new();
    state.active_tab = 2; // "Groups"
    assert!(matches!(get_tab_content(&state), TabContent::Groups));
    state.active_tab = 3; // "Log"
    assert!(matches!(get_tab_content(&state), TabContent::Log));
}

#[test]
fn test_get_tab_content_dm() {
    use p2p_app::tui_render_state::{TuiRenderState, get_tab_content};
    use p2p_app::tui_tabs::TabContent;
    let mut state = TuiRenderState::new();
    state.tab_titles.push("DM: peer-xyz".to_string());
    state.active_tab = state.tab_titles.len() - 1;
    match get_tab_content(&state) {
        TabContent::Direct(peer) => assert_eq!(peer, "peer-xyz"),
        other => panic!("expected Direct, got {other:?}"),
    }
}

#[test]
fn test_tui_tab_content_is_input_enabled() {
    use p2p_app::tui_tabs::TabContent;
    assert!(TabContent::Chat.is_input_enabled());
    assert!(TabContent::Direct("p".into()).is_input_enabled());
    assert!(!TabContent::Contacts.is_input_enabled());
    assert!(!TabContent::Log.is_input_enabled());
}

// ── tui_render_state pure helper functions ────────────────────────────────────

#[test]
fn test_count_lines_single_line() {
    use p2p_app::tui_render_state::count_lines;
    assert_eq!(count_lines("hello", 80), 1);
}

#[test]
fn test_count_lines_empty_string() {
    use p2p_app::tui_render_state::count_lines;
    assert_eq!(count_lines("", 80), 1);
}

#[test]
fn test_count_lines_with_newlines() {
    use p2p_app::tui_render_state::count_lines;
    assert_eq!(count_lines("line1\nline2\nline3", 80), 3);
}

#[test]
fn test_count_lines_wrapping() {
    use p2p_app::tui_render_state::count_lines;
    // 100 chars in 40-width = ceil(100/40) = 3 lines
    let text = "a".repeat(100);
    assert_eq!(count_lines(&text, 40), 3);
}

#[test]
fn test_count_lines_zero_width() {
    use p2p_app::tui_render_state::count_lines;
    assert_eq!(count_lines("hello", 0), 1);
}

#[test]
fn test_broadcast_receipt_prefix_no_receipts() {
    use p2p_app::tui_render_state::broadcast_receipt_prefix;
    use std::collections::HashMap;
    let receipts: HashMap<String, HashMap<String, f64>> = HashMap::new();
    assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "  ");
}

#[test]
fn test_broadcast_receipt_prefix_with_receipt() {
    use p2p_app::tui_render_state::broadcast_receipt_prefix;
    use std::collections::HashMap;
    let mut receipts: HashMap<String, HashMap<String, f64>> = HashMap::new();
    receipts.insert(
        "msg-1".to_string(),
        HashMap::from([("p1".to_string(), 1.0)]),
    );
    assert_eq!(broadcast_receipt_prefix(Some("msg-1"), &receipts), "v ");
}

#[test]
fn test_dm_receipt_prefix_present() {
    use p2p_app::tui_render_state::dm_receipt_prefix;
    use std::collections::HashMap;
    let receipts = HashMap::from([("msg-1".to_string(), ("p1".to_string(), 1.0))]);
    assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "v ");
}

#[test]
fn test_dm_receipt_prefix_absent() {
    use p2p_app::tui_render_state::dm_receipt_prefix;
    use std::collections::HashMap;
    let receipts: HashMap<String, (String, f64)> = HashMap::new();
    assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "  ");
}

#[test]
fn test_calc_visible_strings() {
    use p2p_app::tui_render_state::calc_visible_strings;
    use std::collections::VecDeque;
    let strings = VecDeque::from(vec![
        "line1".to_string(),
        "line2".to_string(),
        "line3".to_string(),
    ]);
    let (visible, offset) = calc_visible_strings(&strings, false, 0, 80, 10);
    assert!(visible <= strings.len());
    assert_eq!(offset, 0);
}

#[test]
fn test_calc_visible_strings_empty() {
    use p2p_app::tui_render_state::calc_visible_strings;
    use std::collections::VecDeque;
    let strings: VecDeque<String> = VecDeque::new();
    let (visible, offset) = calc_visible_strings(&strings, true, 0, 80, 10);
    assert_eq!(visible, 0);
    assert_eq!(offset, 0);
}

#[test]
fn test_tui_render_state_with_sample_data() {
    use p2p_app::tui_render_state::TuiRenderState;
    let state = TuiRenderState::with_sample_data();
    assert!(
        !state.messages.is_empty(),
        "sample data should have messages"
    );
    assert!(!state.peers.is_empty(), "sample data should have peers");
    assert!(
        !state.tab_titles.is_empty(),
        "sample data should have tab titles"
    );
}

#[test]
fn test_row_to_visible_index_in_range() {
    use p2p_app::tui_render_state::row_to_visible_index;
    let line_counts = vec![1, 2, 1, 3]; // 4 visible items
    let first_content = 5;
    // Row 5: first item
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 5),
        Some(0)
    );
    // Row 6-7: second item (2 lines)
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 6),
        Some(1)
    );
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 7),
        Some(1)
    );
}

#[test]
fn test_row_to_visible_index_out_of_bounds() {
    use p2p_app::tui_render_state::row_to_visible_index;
    let line_counts = vec![1, 1];
    let first_content = 10;
    // Click way beyond content
    assert_eq!(row_to_visible_index(&line_counts, first_content, 100), None);
}

#[test]
fn test_row_to_visible_index_exact_last_item() {
    use p2p_app::tui_render_state::row_to_visible_index;
    let line_counts = vec![2, 1];
    let first_content = 5;
    // Rows 5-6 are first item, row 7 is second item
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 7),
        Some(1)
    );
}

// ── Additional TuiRenderState coverage ──────────────────────────────────────────

#[test]
fn test_add_message_multiple_calls() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.add_message("msg1");
    state.add_message("msg2");
    state.add_message("msg3");
    assert_eq!(state.messages.len(), 3);
}

#[test]
fn test_add_message_empty_string() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.add_message("");
    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0], "");
}

#[test]
fn test_add_peer_multiple() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.add_peer("peer1", "Alice", "2024-01-01");
    state.add_peer("peer2", "Bob", "2024-01-01");
    state.add_peer("peer3", "", "2024-01-01");
    assert_eq!(state.peers.len(), 3);
}

#[test]
fn test_add_dm_message_creates_peer_entry() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.add_dm_message("peer-x", "hello");
    assert!(state.dm_messages.contains_key("peer-x"));
    assert_eq!(state.dm_messages["peer-x"].len(), 1);
}

#[test]
fn test_add_dm_message_multiple_peers() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.add_dm_message("peer-a", "msg-a");
    state.add_dm_message("peer-b", "msg-b");
    state.add_dm_message("peer-a", "msg-a2");
    assert_eq!(state.dm_messages.len(), 2);
    assert_eq!(state.dm_messages["peer-a"].len(), 2);
    assert_eq!(state.dm_messages["peer-b"].len(), 1);
}

#[test]
fn test_default_vs_new() {
    let default_state = p2p_app::tui_render_state::TuiRenderState::default();
    let new_state = p2p_app::tui_render_state::TuiRenderState::new();
    assert_eq!(default_state.messages.len(), new_state.messages.len());
    assert_eq!(default_state.peers.len(), new_state.peers.len());
}

#[test]
fn test_with_sample_data_contains_content() {
    let state = p2p_app::tui_render_state::TuiRenderState::with_sample_data();
    assert!(
        !state.messages.is_empty(),
        "sample data should have messages"
    );
    assert!(!state.peers.is_empty(), "sample data should have peers");
    assert!(state.messages.iter().any(|msg| msg.contains("Hello")));
}

#[test]
fn test_count_lines_with_very_long_word() {
    use p2p_app::tui_render_state::count_lines;
    // A word longer than the width
    let long_word = "supercalifragilisticexpialidocious";
    let lines = count_lines(long_word, 10);
    assert!(lines >= 1);
}

#[test]
fn test_count_lines_mixed_newlines_and_wrapping() {
    use p2p_app::tui_render_state::count_lines;
    // Line 1: "hello" (wraps to 2 in 3-char width)
    // Line 2: "world" (wraps to 2 in 3-char width)
    let text = "hello\nworld";
    let lines = count_lines(text, 3);
    // Should count multiple lines with wrapping
    assert!(lines >= 2);
}

#[test]
fn test_row_to_visible_index_single_line_each() {
    use p2p_app::tui_render_state::row_to_visible_index;
    let line_counts = vec![1, 1, 1];
    let first_content = 10;
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 10),
        Some(0)
    );
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 11),
        Some(1)
    );
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 12),
        Some(2)
    );
}

#[test]
fn test_row_to_visible_index_multiline_items() {
    use p2p_app::tui_render_state::row_to_visible_index;
    let line_counts = vec![3, 2, 1]; // Items with 3, 2, 1 lines respectively
    let first_content = 0;
    // Rows 0-2 are item 0
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 0),
        Some(0)
    );
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 2),
        Some(0)
    );
    // Rows 3-4 are item 1
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 3),
        Some(1)
    );
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 4),
        Some(1)
    );
    // Row 5 is item 2
    assert_eq!(
        row_to_visible_index(&line_counts, first_content, 5),
        Some(2)
    );
}

#[test]
fn test_get_tab_content_different_tabs() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    state.active_tab = 0;
    let content_0 = p2p_app::tui_render_state::get_tab_content(&state);
    state.active_tab = 1;
    let content_1 = p2p_app::tui_render_state::get_tab_content(&state);
    // Different tabs should return different content types or values
    let _ = (content_0, content_1);
}

// ── TuiRenderState field access tests ────────────────────────────────────────

#[test]
fn test_state_connected_default() {
    let state = p2p_app::tui_render_state::TuiRenderState::new();
    assert!(state.runtime.connected);
    assert_eq!(state.peer_count, 0);
}

#[test]
fn test_state_popup() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    assert!(state.popup.is_none());
    state.popup = Some("test".to_string());
    assert_eq!(state.popup.as_deref(), Some("test"));
}

#[test]
fn test_state_input_text() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    assert!(state.input_text.is_empty());
    state.input_text = "hello".to_string();
    assert_eq!(state.input_text, "hello");
}

#[test]
fn test_state_editing_nickname() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    assert!(!state.is_editing_nickname());
    state.input_mode = p2p_app::InputMode::EditingNickname;
    state.nickname_peer_id = "peer-1".to_string();
    assert!(state.is_editing_nickname());
    assert!(!state.is_editing_local_nickname());
    assert_eq!(state.nickname_peer_id, "peer-1");

    // The local-nickname mode counts as a nickname edit too, but is
    // distinguishable so the input-section title can say which flavor it is.
    state.input_mode = p2p_app::InputMode::EditingLocalNickname;
    assert!(state.is_editing_nickname());
    assert!(state.is_editing_local_nickname());
}

#[test]
fn test_state_scroll_defaults() {
    let state = p2p_app::tui_render_state::TuiRenderState::new();
    assert_eq!(state.chat_scroll_offset, 0);
    assert!(state.auto_scroll.chat);
}

#[test]
fn test_state_peer_selection() {
    let mut state = p2p_app::tui_render_state::TuiRenderState::new();
    assert_eq!(state.peer_selection, 0);
    state.peer_selection = 3;
    assert_eq!(state.peer_selection, 3);
}

#[test]
fn test_state_with_sample_data_extra() {
    let state = p2p_app::tui_render_state::TuiRenderState::with_sample_data();
    assert_eq!(state.tab_titles.len(), 4);
    assert!(state.runtime.connected);
}

#[test]
fn test_state_default_equals_new() {
    let a = p2p_app::tui_render_state::TuiRenderState::default();
    let b = p2p_app::tui_render_state::TuiRenderState::new();
    assert_eq!(a.tab_titles, b.tab_titles);
    assert_eq!(a.active_tab, b.active_tab);
    assert_eq!(a.runtime.connected, b.runtime.connected);
}
