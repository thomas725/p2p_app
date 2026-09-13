//! Broadcast-chat and direct-message pane rendering.

use crate::fmt::short_peer_id;
use crate::tui_render::bordered_block;
use crate::tui_render_state::{
    TuiRenderState, broadcast_receipt_prefix, calc_visible_list_items, dm_receipt_prefix,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::collections::VecDeque;

/// Render chat messages with scroll support and receipt markers
pub fn render_chat_content(f: &mut ratatui::Frame, area: Rect, state: &mut TuiRenderState) {
    let usable_height = usize::from(area.height.saturating_sub(2));

    let (visible, effective_offset) = calc_visible_list_items(
        &state.messages,
        state.chat_auto_scroll,
        state.chat_scroll_offset,
        usable_height,
    );

    let visible_messages: Vec<ListItem> = state
        .messages
        .iter()
        .skip(effective_offset)
        .enumerate()
        .take(visible)
        .map(|(visible_idx, msg)| {
            let global_idx = effective_offset.saturating_add(visible_idx);
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

    if state.chat_unread_count > 0 {
        let n = state.chat_unread_count;
        let label = if n == 1 {
            "▼ 1 new message — End: jump to latest".to_string()
        } else {
            format!("▼ {n} new messages — End: jump to latest")
        };
        let banner_y = area.y.saturating_add(area.height.saturating_sub(2));
        let banner_area = Rect::new(
            area.x.saturating_add(1),
            banner_y,
            area.width.saturating_sub(2),
            1,
        );
        let banner = Paragraph::new(label).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
        f.render_widget(banner, banner_area);
    }
}

/// Render DM conversation with split view (broadcast messages on top, DM on bottom)
pub fn render_dm_content(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    state: &mut TuiRenderState,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let default_rect = Rect::default();
    let broadcast_area = *chunks.first().unwrap_or(&default_rect);
    let dm_area = *chunks.get(1).unwrap_or(&default_rect);

    let broadcast_usable_height = usize::from(broadcast_area.height.saturating_sub(2));
    let dm_usable_height = usize::from(dm_area.height.saturating_sub(2));

    let short_id = crate::get_peer_display_name(peer_id).unwrap_or_else(|_| short_peer_id(peer_id));

    let broadcast_messages: VecDeque<String> = state
        .messages
        .iter()
        .zip(state.message_peer_ids.iter())
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .map(|(msg, _)| msg.clone())
        .collect();

    if broadcast_messages.is_empty() {
        let broadcast_para = Paragraph::new("No broadcast messages")
            .block(bordered_block(format!("Broadcast from {short_id}")));
        f.render_widget(broadcast_para, broadcast_area);
    } else {
        let (broadcast_scroll_offset, broadcast_auto_scroll) = {
            let (offset, auto_scroll) = state
                .dm_broadcast_scroll_state
                .entry(peer_id.to_string())
                .or_insert((broadcast_messages.len(), true));
            (*offset, *auto_scroll)
        };

        let (visible, effective_offset) = calc_visible_list_items(
            &broadcast_messages,
            broadcast_auto_scroll,
            broadcast_scroll_offset,
            broadcast_usable_height,
        );

        let visible_broadcast: Vec<ListItem> = broadcast_messages
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let broadcast_list = List::new(visible_broadcast)
            .block(bordered_block(format!("Broadcast from {short_id}")));
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
        let (visible, effective_offset) =
            calc_visible_list_items(msgs, auto_scroll_val, scroll_offset_val, dm_usable_height);

        let visible_msgs: Vec<ListItem> = msgs
            .iter()
            .skip(effective_offset)
            .take(visible)
            .enumerate()
            .map(|(visible_idx, m)| {
                let global_idx = effective_offset.saturating_add(visible_idx);
                let msg_id = state
                    .dm_message_ids
                    .get(peer_id)
                    .and_then(|ids| ids.get(global_idx))
                    .and_then(|id| id.as_deref());
                let prefix = dm_receipt_prefix(msg_id, &state.dm_receipts);
                ListItem::new(format!("{prefix}{m}"))
            })
            .collect();

        let dm_list = List::new(visible_msgs).block(bordered_block(format!("DM: {short_id}")));
        f.render_widget(dm_list, dm_area);
    } else {
        let dm_para =
            Paragraph::new("No direct messages").block(bordered_block(format!("DM: {short_id}")));
        f.render_widget(dm_para, dm_area);
    }
}
