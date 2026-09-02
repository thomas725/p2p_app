use super::state::AppState;
use super::state::MAX_DM_HISTORY;
use p2p_app::p2plog_debug;
use std::collections::{HashMap, VecDeque};

/// Handles tab bar clicks and close button
fn handle_tab_click(state: &mut AppState, mouse_column: u16, tab_titles: &[String]) -> bool {
    let mut col_pos: usize = 0;
    for (idx, title) in tab_titles.iter().enumerate() {
        let tab_width = title.len().saturating_add(3);
        let tab_end = col_pos.saturating_add(tab_width);
        if usize::from(mouse_column) >= col_pos && usize::from(mouse_column) < tab_end {
            let close_start = tab_end.saturating_sub(4);
            if usize::from(mouse_column) >= close_start && title.contains("[X]") {
                let tab_content = state.dynamic_tabs.tab_index_to_content(idx);
                let closed_idx = match &tab_content {
                    p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
                        state.dynamic_tabs.remove_dm_tab(peer_id)
                    }
                    p2p_app::tui_tabs::TabContent::PeerInfo(peer_id) => {
                        state.dynamic_tabs.remove_peer_info_tab(peer_id)
                    }
                    _ => None,
                };
                if let Some(closed_idx) = closed_idx {
                    state.active_tab = if closed_idx > 0 {
                        closed_idx.saturating_sub(1)
                    } else {
                        0
                    };
                    p2plog_debug(format!("Closed tab via mouse: {tab_content:?}"));
                }
                return true;
            } else if idx != state.active_tab {
                state.active_tab = idx;
                state.chat_scroll_offset = 0;
                state.cancel_nickname_edit();
                p2plog_debug(format!(
                    "Switched to tab {} via mouse click",
                    state.active_tab
                ));
                return true;
            }
            break;
        }
        col_pos = tab_end;
    }
    false
}

/// Pure: formats DB messages into display-ready strings for a DM chat.
///
/// Separated from the DB call so it can be unit-tested without a database.
fn format_dm_messages_from_db(
    db_messages: &[p2p_app::generated::models_queryable::Message],
    self_nick_for_peer: &str,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
) -> VecDeque<String> {
    let mut messages = VecDeque::new();
    for msg in db_messages.iter().rev() {
        let ts = p2p_app::format_peer_datetime(msg.created_at);
        let sender_display = msg.peer_id.as_ref().map_or_else(
            || self_nick_for_peer.to_string(),
            |p| p2p_app::peer_display_name(p, local_nicknames, received_nicknames),
        );
        messages.push_back(format!("{} [{}] {}", ts, sender_display, msg.content));
    }
    messages
}

/// Loads DM messages from database for a peer
pub fn load_dm_messages(state: &mut AppState, peer_id: &str) {
    if !state.dm_messages.contains_key(peer_id) {
        if let Ok(db_messages) = p2p_app::load_direct_messages(peer_id, MAX_DM_HISTORY) {
            let self_nick_for_peer = state
                .self_nicknames_for_peers
                .get(peer_id)
                .cloned()
                .unwrap_or_else(|| state.own_nickname.clone());
            let messages = format_dm_messages_from_db(
                &db_messages,
                &self_nick_for_peer,
                &state.local_nicknames,
                &state.received_nicknames,
            );
            state.dm_messages.insert(peer_id.to_string(), messages);
            state.dm_message_ids.insert(
                peer_id.to_string(),
                std::iter::repeat_with(|| None)
                    .take(db_messages.len())
                    .collect(),
            );
            let msg_count = db_messages.len();
            state
                .dm_scroll_state
                .entry(peer_id.to_string())
                .or_insert((msg_count, true));
            p2plog_debug(format!("Loaded {msg_count} DM messages for {peer_id}"));
        }
    } else if !state.dm_scroll_state.contains_key(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id)
    {
        state
            .dm_scroll_state
            .insert(peer_id.to_string(), (msgs.len(), true));
    }
}

/// Handles peer row clicks in the Peers tab
fn handle_peer_row_click(state: &mut AppState, row: u16) -> bool {
    if state.peers.is_empty() {
        return false;
    }
    // Map the clicked screen row to an absolute peer index, honoring the
    // scrolling viewport: data rows start at global row 3 (tab + block
    // border + table header) and are offset by the first visible row.
    let page_height = state.chat_area_height.saturating_sub(2).max(1);
    let selected = state
        .peer_selection
        .min(state.peers.len().saturating_sub(1));
    let (start, _end) = p2p_app::tui_helpers::peer_table_visible_range(
        state.peer_table_offset,
        Some(selected),
        state.peers.len(),
        page_height,
    );
    let peer_row = start.saturating_add(usize::from(row).saturating_sub(3));
    if peer_row < state.peers.len()
        && let Some(p) = state.peers.get(peer_row)
    {
        let peer_id_clone = p.peer_id.clone();
        state.peer_selection = peer_row;
        load_dm_messages(state, &peer_id_clone);
        let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
        state.active_tab = tab_idx;
        state.cancel_nickname_edit();
        p2plog_debug(format!("Opened DM with peer via mouse: {peer_id_clone}"));
        return true;
    }
    false
}

