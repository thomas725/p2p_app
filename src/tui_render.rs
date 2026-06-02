//! TUI rendering functions for both binary and tests

use crate::fmt::short_peer_id;
use crate::tui_render_state::{
    TuiRenderState, broadcast_receipt_prefix, calc_visible_strings, dm_receipt_prefix,
    get_tab_content,
};
use crate::tui_tabs::TabContent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
};
use std::collections::VecDeque;

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

/// Render peer info
pub fn render_peer_info(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let peer_info = Paragraph::new(format!("Peers: {}", state.peer_count));
    f.render_widget(peer_info, area);
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
    }
}

/// Render chat messages with scroll support and receipt markers
pub fn render_chat_content(f: &mut ratatui::Frame, area: Rect, state: &mut TuiRenderState) {
    let text_width = area.width.saturating_sub(4) as usize;
    let usable_height = area.height.saturating_sub(2) as usize;

    let (visible, effective_offset) = calc_visible_strings(
        &state.messages,
        state.chat_auto_scroll,
        state.chat_scroll_offset,
        text_width,
        usable_height,
    );

    let visible_messages: Vec<ListItem> = state
        .messages
        .iter()
        .skip(effective_offset)
        .enumerate()
        .take(visible)
        .map(|(visible_idx, msg)| {
            let global_idx = effective_offset + visible_idx;
            let is_selected = state.broadcast_selection == Some(global_idx);
            let msg_id = state
                .message_ids
                .get(global_idx)
                .and_then(|id| id.as_deref());
            let prefix = broadcast_receipt_prefix(msg_id, &state.broadcast_receipts);
            let display = format!("{prefix}{msg}");
            if is_selected {
                ListItem::new(display).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(display)
            }
        })
        .collect();

    let messages_list = List::new(visible_messages).block(
        Block::default()
            .title("Broadcast Chat")
            .borders(Borders::ALL),
    );
    f.render_widget(messages_list, area);
}

/// Render peer list with selection support
pub fn render_peers_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let peer_items: Vec<ListItem> = state
        .peers
        .iter()
        .enumerate()
        .map(|(idx, (id, name, status))| {
            let line = format!("{} ({}) - {}", name, short_peer_id(id), status);
            if idx == state.peer_selection {
                ListItem::new(line).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(line)
            }
        })
        .collect();
    let peers_list = List::new(peer_items).block(
        Block::default()
            .title("Connected Peers")
            .borders(Borders::ALL),
    );
    f.render_widget(peers_list, area);
}

/// Render DM conversation with split view (broadcast messages on top, DM on bottom)
pub fn render_dm_content(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    state: &mut TuiRenderState,
) {
    let text_width = area.width.saturating_sub(4) as usize;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let broadcast_area = chunks[0];
    let dm_area = chunks[1];

    let broadcast_usable_height = broadcast_area.height.saturating_sub(2) as usize;
    let dm_usable_height = dm_area.height.saturating_sub(2) as usize;

    let short_id = short_peer_id(peer_id);

    let broadcast_messages: VecDeque<String> = state
        .messages
        .iter()
        .zip(state.message_peer_ids.iter())
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .map(|(msg, _)| msg.clone())
        .collect();

    if broadcast_messages.is_empty() {
        let broadcast_para = Paragraph::new("No broadcast messages").block(
            Block::default()
                .title(format!("Broadcast from {short_id}"))
                .borders(Borders::ALL),
        );
        f.render_widget(broadcast_para, broadcast_area);
    } else {
        let (broadcast_scroll_offset, broadcast_auto_scroll) = {
            let (offset, auto_scroll) = state
                .dm_broadcast_scroll_state
                .entry(peer_id.to_string())
                .or_insert((broadcast_messages.len(), true));
            (*offset, *auto_scroll)
        };

        let (visible, effective_offset) = calc_visible_strings(
            &broadcast_messages,
            broadcast_auto_scroll,
            broadcast_scroll_offset,
            text_width,
            broadcast_usable_height,
        );

        let visible_broadcast: Vec<ListItem> = broadcast_messages
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let broadcast_list = List::new(visible_broadcast).block(
            Block::default()
                .title(format!("Broadcast from {short_id}"))
                .borders(Borders::ALL),
        );
        f.render_widget(broadcast_list, broadcast_area);
    }

    let (scroll_offset_val, auto_scroll_val) = {
        let (offset, auto_scroll) = state
            .dm_scroll_state
            .entry(peer_id.to_string())
            .or_insert((0, true));
        (*offset, *auto_scroll)
    };

    if let Some(msgs) = state.dm_messages.get(peer_id) {
        let (visible, effective_offset) = calc_visible_strings(
            msgs,
            auto_scroll_val,
            scroll_offset_val,
            text_width,
            dm_usable_height,
        );

        let visible_msgs: Vec<ListItem> = msgs
            .iter()
            .skip(effective_offset)
            .take(visible)
            .enumerate()
            .map(|(visible_idx, m)| {
                let global_idx = effective_offset + visible_idx;
                let msg_id = state
                    .dm_message_ids
                    .get(peer_id)
                    .and_then(|ids| ids.get(global_idx))
                    .and_then(|id| id.as_deref());
                let prefix = dm_receipt_prefix(msg_id, &state.dm_receipts);
                ListItem::new(format!("{prefix}{m}"))
            })
            .collect();

        let dm_list = List::new(visible_msgs).block(
            Block::default()
                .title(format!("DM: {short_id}"))
                .borders(Borders::ALL),
        );
        f.render_widget(dm_list, dm_area);
    } else {
        let dm_para = Paragraph::new("No direct messages").block(
            Block::default()
                .title(format!("DM: {short_id}"))
                .borders(Borders::ALL),
        );
        f.render_widget(dm_para, dm_area);
    }
}

/// Render log content
pub fn render_log_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let text_width = area.width.saturating_sub(4) as usize;
    let usable_height = area.height.saturating_sub(2) as usize;

    let (visible, effective_offset) = calc_visible_strings(
        &state.log_messages,
        state.log_auto_scroll,
        state.log_scroll_offset,
        text_width,
        usable_height,
    );

    let visible_logs: Vec<ListItem> = state
        .log_messages
        .iter()
        .skip(effective_offset)
        .take(visible)
        .map(|line| ListItem::new(line.as_str()))
        .collect();

    let log_list =
        List::new(visible_logs).block(Block::default().title("Logs").borders(Borders::ALL));
    f.render_widget(log_list, area);
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
    let status = Paragraph::new(format!("{conn} [Mouse: {mouse}]"));
    f.render_widget(status, area);
}

/// Render popup
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
