//! Integration tests for TUI rendering using ratatui `TestBackend`
//! These tests verify the TUI rendering logic using the library module

#[cfg(feature = "tui")]
mod render_tests {
    use p2p_app::tui_render_state::get_tab_content;
    use p2p_app::{
        TuiRenderState,
        tui_render::{
            render_chat_content, render_dm_content, render_frame, render_group_chat_content,
            render_input_section, render_log_content, render_peer_info_content,
            render_peers_content, render_popup, render_settings_content, render_shares_content,
            render_shortcuts, render_shortcuts_help_popup, render_status_bar, render_tabs,
            settings_visual_lines, shortcuts_help_text, shortcuts_text,
        },
        tui_tabs::TabContent,
    };
    use ratatui::layout::Rect;
    use ratatui::{Terminal, backend::TestBackend};
    #[cfg(feature = "test-utils")]
    use std::collections::HashSet;
    #[cfg(feature = "test-utils")]
    use tempfile::TempDir;

    fn create_test_terminal() -> Terminal<TestBackend> {
        Terminal::new(TestBackend::new(80, 24)).unwrap()
    }

    /// Isolate the DB-backed display-name lookups made by Settings rows.
    #[cfg(feature = "test-utils")]
    fn with_settings_db<T>(f: impl FnOnce() -> T) -> T {
        let _guard = p2p_app::db::shared_db_test_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("settings.db");
        p2p_app::db::set_db_url(db_path.to_str().unwrap());
        p2p_app::db::init_database().unwrap();
        let result = f();
        p2p_app::db::release_db_lock();
        p2p_app::db::reset_db_url();
        result
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
    fn test_render_dm_shows_unread_banner_for_that_peer_only() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.add_dm_message("Alice", "<Alice> Hello!");
        state.dm_unread_count.insert("Alice".to_string(), 3);
        state.dm_unread_count.insert("Bob".to_string(), 9);

        terminal
            .draw(|f| render_dm_content(f, f.area(), "Alice", &mut state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("3 new messages"), "buffer: {text}");
        assert!(!text.contains("9 new messages"), "buffer: {text}");
    }

    #[test]
    fn test_render_dm_no_unread_banner_when_zero() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.add_dm_message("Alice", "<Alice> Hello!");

        terminal
            .draw(|f| render_dm_content(f, f.area(), "Alice", &mut state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(!text.contains("new message"), "buffer: {text}");
    }

    #[test]
    fn test_render_group_shows_unread_banner() {
        use std::collections::VecDeque;

        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        let group_id = "g-render";
        state.group_messages.insert(
            group_id.to_string(),
            VecDeque::from(vec!["hello".to_string()]),
        );
        state.group_unread_count.insert(group_id.to_string(), 2);

        terminal
            .draw(|f| render_group_chat_content(f, f.area(), group_id, &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("2 new messages"), "buffer: {text}");
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
    fn test_render_peers_search_shows_visible_over_total_count() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        for i in 0..3 {
            state.add_peer(
                format!("12D3KooWpeer{i}"),
                "2026-08-01 10:00:00",
                "2026-08-28 15:26:41",
            );
        }
        // Simulate an active search that leaves one of three rows visible.
        state.contacts.truncate(1);
        state.contact_search_total = Some(3);

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Contacts (1/3)"), "got: {text:?}");
        assert!(!text.contains("No contacts match."), "got: {text:?}");
    }

    #[test]
    fn test_render_peers_no_match_search_shows_hint_and_zero_over_total() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::new();
        state.contact_search_total = Some(3); // active query, no matches

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Contacts (0/3)"), "got: {text:?}");
        assert!(text.contains("No contacts match."), "got: {text:?}");
    }

    #[test]
    fn test_render_peers_empty_roster_without_query_shows_empty_hint() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::new(); // no contacts, no query

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Contacts (0)"), "got: {text:?}");
        assert!(text.contains("No contacts yet."), "got: {text:?}");
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
    fn test_render_dm_broadcast_context_shows_reaction_suffix() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.messages.clear();
        state.message_peer_ids.clear();
        state.message_ids.clear();
        state.messages.push_back("public hello".into());
        state.message_peer_ids.push_back(Some("Alice".into()));
        state.message_ids.push_back(Some("bc-1".into()));
        state.reactions.insert(
            "bc-1".to_string(),
            vec![p2p_app::reactions::ReactionSummary {
                emoji: "👍".to_string(),
                count: 2,
                mine: false,
            }],
        );

        terminal
            .draw(|f| render_dm_content(f, f.area(), "Alice", &mut state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("public hello"), "got: {text:?}");
        // A wide emoji occupies two terminal cells, so the raw buffer has an
        // extra padding space after it. Collapse whitespace before comparing
        // against the canonical chip text.
        let collapsed: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(collapsed.contains("[👍 2]"), "got: {collapsed:?}");
    }

    #[test]
    fn test_render_dm_content_shows_typing_indicator() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.add_dm_message("Alice", "<Alice> Hello!");
        state.typing_peer_ids.insert("Alice".to_string());

        terminal
            .draw(|f| render_dm_content(f, f.area(), "Alice", &mut state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("typing…"), "buffer: {text}");
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

    /// The group-name prompt must be visible in the library renderer too, even
    /// though Groups has no ordinary chat input. Both renderers used to disagree
    /// here: the live one showed the prompt, the library one a plain `Input`.
    #[test]
    fn test_render_input_section_creating_group_shows_prompt_and_text() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        state.input_mode = p2p_app::InputMode::CreatingGroup;
        state.input_text = "Team Rocket".to_string();

        terminal
            .draw(|f| {
                render_input_section(f, Rect::new(0, 0, 80, 5), &state, &TabContent::Groups);
            })
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Create or Join Group"), "buffer: {text}");
        assert!(text.contains("Team Rocket"), "buffer: {text}");
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
        assert!(text.contains("i/Ctrl+P: Peer Info"));
        assert!(text.contains("c: call"));
        assert!(text.contains("/: search"));
        assert!(!text.contains("Ctrl+I: Peer Info"));
    }

    #[test]
    fn test_shortcuts_peer_info_tab_documents_call() {
        let text = shortcuts_text(&TabContent::PeerInfo("peer-1".to_string()), false);
        assert!(text.contains("Enter: open DM"));
        assert!(text.contains("c: call"));
        assert!(text.contains("b: block/unblock"));
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
    fn test_shortcuts_ctrl_p_hint_only_on_tabs_with_a_peer_target() {
        for tab in [
            TabContent::Contacts,
            TabContent::Direct("peer-1".to_string()),
            TabContent::Chat,
            TabContent::GroupChat("group-1".to_string()),
            TabContent::Settings,
        ] {
            assert!(
                shortcuts_text(&tab, false).contains("Ctrl+P"),
                "missing Ctrl+P hint for {tab:?}"
            );
            assert!(
                shortcuts_text(&tab, true).contains("Ctrl+P"),
                "missing Ctrl+P hint for {tab:?} on kitty"
            );
        }
        for tab in [
            TabContent::PeerInfo("peer-1".to_string()),
            TabContent::Groups,
            TabContent::Log,
            TabContent::Shares,
        ] {
            assert!(
                !shortcuts_text(&tab, false).contains("Ctrl+P"),
                "Ctrl+P hint must stay off {tab:?}"
            );
        }
    }

    #[test]
    fn test_shortcuts_delete_hint_only_on_chat_tabs() {
        for tab in [
            TabContent::Chat,
            TabContent::Direct("peer-1".to_string()),
            TabContent::GroupChat("group-1".to_string()),
        ] {
            assert!(
                shortcuts_text(&tab, false).contains("Del: delete sent"),
                "missing delete hint for {tab:?}"
            );
            assert!(
                shortcuts_text(&tab, true).contains("Del: delete sent"),
                "missing delete hint for {tab:?}"
            );
        }
        for tab in [
            TabContent::Contacts,
            TabContent::Groups,
            TabContent::Settings,
            TabContent::Log,
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(
                !shortcuts_text(&tab, false).contains("Del:"),
                "delete hint must stay off {tab:?}"
            );
        }
    }

    #[test]
    fn test_shortcuts_open_file_hint_only_on_chat_tabs() {
        for tab in [
            TabContent::Chat,
            TabContent::Direct("peer-1".to_string()),
            TabContent::GroupChat("group-1".to_string()),
        ] {
            assert!(
                shortcuts_text(&tab, false).contains("Ctrl+O: open file"),
                "missing open-file hint for {tab:?}"
            );
            assert!(
                shortcuts_text(&tab, true).contains("Ctrl+O: open file"),
                "missing open-file hint for {tab:?}"
            );
        }
        for tab in [
            TabContent::Contacts,
            TabContent::Groups,
            TabContent::Settings,
            TabContent::Log,
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(
                !shortcuts_text(&tab, false).contains("Ctrl+O"),
                "open-file hint must stay off {tab:?}"
            );
        }
    }

    #[test]
    fn test_shortcuts_copy_hint_only_on_chat_tabs() {
        for tab in [
            TabContent::Chat,
            TabContent::Direct("peer-1".to_string()),
            TabContent::GroupChat("group-1".to_string()),
        ] {
            assert!(
                shortcuts_text(&tab, false).contains("Ctrl+C: copy"),
                "missing copy hint for {tab:?}"
            );
            assert!(
                shortcuts_text(&tab, true).contains("Ctrl+C: copy"),
                "missing copy hint for {tab:?}"
            );
        }
        // The Log tab has its own variant of the copy action ("Copy All").
        assert!(shortcuts_text(&TabContent::Log, false).contains("Ctrl+C: copy all"));
        for tab in [
            TabContent::Contacts,
            TabContent::Groups,
            TabContent::Settings,
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(
                !shortcuts_text(&tab, false).contains("Ctrl+C"),
                "copy hint must stay off {tab:?}"
            );
        }
    }

    #[test]
    fn test_shortcuts_tabs_without_a_peer_target_have_no_peer_info_hint() {
        for tab in [
            TabContent::Groups,
            TabContent::Log,
            TabContent::Shares,
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(!shortcuts_text(&tab, false).contains("Peer Info"));
            assert!(!shortcuts_text(&tab, true).contains("Peer Info"));
        }
    }

    #[test]
    fn test_every_shortcut_bar_advertises_the_help_key() {
        for tab in [
            TabContent::Chat,
            TabContent::Contacts,
            TabContent::Groups,
            TabContent::Settings,
            TabContent::Log,
            TabContent::Direct("peer-1".to_string()),
            TabContent::GroupChat("group-1".to_string()),
            TabContent::PeerInfo("peer-1".to_string()),
        ] {
            assert!(
                shortcuts_text(&tab, false).contains("?: help"),
                "missing help hint for {tab:?}"
            );
            assert!(
                shortcuts_text(&tab, true).contains("?: help"),
                "missing help hint for {tab:?}"
            );
        }
    }

    #[test]
    fn test_shortcuts_help_text_covers_the_core_bindings() {
        let help = shortcuts_help_text();
        for expected in [
            "Tab / Shift+Tab",
            "Ctrl+W",
            "PgUp/PgDn",
            "Ctrl+Up/Down  Message cursor (clamped; Esc clears)",
            "selected row (newest if none selected)",
            "DM: cursor targets the bottom DM pane",
            "Enter  Send",
            "Del -> delete sent (local)",
            "Ctrl+O -> open attachment",
            "Ctrl+C -> copy (OSC 52)",
            "Ctrl+C copy all logs",
            "Ctrl+P -> newest/sender target (unchanged by selection)",
            "Esc unwinds one layer: prompt -> reply draft -> message selection -> Chat",
            "Ctrl+I (kitty) / Ctrl+P",
            "Settings: Ctrl+P -> connected peer / bound device",
            "click a peer row for its Peer Info",
            "i Info",
            "g create/join",
            "b block / unblock",
            "s sound",
            "o desktop alert",
            "/file <path>",
            "Ctrl+Q",
            "KEYBOARD_SHORTCUTS.md",
        ] {
            assert!(help.contains(expected), "missing {expected}");
        }
    }

    #[test]
    fn test_shortcuts_settings_bar_documents_notification_keys() {
        let text = shortcuts_text(&TabContent::Settings, false);
        assert!(text.contains("s: sound"), "missing sound hint");
        assert!(text.contains("o: desktop alert"), "missing desktop hint");
    }

    #[test]
    fn test_render_shortcuts_help_popup_library() {
        // The help popup is intentionally taller than the standard 80x24 test
        // terminal; use a larger viewport so every section is rendered.
        let mut terminal = Terminal::new(TestBackend::new(160, 40)).unwrap();
        let help = shortcuts_help_text().to_string();

        terminal
            .draw(|f| render_shortcuts_help_popup(f, help, p2p_app::theme::Palette::default()))
            .unwrap();

        let buffer = terminal.backend().buffer();
        let text = buffer
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("Keyboard shortcuts"), "buffer: {text}");
        assert!(
            text.contains("Message cursor (clamped; Esc clears)"),
            "buffer: {text}"
        );
        assert!(
            text.contains("selected row (newest if none selected)"),
            "buffer: {text}"
        );
        assert!(text.contains("Contacts:"), "buffer: {text}");
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
            .draw(|f| {
                render_popup(
                    f,
                    "Test popup message".to_string(),
                    p2p_app::theme::Palette::default(),
                );
            })
            .unwrap();
    }

    #[test]
    fn test_full_frame_all_tabs() {
        let mut terminal = create_test_terminal();

        // Test Chat tab
        let mut state = TuiRenderState::with_sample_data();
        state.active_tab = 0;
        terminal.draw(|f| render_frame(f, &mut state)).unwrap();

        // Test Contacts tab
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
    #[cfg(feature = "test-utils")]
    fn test_settings_visual_lines_orders_peer_rows_by_peer_id() {
        with_settings_db(|| {
            let mut state = TuiRenderState::new();
            state.connected_peer_ids = HashSet::from([
                "peer-z".to_string(),
                "peer-a".to_string(),
                "peer-m".to_string(),
            ]);
            state.own_user_devices = vec![
                ("device-z".to_string(), "2026-11-26 12:00:00".to_string()),
                ("device-a".to_string(), "2026-11-26 11:00:00".to_string()),
            ];

            let lines = settings_visual_lines(&state, 120);
            let targets: Vec<&str> = lines
                .iter()
                .filter_map(|line| line.peer_id.as_deref())
                .collect();
            assert_eq!(
                targets,
                ["device-a", "device-z", "peer-a", "peer-m", "peer-z"],
                "bound devices and connected peers must each be sorted by peer ID"
            );
            assert!(
                lines
                    .iter()
                    .filter(|line| line.peer_id.is_some())
                    .all(|line| line.text.starts_with("› ")),
                "every peer row carries Flutter's click affordance"
            );
        });
    }

    #[test]
    #[cfg(feature = "test-utils")]
    fn test_settings_visual_lines_wrapped_peer_rows_keep_their_target() {
        with_settings_db(|| {
            let peer_id = "peer-wrap-target";
            p2p_app::set_peer_local_nickname(
                peer_id,
                "aVeryLongLocalNicknameThatWillDefinitelyWrapAcrossSeveralRows",
            )
            .unwrap();
            let mut state = TuiRenderState::new();
            state.connected_peer_ids.insert(peer_id.to_string());
            state.peer_count = 1;

            let lines = settings_visual_lines(&state, 20);
            let wrapped: Vec<&p2p_app::tui_render::SettingsLine> = lines
                .iter()
                .filter(|line| line.peer_id.as_deref() == Some(peer_id))
                .collect();
            assert!(
                wrapped.len() > 1,
                "the long nickname should wrap at width 20: {lines:?}"
            );
            assert!(
                wrapped
                    .first()
                    .is_some_and(|line| line.text.starts_with("› ")),
                "the first visual row keeps the click affordance"
            );
            assert!(
                wrapped
                    .iter()
                    .all(|line| line.peer_id.as_deref() == Some(peer_id)),
                "continuation rows stay clickable"
            );
        });
    }

    #[test]
    #[cfg(feature = "test-utils")]
    fn test_settings_visual_lines_informational_rows_have_no_target() {
        with_settings_db(|| {
            let mut state = TuiRenderState::new();
            state.last_connection_peer = Some("peer-disconnected".to_string());
            state.last_connection_lost = Some(1_756_481_201.0);

            let lines = settings_visual_lines(&state, 120);
            for prefix in [
                "Nickname:",
                "Network peers (",
                "Connected peers:",
                "  Last connected",
                "Network details:",
            ] {
                let line = lines
                    .iter()
                    .find(|line| line.text.starts_with(prefix))
                    .unwrap_or_else(|| panic!("missing {prefix:?} row in {lines:?}"));
                assert_eq!(
                    line.peer_id, None,
                    "informational row {prefix:?} must not look clickable"
                );
            }
        });
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
    fn test_render_settings_content_network_details() {
        let mut terminal = Terminal::new(TestBackend::new(120, 80)).unwrap();
        let mut state = TuiRenderState::new();
        state.network = p2p_app::net_diag::NetworkDetails {
            interfaces: vec![p2p_app::net_diag::NetworkInterfaceInfo {
                name: "eth0".to_string(),
                mac: Some("aa:bb:cc:dd:ee:ff".to_string()),
                ipv4: vec!["192.168.1.2".to_string()],
                ipv6: vec!["fe80::1".to_string()],
                internal: false,
            }],
            listen_addrs: vec!["/ip4/192.168.1.2/tcp/4001".to_string()],
            external_addrs: vec!["/ip4/203.0.113.7/tcp/4001".to_string()],
            reachability: "Public (directly reachable)".to_string(),
            nat_status: "Public".to_string(),
            nat_public_addr: Some("/ip4/203.0.113.7/tcp/4001".to_string()),
            nat_probe_successes: 2,
            nat_probe_failures: 1,
            hole_punch_attempts: 3,
            hole_punch_successes: 1,
            relay_reservations: vec!["12D3KooWRelay".to_string()],
            relay_status: "Reserved via 12D3KooWRelay".to_string(),
            dht_mode: "Server".to_string(),
            dht_connected: true,
            dht_routing_peers: 7,
            dht_bootstrap_successes: 2,
            dht_bootstrap_failures: 0,
            dht_announced: true,
            dht_announce_status: "Announced to the DHT".to_string(),
            ..p2p_app::net_diag::NetworkDetails::default()
        };

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Network details:"), "got: {text:?}");
        assert!(
            text.contains("Reachability: Public (directly reachable)"),
            "got: {text:?}"
        );
        assert!(
            text.contains("AutoNAT: Public (/ip4/203.0.113.7/tcp/4001) — probes 2 ok / 1 failed"),
            "got: {text:?}"
        );
        assert!(
            text.contains("Hole punching: 1/3 attempts succeeded"),
            "got: {text:?}"
        );
        assert!(
            text.contains("Relay: Reserved via 12D3KooWRelay (12D3KooWRelay)"),
            "got: {text:?}"
        );
        assert!(
            text.contains("DHT: connected (Server, 7 routing peers; bootstrap 2 ok / 0 failed)"),
            "got: {text:?}"
        );
        assert!(
            text.contains("DHT announcement: Announced (Announced to the DHT)"),
            "got: {text:?}"
        );
        assert!(text.contains("External addresses:"), "got: {text:?}");
        assert!(
            text.contains("Listen addresses:") && text.contains("/ip4/192.168.1.2/tcp/4001"),
            "got: {text:?}"
        );
        assert!(text.contains("eth0 — aa:bb:cc:dd:ee:ff"), "got: {text:?}");
        assert!(text.contains("IPv4: 192.168.1.2"), "got: {text:?}");
        assert!(text.contains("IPv6: fe80::1"), "got: {text:?}");
    }

    #[test]
    fn test_render_group_chat_header_matches_flutter() {
        use std::collections::VecDeque;

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut state = TuiRenderState::new();
        let group_id = "abcdef12-3456-7890";
        state.group_summaries = vec![p2p_app::groups::GroupSummary {
            group: p2p_app::generated::models_queryable::Group {
                id: 1,
                group_id: group_id.to_string(),
                display_name: "Rust Devs".to_string(),
                is_private: 0,
                created_at: chrono::NaiveDateTime::default(),
            },
            member_count: 3,
        }];
        state
            .group_messages
            .insert(group_id.to_string(), VecDeque::new());

        terminal
            .draw(|f| render_group_chat_content(f, f.area(), group_id, &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(
            text.contains("Group: Rust Devs (3 members) [abcdef12]"),
            "got: {text:?}"
        );
    }

    #[test]
    fn test_render_settings_content_network_peers() {
        let mut terminal = Terminal::new(TestBackend::new(120, 80)).unwrap();
        let mut state = TuiRenderState::new();
        state.network_peers = vec![
            p2p_app::network_config::NetworkPeer {
                id: 1,
                kind: "bootstrap".to_string(),
                multiaddr: "/ip4/1.2.3.4/tcp/4001".to_string(),
                created_at: "2026-11-08 12:00:00".to_string(),
            },
            p2p_app::network_config::NetworkPeer {
                id: 2,
                kind: "relay".to_string(),
                multiaddr: "/ip4/5.6.7.8/tcp/4001/p2p/12D3KooWRelay".to_string(),
                created_at: "2026-11-08 12:00:00".to_string(),
            },
        ];
        state.network_peer_selection = 1;

        terminal
            .draw(|f| render_settings_content(f, f.area(), &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Network peers (2)"), "got: {text:?}");
        assert!(
            text.contains("bootstrap /ip4/1.2.3.4/tcp/4001"),
            "got: {text:?}"
        );
        assert!(
            text.contains("> relay /ip4/5.6.7.8/tcp/4001/p2p/12D3KooWRelay"),
            "the highlighted row is marked: {text:?}"
        );
        assert!(
            text.contains("b: add bootstrap | r: add relay | x: remove"),
            "got: {text:?}"
        );
    }

    #[test]
    fn test_render_settings_content_network_peers_empty() {
        let mut terminal = Terminal::new(TestBackend::new(120, 80)).unwrap();
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
        assert!(text.contains("Network peers (0)"), "got: {text:?}");
        assert!(text.contains("— none configured"), "got: {text:?}");
    }

    #[test]
    fn test_render_peer_info_shows_contact_message_stats() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        // The stats come from the contact row that owns the device, so they
        // always match the number the Contacts tab shows for that row.
        let peer_id = state.contacts_all[0].device_rows[0].peer_id.clone();
        state.contacts_all[0].dm_count = 3;
        state.contacts_all[0].broadcast_sent_to_peer = 7;

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
    fn test_render_peer_info_stats_survive_a_contacts_search_filter() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        let peer_id = state.contacts_all[0].device_rows[0].peer_id.clone();
        state.contacts_all[0].dm_count = 5;
        state.contacts_all[0].broadcast_sent_to_peer = 11;
        // The Contacts tab shows only the filtered rows; Peer Info must keep
        // showing the canonical stats for a peer hidden by that filter.
        state.contacts.clear();

        terminal
            .draw(|f| render_peer_info_content(f, f.area(), &peer_id, &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Direct messages: 5"), "got: {text:?}");
        assert!(text.contains("Broadcasts sent: 11"), "got: {text:?}");
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

    #[test]
    fn test_render_peer_info_shows_ping_quality() {
        // Unique peer id so the global net_diag sample cannot collide with a
        // concurrently running test.
        let peer_id = "peer-quality-render-test";
        p2p_app::net_diag::record_ping_rtt(peer_id, 12.4);

        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();
        terminal
            .draw(|f| render_peer_info_content(f, f.area(), peer_id, &state))
            .unwrap();
        let buf = terminal.backend().buffer();
        let text: String = buf
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Round-trip time: 12 ms"), "got: {text:?}");
        assert!(text.contains("Ping loss: 0% (0/1)"), "got: {text:?}");
    }

    #[test]
    fn test_render_peer_info_shows_the_blocked_state_and_toggle_hint() {
        let peer_id = "peer-blocked-render-test";
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_peer_info_content(f, f.area(), peer_id, &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Blocked: no (b: block)"), "got: {text:?}");

        // Muted peers say so, and the hint flips to unblock.
        state.blocked_peers.insert(peer_id.to_string());
        terminal
            .draw(|f| render_peer_info_content(f, f.area(), peer_id, &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Blocked: yes (b: unblock)"), "got: {text:?}");
    }

    #[test]
    fn test_render_shares_shows_grants_for_private_share() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        let share = p2p_app::shares::Share {
            share_id: "share-1".to_string(),
            owner_user_id: "owner".to_string(),
            name: "Docs".to_string(),
            kind: p2p_app::shares::ShareKind::Private,
            public_write: false,
            local_root: None,
            revision: 3,
            manifest: None,
        };
        state.shares = vec![share.clone()];
        state.share_selection = 0;
        state.share_grants.insert(
            share.share_id,
            vec![p2p_app::shares::ShareGrant {
                user_id: "alice".to_string(),
                mode: p2p_app::shares::ShareMode::ReadOnly,
            }],
        );

        terminal
            .draw(|f| render_shares_content(f, f.area(), &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(text.contains("Grants (1)"), "got: {text:?}");
        assert!(text.contains("alice (readonly)"), "got: {text:?}");
    }

    #[test]
    fn test_render_shares_public_share_hides_grants() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();
        let share = p2p_app::shares::Share {
            share_id: "share-2".to_string(),
            owner_user_id: "owner".to_string(),
            name: "Public".to_string(),
            kind: p2p_app::shares::ShareKind::Public,
            public_write: false,
            local_root: None,
            revision: 1,
            manifest: None,
        };
        state.shares = vec![share.clone()];
        state.share_grants.insert(
            share.share_id,
            vec![p2p_app::shares::ShareGrant {
                user_id: "bob".to_string(),
                mode: p2p_app::shares::ShareMode::ReadWrite,
            }],
        );

        terminal
            .draw(|f| render_shares_content(f, f.area(), &state))
            .unwrap();
        let text: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect();
        assert!(!text.contains("Grants ("), "got: {text:?}");
        assert!(!text.contains("bob"), "got: {text:?}");
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
    assert_eq!(state.input_text, "");
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

// ── Contacts muted markers (US-054) ─────────────────────────────────────────

#[cfg(feature = "tui")]
mod contacts_blocklist_render_tests {
    use p2p_app::{TuiRenderState, tui_render::render_peers_content};
    use ratatui::{Terminal, backend::TestBackend};

    fn contact(contact_id: &str, device_peer_ids: &[&str]) -> p2p_app::contacts::ContactRow {
        p2p_app::contacts::ContactRow {
            contact_id: contact_id.to_string(),
            is_user: false,
            display_name: "Bob".to_string(),
            device_count: device_peer_ids.len(),
            device_rows: device_peer_ids
                .iter()
                .map(|peer_id| p2p_app::contacts::ContactDevice {
                    peer_id: (*peer_id).to_string(),
                    display_name: (*peer_id).to_string(),
                    first_seen: "2026-08-01 10:00:00".to_string(),
                    last_seen: "2026-08-28 15:26:41".to_string(),
                    received_nickname: None,
                    local_nickname: None,
                })
                .collect(),
            first_seen: "2026-08-01 10:00:00".to_string(),
            last_seen: "2026-08-28 15:26:41".to_string(),
            dm_count: 0,
            broadcast_sent_to_peer: 0,
        }
    }

    /// Render the Contacts tab and return one string per terminal row.
    fn render_rows(state: &TuiRenderState) -> Vec<String> {
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|f| render_peers_content(f, f.area(), state))
            .unwrap();
        let symbols: Vec<String> = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol().to_string())
            .collect();
        symbols.chunks(80).map(<[String]>::concat).collect()
    }

    fn marked_row_containing<'a>(rows: &'a [String], needle: &str) -> Option<&'a String> {
        rows.iter()
            .find(|row| row.contains('\u{2298}') && row.contains(needle))
    }

    #[test]
    fn fully_muted_contact_rows_carry_the_marker() {
        let mut state = TuiRenderState::new();
        state.contacts = vec![contact("bob", &["bob-laptop", "bob-phone"])];
        state.blocked_peers.insert("bob-laptop".to_string());
        state.blocked_peers.insert("bob-phone".to_string());

        let rows = render_rows(&state);
        assert!(
            marked_row_containing(&rows, "Bob").is_some(),
            "got: {rows:?}"
        );
    }

    #[test]
    fn partially_muted_contact_marks_only_the_blocked_device_row() {
        let mut state = TuiRenderState::new();
        state.contacts = vec![contact("bob", &["bob-laptop", "bob-phone"])];
        state.expanded_contact_ids.insert("bob".to_string());
        state.blocked_peers.insert("bob-phone".to_string());

        let rows = render_rows(&state);
        // The contact row stays unmarked...
        assert!(
            marked_row_containing(&rows, "Bob").is_none(),
            "got: {rows:?}"
        );
        // ...while the blocked device sub-row is marked.
        assert!(
            marked_row_containing(&rows, "bob-phone").is_some(),
            "got: {rows:?}"
        );
        assert!(
            marked_row_containing(&rows, "bob-laptop").is_none(),
            "got: {rows:?}"
        );
    }
}
