use crate::tui::state::AppState;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};

/// Render the tab navigation bar
pub fn render_tabs(f: &mut Frame, tab_area: Rect, state: &AppState) {
    let tab_titles = state.dynamic_tabs.all_titles();
    let tabs = ratatui::widgets::Tabs::new(tab_titles)
        .style(Style::default().fg(Color::Cyan))
        .select(state.active_tab);
    f.render_widget(tabs, tab_area);
}

/// Render the peer count display
pub fn render_peer_info(f: &mut Frame, peer_area: Rect, state: &AppState) {
    let peer_info = Paragraph::new(format!("Peers: {}", state.concurrent_peers));
    f.render_widget(peer_info, peer_area);
}

/// Render the input section with optional nickname editing
pub fn render_input_section(
    f: &mut Frame,
    input_area: Rect,
    state: &AppState,
    tab_content: &TabContent,
) {
    let title = if state.editing_nickname {
        format!(
            "Edit Nickname ({}) - Enter to save, Esc to cancel",
            p2p_app::short_peer_id(&state.local_peer_id)
        )
    } else {
        "Input".to_string()
    };
    let input_block = Block::default().title(title).borders(Borders::ALL);
    if tab_content.is_input_enabled() || state.editing_nickname {
        let inner_area = input_block.inner(input_area);
        f.render_widget(input_block, input_area);
        let mut textarea = state.chat_input.clone();
        textarea.set_cursor_line_style(Style::default());
        f.render_widget(&textarea, inner_area);
    } else {
        f.render_widget(input_block, input_area);
    }
}

/// Render the help text shortcuts
pub fn render_shortcuts(f: &mut Frame, shortcuts_area: Rect) {
    let shortcuts = Paragraph::new(
        "Tab: next tab | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit",
    );
    f.render_widget(shortcuts, shortcuts_area);
}

/// Render the status bar with connection and mouse mode info
pub fn render_status_bar(f: &mut Frame, status_area: Rect, state: &AppState) {
    let mouse_mode = if state.mouse_capture { "ON" } else { "OFF" };
    let status = Paragraph::new(format!("Connected [Mouse: {}]", mouse_mode));
    f.render_widget(status, status_area);
}
