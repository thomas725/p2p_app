use super::state::AppState;
use super::state::MAX_DM_HISTORY;
use p2p_app::p2plog_debug;
use std::collections::{HashMap, VecDeque};

/// Handles tab bar clicks and close button.
///
/// Mirrors how ratatui's `Tabs` widget lays a bar out: every tab renders as
/// `" " + title + " "`, and a `"|"` divider is drawn *between* tabs only. The
/// close button `[X]` occupies the last three display columns of a removable
/// title. Widths are measured in terminal columns (not UTF-8 bytes) so wide
/// glyphs in a title don't shift the hitboxes.
fn handle_tab_click(state: &mut AppState, mouse_column: u16, tab_titles: &[String]) -> bool {
    use unicode_width::UnicodeWidthStr;

    let click_col = usize::from(mouse_column);
    let mut col_pos: usize = 0;
    for (idx, title) in tab_titles.iter().enumerate() {
        // " " + title + " "; the divider column after it is owned by neither tab.
        let title_end = col_pos
            .saturating_add(1)
            .saturating_add(title.as_str().width());
        let tab_end = title_end.saturating_add(1);
        if click_col >= col_pos && click_col < tab_end {
            let close_start = title_end.saturating_sub(3);
            if title.contains("[X]") && click_col >= close_start && click_col < title_end {
                let tab_content = state.dynamic_tabs.tab_index_to_content(idx);
                if let Some(closed_idx) = state.dynamic_tabs.remove_tab(&tab_content) {
                    // Keep focus put unless it pointed at (or after) the tab
                    // that just closed, in which case shift it down by one.
                    if state.active_tab == closed_idx {
                        state.active_tab = closed_idx.saturating_sub(1);
                    } else if state.active_tab > closed_idx {
                        state.active_tab = state.active_tab.saturating_sub(1);
                    }
                    p2plog_debug(format!("Closed tab via mouse: {tab_content:?}"));
                }
                return true;
            }
            if idx != state.active_tab {
                state.active_tab = idx;
                state.chat_scroll_offset = 0;
                state.cancel_nickname_edit();
                p2plog_debug(format!(
                    "Switched to tab {} via mouse click",
                    state.active_tab
                ));
                return true;
            }
            return false;
        }
        col_pos = tab_end.saturating_add(1);
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

/// Pure: formats DB messages into display-ready strings for a group chat.
///
/// The database returns newest-first; we reverse to chronological order.
fn format_group_messages_from_db(
    db_messages: &[p2p_app::generated::models_queryable::GroupMessage],
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    own_nickname: &str,
) -> (
    VecDeque<String>,
    VecDeque<Option<String>>,
    VecDeque<Option<String>>,
) {
    let mut messages = VecDeque::new();
    let mut message_ids = VecDeque::new();
    let mut peer_ids = VecDeque::new();
    for msg in db_messages.iter().rev() {
        let ts = p2p_app::format_peer_datetime(msg.created_at);
        let sender = msg.sender_nickname.as_ref().map_or_else(
            || {
                msg.peer_id.as_ref().map_or_else(
                    || format!("[{own_nickname}]"),
                    |pid| {
                        format!(
                            "[{}]",
                            p2p_app::peer_display_name(pid, local_nicknames, received_nicknames)
                        )
                    },
                )
            },
            |nick| format!("[{nick}]"),
        );
        messages.push_back(format!("{ts} {sender} {}", msg.content));
        message_ids.push_back(msg.msg_id.clone());
        peer_ids.push_back(msg.peer_id.clone());
    }
    (messages, message_ids, peer_ids)
}

/// Load a group's message history from the database into state.
///
/// The database is the single source of truth for incoming group messages (the
/// swarm handler persists them centrally), so opening a chat always reloads the
/// map from the DB; live messages are then appended on top while the tab stays
/// open. Replacing on every open avoids duplicate re-deliveries.
pub fn load_group_messages_for(state: &mut AppState, group_id: &str) {
    if let Ok(db_messages) = p2p_app::groups::load_group_messages(group_id, MAX_DM_HISTORY) {
        let (messages, message_ids, peer_ids) = format_group_messages_from_db(
            &db_messages,
            &state.local_nicknames,
            &state.received_nicknames,
            &state.own_nickname,
        );
        state.group_messages.insert(group_id.to_string(), messages);
        state
            .group_message_ids
            .insert(group_id.to_string(), message_ids);
        state
            .group_message_peer_ids
            .insert(group_id.to_string(), peer_ids);
        let msg_count = db_messages.len();
        state
            .group_scroll_state
            .insert(group_id.to_string(), (msg_count, true));
        p2plog_debug(format!("Loaded {msg_count} messages for group {group_id}"));
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

/// Open a group chat: load its history, add (or focus) its tab.
pub fn open_group_chat(state: &mut AppState, group_id: &str, display_name: &str) {
    load_group_messages_for(state, group_id);
    let tab_idx = state
        .dynamic_tabs
        .add_group_tab(group_id.to_string(), display_name.to_string());
    state.active_tab = tab_idx;
    state.cancel_nickname_edit();
}

/// Handles group row clicks in the Groups tab
fn handle_group_row_click(state: &mut AppState, row: u16) -> bool {
    if state.group_summaries.is_empty() {
        return false;
    }
    // Data rows start at global row 2 (tab bar + block border). The viewport
    // matches render_groups_content: `area.height - 3` (2 borders + 1 hint)
    // where the content chunk is `chat_area_height + 2`.
    let page_height = state.chat_area_height.saturating_sub(1).max(1);
    let selected = state
        .group_selection
        .min(state.group_summaries.len().saturating_sub(1));
    let (start, _end) = p2p_app::tui_helpers::peer_table_visible_range(
        0,
        Some(selected),
        state.group_summaries.len(),
        page_height,
    );
    let group_row = start.saturating_add(usize::from(row).saturating_sub(2));
    if group_row < state.group_summaries.len()
        && let Some(g) = state.group_summaries.get(group_row)
    {
        let group_id = g.group.group_id.clone();
        let display_name = g.group.display_name.clone();
        state.group_selection = group_row;
        open_group_chat(state, &group_id, &display_name);
        p2plog_debug(format!("Opened group via mouse: {display_name}"));
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
        p2p_app::tui_tabs::TabContent::GroupChat(group_id) => {
            let Some(strings) = state.group_messages.get(group_id) else {
                return false;
            };
            if strings.is_empty() {
                return false;
            }
            let usable_height = state.chat_area_height;
            let (offset, auto_scroll) = state
                .group_scroll_state
                .get(group_id)
                .copied()
                .unwrap_or((0, true));
            let (visible, start) =
                p2p_app::calc_visible_list_items(strings, auto_scroll, offset, usable_height);
            let line_counts: Vec<usize> = strings
                .iter()
                .skip(start)
                .take(visible)
                .map(|m| p2p_app::list_item_lines(m))
                .collect();
            let click_row = usize::from(mouse_row);
            if let Some(rel) = p2p_app::row_to_visible_index(&line_counts, 2, click_row) {
                let actual_idx = start.saturating_add(rel);
                if let Some(peer_id) = state
                    .group_message_peer_ids
                    .get(group_id)
                    .and_then(|ids| ids.get(actual_idx))
                    .and_then(Clone::clone)
                {
                    let idx = state.dynamic_tabs.add_peer_info_tab(peer_id.clone());
                    state.active_tab = idx;
                    p2plog_debug(format!("Opened Peer Info for group sender: {peer_id}"));
                    return true;
                }
            }
            false
        }
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
            // Match render_chat_content: usable height is `chat_area_height`
            // (content block height minus its 2-line border).
            let usable_height = state.chat_area_height;
            let (visible, start) = p2p_app::calc_visible_list_items(
                &strings,
                state.chat_auto_scroll,
                state.chat_scroll_offset,
                usable_height,
            );
            // The chat renderer shows one `ListItem` per message, and ratatui's
            // `List` lays each item out over `list_item_lines` rows (one per
            // newline-segment; long lines are clipped, never width-wrapped).
            // The click mapping uses the same counts so hits land on the exact
            // row a message occupies.
            let line_counts: Vec<usize> = strings
                .iter()
                .skip(start)
                .take(visible)
                .map(|m| p2p_app::list_item_lines(m))
                .collect();
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
    let is_groups_tab = matches!(tab_content, p2p_app::tui_tabs::TabContent::Groups);
    // The peers table and groups list reserve the bottom row of the content
    // chunk for a hint, so their block's bottom border sits one row higher than
    // the full-height message panes'. Clicks on that border must not land on a
    // phantom trailing row.
    let max_row = if is_peers_tab || is_groups_tab {
        state.chat_area_height
    } else {
        state.chat_area_height.saturating_add(1)
    };
    let clickable = is_peers_tab
        || is_groups_tab
        || matches!(
            tab_content,
            p2p_app::tui_tabs::TabContent::Chat
                | p2p_app::tui_tabs::TabContent::Log
                | p2p_app::tui_tabs::TabContent::Direct(_)
                | p2p_app::tui_tabs::TabContent::GroupChat(_)
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
        if is_groups_tab {
            return handle_group_row_click(state, mouse_row);
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
        &state.broadcast_sent_to_peer,
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
