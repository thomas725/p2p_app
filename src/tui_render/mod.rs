//! TUI rendering functions for both binary and tests.

mod chat;
mod log;
mod peer_info;
mod peers;
mod settings;

pub use chat::{render_chat_content, render_dm_content};
pub use log::render_log_content;
pub use peer_info::render_peer_info_content;
pub use peers::render_peers_content;
pub use settings::render_settings_content;

use crate::fmt::short_peer_id;
use crate::tui_render_state::{TuiRenderState, get_tab_content};
use crate::tui_tabs::TabContent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph, Tabs, Wrap},
};

/// Render a full TUI frame
pub fn render_frame(f: &mut ratatui::Frame, state: &mut TuiRenderState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    let default_rect = Rect::default();
    let tab_area = chunks.first().unwrap_or(&default_rect);
    let content_area = chunks.get(1).unwrap_or(&default_rect);
    let input_area = chunks.get(2).unwrap_or(&default_rect);
    let shortcut_area = chunks.get(3).unwrap_or(&default_rect);
    let status_area = chunks.get(4).unwrap_or(&default_rect);

    render_tabs(f, *tab_area, state);

    let tab_content = get_tab_content(state);
    render_tab_content(f, *content_area, &tab_content, state);

    render_input_section(f, *input_area, state, &tab_content);
    render_shortcuts(f, *shortcut_area, &tab_content, state.kitty_keyboard_active);
    render_status_bar(f, *status_area, state);

    if let Some(ref text) = state.popup {
        render_popup(f, text.clone());
    }
}

/// Render the tab bar
pub fn render_tabs(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let titles: Vec<&str> = state
        .tab_titles
        .iter()
        .map(std::string::String::as_str)
        .collect();
    let tabs = Tabs::new(titles)
        .style(Style::default().fg(Color::Cyan))
        .select(state.active_tab)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(tabs, area);
}

/// Render tab content
pub fn render_tab_content(
    f: &mut ratatui::Frame,
    area: Rect,
    tab_content: &TabContent,
    state: &mut TuiRenderState,
) {
    match tab_content {
        TabContent::Chat => render_chat_content(f, area, state),
        TabContent::Peers => render_peers_content(f, area, state),
        TabContent::Direct(peer_id) => render_dm_content(f, area, peer_id, state),
        TabContent::Log => render_log_content(f, area, state),
        TabContent::Settings => render_settings_content(f, area, state),
        TabContent::PeerInfo(peer_id) => render_peer_info_content(f, area, peer_id, state),
    }
}

/// Shared bordered list/paragraph block with the given title.
pub(crate) fn bordered_block(title: String) -> Block<'static> {
    Block::default().title(title).borders(Borders::ALL)
}

/// Render input section
pub fn render_input_section(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &TuiRenderState,
    tab_content: &TabContent,
) {
    let title = if state.editing_nickname {
        format!("Edit Nickname ({})", short_peer_id(&state.nickname_peer_id))
    } else {
        "Input".to_string()
    };

    let input_block = Block::default().title(title).borders(Borders::ALL);

    if tab_content.is_input_enabled() || state.editing_nickname {
        let inner_area = input_block.inner(area);
        f.render_widget(input_block, area);
        let input = Paragraph::new(state.input_text.as_str());
        f.render_widget(input, inner_area);
    } else {
        f.render_widget(input_block, area);
    }
}

/// Shortcut hint line for the given tab. Peer-Info bindings are documented only
/// on tabs where they are active: Ctrl+I (kitty) and Ctrl+P (universal) on a
/// Direct/DM tab, `i` on Peers.
#[must_use]
pub const fn shortcuts_text(tab_content: &TabContent, kitty: bool) -> &'static str {
    match tab_content {
        TabContent::Peers => {
            "Tab: next | Up/Down: select | Enter: open DM | i: Peer Info | F12: mouse | Ctrl+Q: quit"
        }
        TabContent::Direct(_) if kitty => {
            "Tab: next | Ctrl+I/Ctrl+P: Peer Info | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit"
        }
        TabContent::Direct(_) => {
            "Tab: next | Ctrl+P: Peer Info | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit"
        }
        _ => {
            "Tab: next | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit"
        }
    }
}

/// Render shortcuts help, tailored to the active tab
pub fn render_shortcuts(f: &mut ratatui::Frame, area: Rect, tab_content: &TabContent, kitty: bool) {
    let shortcuts = Paragraph::new(shortcuts_text(tab_content, kitty));
    f.render_widget(shortcuts, area);
}

/// Render status bar
pub fn render_status_bar(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let mouse = if state.mouse_capture { "ON" } else { "OFF" };
    let conn = if state.connected {
        "Connected"
    } else {
        "Disconnected"
    };
    let status = Paragraph::new(format!("{conn} [Mouse: {mouse}]"));
    f.render_widget(status, area);
}

/// Render popup
#[allow(
    clippy::as_conversions,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::arithmetic_side_effects
)]
pub fn render_popup(f: &mut ratatui::Frame, text: String) {
    let area = f.area();
    let w = (f32::from(area.width) * 0.70) as u16;
    let h = (f32::from(area.height) * 0.40) as u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w.max(20).min(area.width),
        height: h.max(6).min(area.height),
    };

    f.render_widget(Clear, popup);
    let p = Paragraph::new(text)
        .block(
            Block::default()
                .title("Details")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}
