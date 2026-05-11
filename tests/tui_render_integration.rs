//! Integration tests for TUI rendering using ratatui TestBackend
//! These tests verify the TUI rendering logic using the library module

#[cfg(feature = "tui")]
mod render_tests {
    use p2p_app::tui_render_state::get_tab_content;
    use p2p_app::{
        TuiRenderState,
        tui_render::{
            render_chat_content, render_dm_content, render_frame, render_input_section,
            render_log_content, render_peer_info, render_peers_content, render_popup,
            render_shortcuts, render_status_bar, render_tabs,
        },
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
    fn test_render_peer_info_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_peer_info(f, f.area(), &state))
            .unwrap();
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
    fn test_render_peers_content_library() {
        let mut terminal = create_test_terminal();
        let state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_peers_content(f, f.area(), &state))
            .unwrap();
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

    #[test]
    fn test_render_shortcuts_library() {
        let mut terminal = create_test_terminal();

        terminal.draw(|f| render_shortcuts(f, f.area())).unwrap();
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
    assert_eq!(state.peers[0].0, "peer-1");
    assert_eq!(state.peers[0].1, "Alice");
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
    state.active_tab = 1; // "Peers"
    assert!(matches!(get_tab_content(&state), TabContent::Peers));
}

#[test]
fn test_get_tab_content_log() {
    use p2p_app::tui_render_state::{TuiRenderState, get_tab_content};
    use p2p_app::tui_tabs::TabContent;
    let mut state = TuiRenderState::new();
    state.active_tab = 2; // "Log"
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
        other => panic!("expected Direct, got {:?}", other),
    }
}

#[test]
fn test_tui_tab_content_is_input_enabled() {
    use p2p_app::tui_tabs::TabContent;
    assert!(TabContent::Chat.is_input_enabled());
    assert!(TabContent::Direct("p".into()).is_input_enabled());
    assert!(!TabContent::Peers.is_input_enabled());
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
