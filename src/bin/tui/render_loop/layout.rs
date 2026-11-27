use super::super::state::{AppState, ShareGrantAction};
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
    let title = if state.is_adding_reaction() {
        format!(
            "Reaction {} - number/Enter: toggle, Esc: cancel",
            p2p_app::reactions::quick_reaction_hint()
        )
    } else if state.is_editing_message() {
        "Edit sent message - Enter to save locally, Esc to cancel".to_string()
    } else if state.is_editing_share_grant() {
        match state
            .share_grant_prompt
            .as_ref()
            .map(|prompt| &prompt.action)
        {
            Some(ShareGrantAction::Add { mode }) => format!(
                "Add share grant ({}) - Tab: mode, Enter: save, Esc: cancel",
                p2p_app::shares::mode_label(*mode)
            ),
            Some(ShareGrantAction::Revoke { user_id }) => format!(
                "Revoke share grant ({}) - Enter to remove, Esc to cancel",
                p2p_app::short_peer_id(user_id)
            ),
            None => "Share grant - Enter to save, Esc to cancel".to_string(),
        }
    } else if state.is_dialing_peer() {
        "Dial peer address - Enter to dial, Esc to cancel".to_string()
    } else if state.is_adding_network_peer() {
        format!(
            "Add {} multiaddr - Enter to save, Esc to cancel",
            state.network_peer_kind.map_or(
                "network peer",
                p2p_app::network_config::NetworkPeerKind::as_str
            )
        )
    } else if state.is_exporting_location_track() {
        "Export location track (from;to) - Enter to export, Esc to cancel".to_string()
    } else if state.is_creating_group() {
        "Create or Join Group - Enter to confirm, Esc to cancel".to_string()
    } else if state.is_searching_contacts() {
        "Search Contacts - Enter to keep, Esc to clear".to_string()
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
    } else if let Some(reply) = state.reply_draft(tab_content) {
        format!("Reply: {}", p2p_app::reply::quoted_header(reply))
    } else {
        "Input".to_string()
    };
    let input_block = Block::default().title(title).borders(Borders::ALL);
    if tab_content.is_input_enabled() || state.is_modal_editor() || state.is_searching_contacts() {
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

#[cfg(test)]
#[path = "../../../../tests/unit/unit_bin_tui_render_loop_layout.rs"]
mod tests;
