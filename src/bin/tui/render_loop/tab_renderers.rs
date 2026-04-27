use super::visibility::{calc_visible_tuples, calc_visible_strings, count_lines};
use crate::tui::state::AppState;
use std::collections::VecDeque;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, ListItem, List, Paragraph},
    Frame,
};
use p2p_app::get_tui_logs;

/// Render the broadcast chat tab with message selection
pub fn render_chat_tab(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    text_width: usize,
    usable_height: usize,
) {
    state.chat_area_height = area.height as usize;
    let (visible, effective_offset) = calc_visible_tuples(
        &state.messages,
        state.chat_auto_scroll,
        state.chat_scroll_offset,
        text_width,
        usable_height,
    );
    state.visible_message_count = visible;
    state.chat_message_offset = effective_offset;

    let visible_messages: Vec<ListItem> = state.messages
        .iter()
        .skip(effective_offset)
        .enumerate()
        .take(visible)
        .map(|(visible_idx, (msg, _))| {
            let global_idx = effective_offset + visible_idx;
            let is_selected = state.broadcast_selection == Some(global_idx);
            if is_selected {
                ListItem::new(msg.as_str()).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(msg.as_str())
            }
        })
        .collect();

    state.chat_message_lines = state.messages
        .iter()
        .skip(effective_offset)
        .take(visible)
        .map(|(msg, _)| count_lines(msg, text_width))
        .collect();

    let messages_list = List::new(visible_messages)
        .block(Block::default().title("Broadcast Chat").borders(Borders::ALL));
    frame.render_widget(messages_list, area);
}

/// Render the peers list with selection
pub fn render_peers_tab(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
) {
    let peer_items: Vec<ListItem> = state.peers
        .iter()
        .enumerate()
        .map(|(idx, (id, _first_seen, last_seen))| {
            let line = format!("{} ({})", id, last_seen);
            if idx == state.peer_selection {
                ListItem::new(line).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(line)
            }
        })
        .collect();
    let peers_list = List::new(peer_items)
        .block(Block::default().title("Connected Peers").borders(Borders::ALL));
    frame.render_widget(peers_list, area);
}

/// Render the split DM tab (broadcast messages on top, direct messages on bottom)
pub fn render_dm_tab(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    peer_id: &str,
    text_width: usize,
    _usable_height: usize,
) {
    let short_id = if peer_id.len() <= 8 {
        peer_id.to_string()
    } else {
        peer_id[peer_id.len()-8..].to_string()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let broadcast_area = chunks[0];
    let dm_area = chunks[1];

    let broadcast_usable_height = broadcast_area.height.saturating_sub(2) as usize;
    let dm_usable_height = dm_area.height.saturating_sub(2) as usize;

    let broadcast_messages: Vec<(String, Option<String>)> = state.messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .cloned()
        .collect();

    if !broadcast_messages.is_empty() {
        let broadcast_strings: VecDeque<String> = broadcast_messages
            .iter()
            .map(|(msg, _)| msg.clone())
            .collect();

        let (broadcast_scroll_offset, broadcast_auto_scroll) = {
            let (offset, auto_scroll) = state.dm_broadcast_scroll_state
                .entry(peer_id.to_string())
                .or_insert((broadcast_messages.len(), true));
            (*offset, *auto_scroll)
        };

        let (visible, effective_offset) = calc_visible_strings(
            &broadcast_strings,
            broadcast_auto_scroll,
            broadcast_scroll_offset,
            text_width,
            broadcast_usable_height,
        );
        let (_, dm_visible) = state.dm_visible_counts.get(peer_id).copied().unwrap_or((0, 0));
        state.dm_visible_counts.insert(peer_id.to_string(), (visible, dm_visible));
        state.dm_broadcast_offset.insert(peer_id.to_string(), effective_offset);

        let broadcast_line_counts: Vec<usize> = broadcast_strings
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|msg| count_lines(msg, text_width))
            .collect();
        state.dm_broadcast_message_lines.insert(peer_id.to_string(), broadcast_line_counts);

        let visible_broadcast: Vec<ListItem> = broadcast_strings
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();
        let broadcast_list = List::new(visible_broadcast)
            .block(Block::default().title(format!("Broadcast from {}", short_id)).borders(Borders::ALL));
        frame.render_widget(broadcast_list, broadcast_area);
    } else {
        let broadcast_para = Paragraph::new("No broadcast messages")
            .block(Block::default().title(format!("Broadcast from {}", short_id)).borders(Borders::ALL));
        frame.render_widget(broadcast_para, broadcast_area);
    }

    let (scroll_offset_val, auto_scroll_val) = {
        let (offset, auto_scroll) = state.dm_scroll_state
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
        let (broadcast_visible, _) = state.dm_visible_counts.get(peer_id).copied().unwrap_or((0, 0));
        state.dm_visible_counts.insert(peer_id.to_string(), (broadcast_visible, visible));

        let visible_msgs: Vec<ListItem> = msgs
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let dm_list = List::new(visible_msgs)
            .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
        frame.render_widget(dm_list, dm_area);
    } else {
        let dm_para = Paragraph::new("No direct messages")
            .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
        frame.render_widget(dm_para, dm_area);
    }
}

/// Render the log tab
pub fn render_log_tab(frame: &mut Frame, area: Rect) {
    let log_text = get_tui_logs().join("\n");
    let log_para = Paragraph::new(log_text)
        .block(Block::default().title("Logs").borders(Borders::ALL));
    frame.render_widget(log_para, area);
}
