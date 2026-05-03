//! TUI rendering functions for both binary and tests

use crate::tui_render_state::{TuiRenderState, TuiTabContent, get_tab_content};
use ratatui::{
    layout::Alignment,
    layout::Constraint,
    layout::Direction,
    layout::Layout,
    layout::Rect,
    style::Color,
    style::Modifier,
    style::Style,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};

/// Render a full TUI frame
pub fn render_frame(f: &mut ratatui::Frame, state: &mut TuiRenderState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(f.area());

    render_tabs(f, chunks[0], state);
    render_peer_info(f, chunks[1], state);

    let tab_content = get_tab_content(state);
    render_tab_content(f, chunks[2], &tab_content, state);

    render_input_section(f, chunks[3], state, &tab_content);
    render_shortcuts(f, chunks[4]);
    render_status_bar(f, chunks[5], state);

    if let Some(ref text) = state.popup {
        render_popup(f, text.clone());
    }
}

/// Render the tab bar
pub fn render_tabs(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let titles: Vec<&str> = state.tab_titles.iter().map(|s| s.as_str()).collect();
    let tabs = Tabs::new(titles)
        .style(Style::default().fg(Color::Cyan))
        .select(state.active_tab)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    f.render_widget(tabs, area);
}

/// Render peer info
pub fn render_peer_info(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let peer_info = Paragraph::new(format!("Peers: {}", state.peer_count));
    f.render_widget(peer_info, area);
}

/// Render tab content
pub fn render_tab_content(
    f: &mut ratatui::Frame,
    area: Rect,
    tab_content: &TuiTabContent,
    state: &TuiRenderState,
) {
    match tab_content {
        TuiTabContent::Chat => render_chat_content(f, area, state),
        TuiTabContent::Peers => render_peers_content(f, area, state),
        TuiTabContent::Direct(peer_id) => render_dm_content(f, area, peer_id, state),
        TuiTabContent::Log => render_log_content(f, area, state),
    }
}

/// Render chat messages
pub fn render_chat_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let items: Vec<ListItem> = state
        .messages
        .iter()
        .map(|m| ListItem::new(m.as_str()))
        .collect();
    let list = List::new(items);
    f.render_widget(list, area);
}

/// Render peer list
pub fn render_peers_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let items: Vec<ListItem> = state
        .peers
        .iter()
        .map(|(id, name, status)| {
            ListItem::new(format!("{} ({}) - {}", name, short_id(id), status))
        })
        .collect();
    let list = List::new(items);
    f.render_widget(list, area);
}

/// Render DM conversation
pub fn render_dm_content(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    state: &TuiRenderState,
) {
    let messages = state.dm_messages.get(peer_id).cloned().unwrap_or_default();
    let block = Block::default()
        .title(format!("DM: {}", peer_id))
        .borders(Borders::ALL);
    let items: Vec<ListItem> = messages.iter().map(|m| ListItem::new(m.as_str())).collect();
    let list = List::new(items);
    f.render_widget(list.block(block), area);
}

/// Render log content
pub fn render_log_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let log_text = "[12:00] Connected to test-net\n[12:00] Discovered 2 peers";
    let log = Paragraph::new(log_text).block(Block::default().title("Logs").borders(Borders::ALL));
    f.render_widget(log, area);
}

/// Render input section
pub fn render_input_section(
    f: &mut ratatui::Frame,
    area: Rect,
    state: &TuiRenderState,
    tab_content: &TuiTabContent,
) {
    let title = if state.editing_nickname {
        format!("Edit Nickname ({})", short_id(&state.nickname_peer_id))
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

/// Render shortcuts help
pub fn render_shortcuts(f: &mut ratatui::Frame, area: Rect) {
    let shortcuts = Paragraph::new("Tab: next | PgUp/PgDn: scroll | Enter: send | Esc: clear");
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
    let status = Paragraph::new(format!("{} [Mouse: {}]", conn, mouse));
    f.render_widget(status, area);
}

/// Render popup
pub fn render_popup(f: &mut ratatui::Frame, text: String) {
    let area = f.area();
    let w = (area.width as f32 * 0.70) as u16;
    let h = (area.height as f32 * 0.40) as u16;
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

/// Get short peer ID (last 4 chars)
fn short_id(id: &str) -> String {
    let len = id.len();
    if len > 4 {
        id[len - 4..].to_string()
    } else {
        id.to_string()
    }
}