/// Handles message row clicks in the Chat / DM tabs, opening the
/// sender's Peer Info tab. (Log lines have no sender, so they are a no-op.)
fn handle_message_click(
    state: &mut AppState,
    mouse_row: u16,
    tab_content: &p2p_app::tui_tabs::TabContent,
) -> bool {
    match tab_content {
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            let idx = state.dynamic_tabs.add_peer_info_tab(peer_id.clone());
            state.active_tab = idx;
            p2plog_debug(format!("Opened Peer Info for DM peer: {peer_id}"));
            true
        }
        p2p_app::tui_tabs::TabContent::Chat => {
            let strings: VecDeque<String> = state.messages.iter().map(|m| m.text.clone()).collect();
            if strings.is_empty() {
                return false;
            }
            // Match render_chat_content: text wraps at `width - 4`, usable height is
            // `chat_area_height` (content block height minus its 2-line border).
            let text_width = state.terminal_width.saturating_sub(4).max(1);
            let usable_height = state.chat_area_height;
            let (visible, start) = p2p_app::calc_visible_strings(
                &strings,
                state.chat_auto_scroll,
                state.chat_scroll_offset,
                text_width,
                usable_height,
            );
            // The chat renderer shows each message as a single (clipped, non-wrapped)
            // List item, so every message occupies exactly one terminal row. The
            // click mapping must match that layout rather than assuming word-wrap,
            // i.e. the visible window is `visible` one-line rows starting at `start`.
            let line_counts: Vec<usize> = vec![1; visible];
            let click_row = usize::from(mouse_row);
            if let Some(rel) = p2p_app::row_to_visible_index(&line_counts, 2, click_row) {
                let actual_idx = start.saturating_add(rel);
                if let Some(msg) = state.messages.get(actual_idx)
                    && let Some(peer_id) = &msg.sender_peer_id
                {
                    let idx = state.dynamic_tabs.add_peer_info_tab(peer_id.clone());
                    state.active_tab = idx;
                    p2plog_debug(format!("Opened Peer Info for sender: {peer_id}"));
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// Handles left mouse button clicks
pub fn handle_mouse_left_click(
    state: &mut AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_peers_tab: bool,
) -> bool {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        return handle_tab_click(state, mouse_column, &tab_titles);
    }
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    let max_row = state.chat_area_height.saturating_add(1);
    let clickable = is_peers_tab
        || matches!(
            tab_content,
            p2p_app::tui_tabs::TabContent::Chat
                | p2p_app::tui_tabs::TabContent::Log
                | p2p_app::tui_tabs::TabContent::Direct(_)
        );
    if clickable && mouse_row > 1 && usize::from(mouse_row) <= max_row {
        if is_peers_tab {
            // The peers view is a table; the row just inside the border (global
            // row 2) is the header, which toggles/sets the sort column.
            if mouse_row == 2 {
                return handle_peer_header_click(state, mouse_column);
            }
            return handle_peer_row_click(state, mouse_row);
        }
        return handle_message_click(state, mouse_row, &tab_content);
    }
    false
}

/// Handles a click on the peers-table header, mapping the column under the
/// cursor to a sort column (mirroring Flutter's `_PeerList` header tap).
fn handle_peer_header_click(state: &mut AppState, column: u16) -> bool {
    // Resolve the column geometry with ratatui's own `Layout` so the sort
    // column matches exactly what the `Table` widget rendered: the columns are
    // sized to fit their longest header/cell (`peer_table_column_widths`) over
    // the same visible window the renderer materializes, with the same block
    // borders and `column_spacing`.
    let sender_ids: VecDeque<Option<String>> = state
        .messages
        .iter()
        .map(|m| m.sender_peer_id.clone())
        .collect();
    let peers: Vec<p2p_app::PeerRecord> = state.peers.iter().cloned().collect();
    let page_height = state.chat_area_height.saturating_sub(2).max(1);
    let selected = state.peer_selection.min(peers.len().saturating_sub(1));
    let (start, end) = p2p_app::tui_helpers::peer_table_visible_range(
        state.peer_table_offset,
        Some(selected),
        peers.len(),
        page_height,
    );
    let rows = p2p_app::tui_helpers::peer_table_rows_range(
        &peers,
        &state.dm_messages,
        &sender_ids,
        start,
        end,
    );
    let widths = p2p_app::tui_helpers::peer_table_column_widths(
        &rows,
        state.peer_sort_column,
        state.peer_sort_ascending,
    );
    let constraints: Vec<ratatui::layout::Constraint> = widths
        .into_iter()
        .map(|w| ratatui::layout::Constraint::Length(u16::try_from(w).unwrap_or(u16::MAX)))
        .collect();
    let width = u16::try_from(state.terminal_width.max(1)).unwrap_or(u16::MAX);
    let area = ratatui::layout::Rect::new(0, 0, width, 1);
    let inner = ratatui::widgets::Block::default()
        .borders(ratatui::widgets::Borders::ALL)
        .inner(area);
    let cols = ratatui::layout::Layout::default()
        .direction(ratatui::layout::Direction::Horizontal)
        .constraints(constraints)
        .spacing(1)
        .split(inner);
    let col = if column < cols.first().map_or(0, |c| c.x) {
        0
    } else {
        // The first column whose right edge is past the click; a click in the
        // 1-char gap between columns resolves to the column that follows it.
        cols.iter()
            .position(|c| column < c.x.saturating_add(c.width))
            .unwrap_or(4)
    };
    if state.peer_sort_column == col {
        state.peer_sort_ascending = !state.peer_sort_ascending;
    } else {
        state.peer_sort_column = col;
        state.peer_sort_ascending = false;
    }
    state.resort_peers();
    p2plog_debug(format!(
        "Sorted peers by column {col} ascending={}",
        state.peer_sort_ascending
    ));
    true
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_click_handlers.rs"]
mod tests;
