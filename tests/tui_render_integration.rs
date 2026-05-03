//! Integration tests for TUI rendering using ratatui TestBackend
//! These tests verify the TUI rendering logic using the library module

#[cfg(feature = "tui")]
mod render_tests {
    use p2p_app::tui_render_state::{TuiTabContent, get_tab_content};
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
            .draw(|f| render_chat_content(f, f.area(), &state))
            .unwrap();
    }

    #[test]
    fn test_render_peers_content_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

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
                render_dm_content(f, inner, "Alice", &state);
            })
            .unwrap();
    }

    #[test]
    fn test_render_log_content_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

        terminal
            .draw(|f| render_log_content(f, f.area(), &state))
            .unwrap();
    }

    #[test]
    fn test_render_input_section_library() {
        let mut terminal = create_test_terminal();
        let mut state = TuiRenderState::with_sample_data();

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
