use crate::tui::state::AppState;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Paragraph},
};

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

/// Shortcut hint line for the given tab. Peer-Info bindings are documented only
/// on tabs where they are active: Ctrl+I (kitty) or Ctrl+? (non-kitty) on a
/// Direct/DM tab, `i` on Peers.
const fn shortcuts_text(tab_content: &TabContent, kitty: bool) -> &'static str {
    match tab_content {
        TabContent::Peers => {
            "Tab: next | Up/Down: select | Enter: open DM | i: Peer Info | F12: mouse | Ctrl+Q: quit"
        }
        TabContent::Direct(_) if kitty => {
            "Tab: next | Ctrl+I: Peer Info | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit"
        }
        TabContent::Direct(_) => {
            "Tab: next | Ctrl+?: Peer Info | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit"
        }
        _ => "Tab: next | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit",
    }
}

/// Render the help text shortcuts
pub fn render_shortcuts(
    f: &mut Frame,
    shortcuts_area: Rect,
    tab_content: &TabContent,
    kitty: bool,
) {
    f.render_widget(Paragraph::new(shortcuts_text(tab_content, kitty)), shortcuts_area);
}

/// Render the status bar with connection and mouse mode info
pub fn render_status_bar(f: &mut Frame, status_area: Rect, state: &AppState) {
    let mouse_mode = if state.mouse_capture { "ON" } else { "OFF" };
    let connected = state.concurrent_peers > 0 || state.local_peer_id != "unknown";
    let conn = if connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status = Paragraph::new(format!("{conn} [Mouse: {mouse_mode}]"));
    f.render_widget(status, status_area);
}

#[cfg(test)]
#[path = "../../../../tests/unit/unit_bin_tui_render_loop_layout.rs"]
mod tests;
