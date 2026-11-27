//! Broadcast-chat and direct-message pane rendering.

use crate::tui_render::{bordered_block, render_unread_banner};
use crate::tui_render_state::{
    TuiRenderState, broadcast_receipt_prefix, calc_visible_list_items, dm_receipt_prefix,
    reaction_suffix, selected_visible_index,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::collections::VecDeque;

/// Render chat messages with scroll support and receipt markers
pub fn render_chat_content(f: &mut ratatui::Frame, area: Rect, state: &mut TuiRenderState) {
    let usable_height = usize::from(area.height.saturating_sub(2));

    let (visible, effective_offset) = calc_visible_list_items(
        &state.messages,
        state.auto_scroll.chat,
        state.chat_scroll_offset,
        usable_height,
    );

    let selected = selected_visible_index(state.message_selection_index, effective_offset, visible);
    let visible_messages: Vec<ListItem> = state
        .messages
        .iter()
        .skip(effective_offset)
        .enumerate()
        .take(visible)
        .map(|(visible_idx, msg)| {
            let global_idx = effective_offset.saturating_add(visible_idx);
            let msg_id = state
                .message_ids
                .get(global_idx)
                .and_then(|id| id.as_deref());
            let prefix = broadcast_receipt_prefix(msg_id, &state.broadcast_receipts);
            let reactions = reaction_suffix(msg_id, &state.reactions);
            ListItem::new(format!("{prefix}{msg}{reactions}"))
        })
        .collect();

    let messages_list = List::new(visible_messages)
        .block(
            Block::default()
                .title("Broadcast Chat")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(state.palette.selection_bg()));
    let mut list_state = ListState::default();
    list_state.select(selected);
    f.render_stateful_widget(messages_list, area, &mut list_state);

    render_unread_banner(f, area, state.chat_unread_count, state.palette.accent());
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

    let short_id = crate::get_peer_display_name_or_short(peer_id);
    let dm_title = if state.typing_peer_ids.contains(peer_id) {
        format!("DM: {short_id} — typing…")
    } else {
        format!("DM: {short_id}")
    };

    render_dm_broadcast_pane(f, broadcast_area, peer_id, &short_id, state);
    render_dm_conversation_pane(f, dm_area, peer_id, &dm_title, state);

    render_unread_banner(
        f,
        dm_area,
        state.dm_unread_count.get(peer_id).copied().unwrap_or(0),
        state.palette.accent(),
    );
}

/// Top pane: broadcasts from `peer_id`, with the same reaction chips as Chat.
fn render_dm_broadcast_pane(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    short_id: &str,
    state: &mut TuiRenderState,
) {
    let usable_height = usize::from(area.height.saturating_sub(2));

    // Keep the wire ids aligned with the visible broadcast rows so this pane
    // shows the same reaction chips as the Chat tab and Flutter (US-008).
    let mut broadcast_messages = VecDeque::new();
    let mut broadcast_ids = VecDeque::new();
    for (idx, (msg, sender_id)) in state
        .messages
        .iter()
        .zip(state.message_peer_ids.iter())
        .enumerate()
    {
        if sender_id.as_ref().is_some_and(|id| id == peer_id) {
            broadcast_messages.push_back(msg.clone());
            broadcast_ids.push_back(state.message_ids.get(idx).and_then(Option::as_deref));
        }
    }

    if broadcast_messages.is_empty() {
        let para = Paragraph::new("No broadcast messages")
            .block(bordered_block(format!("Broadcast from {short_id}")));
        f.render_widget(para, area);
        return;
    }

    let (scroll_offset, auto_scroll) = {
        let (offset, auto_scroll) = state
            .dm_broadcast_scroll_state
            .entry(peer_id.to_string())
            .or_insert((broadcast_messages.len(), true));
        (*offset, *auto_scroll)
    };
    let (visible, effective_offset) = calc_visible_list_items(
        &broadcast_messages,
        auto_scroll,
        scroll_offset,
        usable_height,
    );

    let visible_broadcast: Vec<ListItem> = broadcast_messages
        .iter()
        .zip(broadcast_ids.iter())
        .skip(effective_offset)
        .take(visible)
        .map(|(m, id)| {
            let reactions = reaction_suffix(*id, &state.reactions);
            ListItem::new(format!("{m}{reactions}"))
        })
        .collect();

    let list =
        List::new(visible_broadcast).block(bordered_block(format!("Broadcast from {short_id}")));
    f.render_widget(list, area);
}

/// Bottom pane: the DM transcript. The message cursor is scoped to this pane;
/// bare scrolling may hover the top pane instead (see `KEYBOARD_SHORTCUTS.md`).
fn render_dm_conversation_pane(
    f: &mut ratatui::Frame,
    area: Rect,
    peer_id: &str,
    dm_title: &str,
    state: &mut TuiRenderState,
) {
    let usable_height = usize::from(area.height.saturating_sub(2));
    let (scroll_offset, auto_scroll) = {
        let (offset, auto_scroll) = state
            .dm_scroll_state
            .entry(peer_id.to_string())
            .or_insert((0, true));
        (*offset, *auto_scroll)
    };
    let Some(msgs) = state.dm_messages.get(peer_id) else {
        let para = Paragraph::new("No direct messages").block(bordered_block(dm_title.to_string()));
        f.render_widget(para, area);
        return;
    };

    let (visible, effective_offset) =
        calc_visible_list_items(msgs, auto_scroll, scroll_offset, usable_height);
    let selected = selected_visible_index(state.message_selection_index, effective_offset, visible);
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
            let reactions = reaction_suffix(msg_id, &state.reactions);
            ListItem::new(format!("{prefix}{m}{reactions}"))
        })
        .collect();

    let list = List::new(visible_msgs)
        .block(bordered_block(dm_title.to_string()))
        .highlight_style(Style::default().bg(state.palette.selection_bg()));
    let mut list_state = ListState::default();
    list_state.select(selected);
    f.render_stateful_widget(list, area, &mut list_state);
}
