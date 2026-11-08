use super::super::state::AppState;
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
    let title = if state.is_dialing_peer() {
        "Dial peer address - Enter to dial, Esc to cancel".to_string()
    } else if state.is_adding_network_peer() {
        format!(
            "Add {} multiaddr - Enter to save, Esc to cancel",
            state.network_peer_kind.map_or(
                "network peer",
                p2p_app::network_config::NetworkPeerKind::as_str
            )
        )
    } else if state.is_creating_group() {
        "Create or Join Group - Enter to confirm, Esc to cancel".to_string()
    } else if state.is_editing_nickname() {
        let (label, target) = if state.is_editing_local_nickname() {
            (
                "Local Nickname",
                state
                    .editing_nickname_peer
                    .clone()
                    .unwrap_or_else(|| state.local_peer_id.clone()),
            )
        } else {
            ("Edit Nickname", state.local_peer_id.clone())
        };
        format!(
            "{label} ({}) - Enter to save, Esc to cancel",
            p2p_app::short_peer_id(&target)
        )
    } else {
        "Input".to_string()
    };
    let input_block = Block::default().title(title).borders(Borders::ALL);
    if tab_content.is_input_enabled()
        || state.is_editing_nickname()
        || state.is_creating_group()
        || state.is_dialing_peer()
        || state.is_adding_network_peer()
    {
        let inner_area = input_block.inner(input_area);
        f.render_widget(input_block, input_area);
        let mut textarea = state.chat_input.clone();
        textarea.set_cursor_line_style(Style::default());
        f.render_widget(&textarea, inner_area);
    } else {
        f.render_widget(input_block, input_area);
    }
}

/// Render the status bar with connection and mouse mode info
pub fn render_status_bar(f: &mut Frame, status_area: Rect, state: &AppState) {
    let mouse_mode = if state.env.mouse_capture { "ON" } else { "OFF" };
    let connected = state.concurrent_peers > 0 || state.local_peer_id != "unknown";
    let conn = if connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status = Paragraph::new(format!("{conn} [Mouse: {mouse_mode}]"));
    f.render_widget(status, status_area);
}
