//! Logs-tab rendering.

use crate::tui_render_state::{TuiRenderState, calc_visible_list_items};
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, List, ListItem},
};

/// Render log content
pub fn render_log_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    let usable_height = usize::from(area.height.saturating_sub(2));

    let (visible, effective_offset) = calc_visible_list_items(
        &state.log_messages,
        state.log_auto_scroll,
        state.log_scroll_offset,
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
