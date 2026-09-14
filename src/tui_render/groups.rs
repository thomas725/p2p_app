//! Groups-list and group-chat rendering.

use crate::tui_helpers::peer_table_visible_range;
use crate::tui_render::bordered_block;
use crate::tui_render_state::{TuiRenderState, calc_visible_list_items};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};

/// Render the Groups list tab with selection support.
pub fn render_groups_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    // One row per group; the block loses 2 rows to borders and 1 to the hint.
    let page_height = usize::from(area.height.saturating_sub(3)).max(1);
    let total = state.group_summaries.len();
    let selected = (total > 0).then(|| state.group_selection.min(total.saturating_sub(1)));
    let (start, end) = peer_table_visible_range(0, selected, total, page_height);

    let items: Vec<ListItem> = state
        .group_summaries
        .iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|g| {
            ListItem::from(format!("{} ({})", g.group.display_name, g.member_count))
                .style(Style::default().add_modifier(Modifier::BOLD))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(format!("Groups ({total})"))
                .borders(Borders::ALL),
        )
        .highlight_style(Style::default().bg(Color::DarkGray));

    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let list_area = *inner.first().unwrap_or(&area);
    let hint_area = *inner.get(1).unwrap_or(&area);

    let mut list_state = ListState::default();
    list_state.select(selected.map(|sel| sel.saturating_sub(start)));
    f.render_stateful_widget(list, list_area, &mut list_state);

    let hint = if state.creating_group {
        let name = state.input_text.clone();
        let target = if name.is_empty() {
            "enter a group name".to_string()
        } else {
            format!("\"{name}\"")
        };
        format!("creating/joining {target} — Enter confirms, Esc cancels")
    } else {
        "g: create or join group · Enter: open · PgUp/PgDn: page".to_string()
    };
    let hint = Paragraph::new(hint).style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(hint, hint_area);
}

/// Render a single group conversation (mirrors the broadcast-chat pane).
pub fn render_group_chat_content(
    f: &mut ratatui::Frame,
    area: Rect,
    group_id: &str,
    state: &TuiRenderState,
) {
    let display_name = state
        .group_summaries
        .iter()
        .find(|g| g.group.group_id == group_id)
        .map_or_else(|| group_id.to_string(), |g| g.group.display_name.clone());
    let usable_height = usize::from(area.height.saturating_sub(2));

    if let Some(msgs) = state.group_messages.get(group_id) {
        let (scroll_offset, auto_scroll) = state
            .group_scroll_state
            .get(group_id)
            .copied()
            .unwrap_or((0, true));
        let (visible, effective_offset) =
            calc_visible_list_items(msgs, auto_scroll, scroll_offset, usable_height);

        let items: Vec<ListItem> = msgs
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let list = List::new(items).block(bordered_block(format!("Group: {display_name}")));
        f.render_widget(list, area);
    } else {
        let para = Paragraph::new("No messages in this group yet")
            .block(bordered_block(format!("Group: {display_name}")));
        f.render_widget(para, area);
    }
}
