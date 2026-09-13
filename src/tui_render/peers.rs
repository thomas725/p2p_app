//! Peers-table rendering (stateful `Table` + sort hint).

use crate::tui_helpers::{
    peer_table_column_widths, peer_table_header_labels, peer_table_rows_range,
    peer_table_visible_range,
};
use crate::tui_render_state::TuiRenderState;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
};

/// Render peer list with selection support
pub fn render_peers_content(f: &mut ratatui::Frame, area: Rect, state: &TuiRenderState) {
    // The table body is content_height - 1 (hint) - 2 (borders) - 1 (header)
    // rows tall; a stateful `TableState` scrolls the viewport to keep the
    // selected row visible, so PageUp/PageDown step the selection by a page.
    let page_height = usize::from(area.height.saturating_sub(4)).max(1);
    let total = state.peers.len();
    let selected = (total > 0).then(|| state.peer_selection.min(total.saturating_sub(1)));
    let (start, end) =
        peer_table_visible_range(state.peer_table_offset, selected, total, page_height);

    // Materialize only the visible window: display-name lookups are the
    // expensive per-frame cost, so paged rendering stays O(page) no matter how
    // many peers are known.
    let rows = peer_table_rows_range(
        &state.peers,
        &state.dm_messages,
        &state.broadcast_sent_to_peer,
        start,
        end,
    );

    let header_cells: Vec<Cell> =
        peer_table_header_labels(state.peer_sort_column, state.peer_sort_ascending)
            .into_iter()
            .map(Cell::from)
            .collect();
    let header = Row::new(header_cells).style(Style::default().add_modifier(Modifier::BOLD));

    let body: Vec<Row> = rows
        .iter()
        .map(|r| {
            let cells = [
                Cell::from(r.display_name.clone()),
                Cell::from(r.dm_count.to_string()),
                Cell::from(r.broadcast_count.to_string()),
                Cell::from(r.last_seen.clone()),
                Cell::from(r.first_seen.clone()),
            ];
            Row::new(cells)
        })
        .collect();

    let widths: Vec<Constraint> =
        peer_table_column_widths(&rows, state.peer_sort_column, state.peer_sort_ascending)
            .into_iter()
            .map(|w| Constraint::Length(u16::try_from(w).unwrap_or(u16::MAX)))
            .collect();
    let table = Table::new(body, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!("Connected Peers ({total})"))
                .borders(Borders::ALL),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray));

    // Reserve the last row of the block's inner area for a sort hint.
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    let table_area = *inner.first().unwrap_or(&area);
    let hint_area = *inner.get(1).unwrap_or(&area);

    let mut table_state = TableState::new();
    table_state.select(selected.map(|sel| sel.saturating_sub(start)));
    *table_state.offset_mut() = 0;
    f.render_stateful_widget(table, table_area, &mut table_state);

    let hint = Paragraph::new(
        "1-5 (or n/m/b/l/f): sort · o: toggle order · click header · PgUp/PgDn: page",
    )
    .style(Style::default().add_modifier(Modifier::DIM));
    f.render_widget(hint, hint_area);
}
