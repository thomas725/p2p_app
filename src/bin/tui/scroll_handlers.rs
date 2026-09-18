use super::state::AppState;
use p2p_app::{
    DisplayMessage, WHEEL_SCROLL_LINES, get_tui_logs, p2plog_debug,
    tui_helpers::key_code_to_scroll_action,
};

/// Handles tab navigation (Tab and `BackTab` keys)
pub async fn handle_navigation_key(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    match key_code {
        crossterm::event::KeyCode::Tab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = state
                .active_tab
                .saturating_add(1)
                .checked_rem(max_tabs)
                .unwrap_or(0);
            state.chat_scroll_offset = 0;
            state.cancel_nickname_edit();
            if state.active_tab == 0 {
                state.chat_unread_count = 0;
            }
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        crossterm::event::KeyCode::BackTab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = if state.active_tab == 0 {
                max_tabs.saturating_sub(1)
            } else {
                state.active_tab.saturating_sub(1)
            };
            state.chat_scroll_offset = 0;
            state.cancel_nickname_edit();
            if state.active_tab == 0 {
                state.chat_unread_count = 0;
            }
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        _ => {}
    }
}

/// Handle a single scroll key press for broadcast/DM section with mutable state
fn handle_scroll_key_for_section(
    key_code: crossterm::event::KeyCode,
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    max_offset: usize,
) {
    let Some(action) = key_code_to_scroll_action(key_code) else {
        return;
    };
    let (new_offset, new_auto) = p2p_app::tui_helpers::handle_scroll_key_for_section(
        action,
        *scroll_offset,
        *auto_scroll,
        max_offset,
    );
    *scroll_offset = new_offset;
    *auto_scroll = new_auto;
}

/// Line counts per rendered `List` item for string messages.
fn string_line_counts<'a>(messages: impl Iterator<Item = &'a String>) -> Vec<usize> {
    messages.map(|m| p2p_app::list_item_lines(m)).collect()
}

/// Line counts per rendered `List` item for broadcast messages.
fn display_line_counts<'a>(messages: impl Iterator<Item = &'a DisplayMessage>) -> Vec<usize> {
    messages
        .map(|m| p2p_app::list_item_lines(&m.text))
        .collect()
}

/// Number of visible message lines in the DM (bottom) pane of a DM tab.
///
/// The DM tab splits the message area (an `f.area().height - 8` chunk, i.e.
/// `chat_area_height + 2`) into two halves, each wrapped in its own 2-line
/// block border. With ratatui's default `Flex::Start`, an odd split gives the
/// extra row to the *top* pane, so the bottom pane is exactly
/// `floor((chat_area_height + 2) / 2)` rows.
fn dm_pane_visible_lines(state: &AppState) -> usize {
    let pane_height = state.chat_area_height.saturating_add(2).saturating_div(2);
    pane_height.saturating_sub(2).max(1)
}

/// Number of visible message lines in the broadcast (top) pane of a DM tab.
///
/// Mirror of [`dm_pane_visible_lines`]: the top pane receives the odd row, so
/// it is `ceil((chat_area_height + 2) / 2)` rows.
fn broadcast_pane_visible_lines(state: &AppState) -> usize {
    let pane_height = state.chat_area_height.saturating_add(3).saturating_div(2);
    pane_height.saturating_sub(2).max(1)
}

/// First global row of the DM (bottom) pane, used to route hovered scroll input.
///
/// The top pane starts at row 1 and is `ceil((chat_area_height + 2) / 2)` rows
/// tall, so the bottom pane begins on the following row.
const fn dm_pane_first_row(state: &AppState) -> usize {
    state
        .chat_area_height
        .saturating_add(3)
        .saturating_div(2)
        .saturating_add(1)
}

/// Handle scroll key for broadcast section of DM tab
fn scroll_broadcast_section(
    key_code: crossterm::event::KeyCode,
    state: &mut AppState,
    peer_id: &str,
) {
    let broadcast_messages: Vec<DisplayMessage> = state
        .messages
        .iter()
        .filter(|dm| dm.sender_peer_id.as_ref().is_some_and(|id| id == peer_id))
        .cloned()
        .collect();

    if broadcast_messages.is_empty() {
        return;
    }
    let line_counts = display_line_counts(broadcast_messages.iter());
    let max_offset =
        p2p_app::max_list_scroll_offset(&line_counts, broadcast_pane_visible_lines(state));

    if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handle scroll key for DM section of DM tab
fn scroll_dm_section(key_code: crossterm::event::KeyCode, state: &mut AppState, peer_id: &str) {
    let Some(msgs) = state.dm_messages.get(peer_id) else {
        return;
    };
    if msgs.is_empty() {
        return;
    }
    let line_counts = string_line_counts(msgs.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, dm_pane_visible_lines(state));
    if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id) {
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handle scroll key for Chat tab (broadcast)
fn scroll_chat_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let line_counts = display_line_counts(state.messages.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
    let was_auto_scroll = state.chat_auto_scroll;
    handle_scroll_key_for_section(
        key_code,
        &mut state.chat_scroll_offset,
        &mut state.chat_auto_scroll,
        max_offset,
    );
    if state.chat_auto_scroll && !was_auto_scroll {
        state.chat_unread_count = 0;
    }
}

/// Handle scroll key for Log tab
fn scroll_log_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let logs = get_tui_logs();
    let line_counts = string_line_counts(logs.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
    handle_scroll_key_for_section(
        key_code,
        &mut state.log_scroll_offset,
        &mut state.log_auto_scroll,
        max_offset,
    );
}

/// Handle scroll key for Peers tab
///
/// `page_size` is the number of data rows that fit the visible viewport. The
/// peer table lives in the content area `chat_area_height + 1`: minus 1 for the
/// in-block hint row, 2 for the block borders, and 1 for the header row.
fn expected_peer_page_size(state: &AppState) -> usize {
    state.chat_area_height.saturating_sub(2).max(1)
}

/// Number of data rows that fit the Groups list viewport.
///
/// The list needs the content chunk (`chat_area_height + 2`) minus the two
/// block borders and the hint row, i.e. `chat_area_height - 1`. Using the peer
/// page size here made `PageDown` step one row short.
fn expected_group_page_size(state: &AppState) -> usize {
    state.chat_area_height.saturating_sub(1).max(1)
}

#[allow(clippy::missing_const_for_fn)]
fn compute_new_peer_selection(
    key_code: crossterm::event::KeyCode,
    current_selection: usize,
    peer_count: usize,
    page_size: usize,
) -> usize {
    if peer_count == 0 {
        return 0;
    }
    let last = peer_count.saturating_sub(1);
    let page_size = page_size.max(1);
    match key_code {
        crossterm::event::KeyCode::Up => current_selection.saturating_sub(1),
        crossterm::event::KeyCode::Down => current_selection.saturating_add(1).min(last),
        crossterm::event::KeyCode::PageUp => current_selection.saturating_sub(page_size),
        crossterm::event::KeyCode::PageDown => {
            current_selection.saturating_add(page_size).min(last)
        }
        crossterm::event::KeyCode::Home => 0,
        crossterm::event::KeyCode::End => last,
        _ => current_selection,
    }
}

fn scroll_peers_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let page_size = expected_peer_page_size(state);
    state.peer_selection =
        compute_new_peer_selection(key_code, state.peer_selection, state.peers.len(), page_size);
}

/// Move the Groups list selection; the list uses the same visible-window math
/// as the Peers table.
fn scroll_groups_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let page_size = expected_group_page_size(state);
    state.group_selection = compute_new_peer_selection(
        key_code,
        state.group_selection,
        state.group_summaries.len(),
        page_size,
    );
}

/// Handle scroll key for a group chat tab
fn scroll_group_chat_tab(
    key_code: crossterm::event::KeyCode,
    state: &mut AppState,
    group_id: &str,
) {
    let Some(msgs) = state.group_messages.get(group_id) else {
        return;
    };
    if msgs.is_empty() {
        return;
    }
    let line_counts = string_line_counts(msgs.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
    if let Some((scroll_offset, auto_scroll)) = state.group_scroll_state.get_mut(group_id) {
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handles scroll keys (arrow keys, Page Up/Down, Home, End) with hover-aware targeting
pub async fn handle_scroll_key(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    match &tab_content {
        p2p_app::tui_tabs::TabContent::Peers => {
            scroll_peers_tab(key_code, state);
        }
        p2p_app::tui_tabs::TabContent::Groups => {
            scroll_groups_tab(key_code, state);
        }
        p2p_app::tui_tabs::TabContent::GroupChat(group_id) => {
            scroll_group_chat_tab(key_code, state, group_id);
        }
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            let mid_row = dm_pane_first_row(state);
            let mouse_row = usize::from(state.last_mouse_row);
            if mouse_row < mid_row {
                scroll_broadcast_section(key_code, state, peer_id);
            } else {
                scroll_dm_section(key_code, state, peer_id);
            }
        }
        p2p_app::tui_tabs::TabContent::Log => {
            scroll_log_tab(key_code, state);
        }
        p2p_app::tui_tabs::TabContent::Chat => {
            scroll_chat_tab(key_code, state);
        }
        p2p_app::tui_tabs::TabContent::PeerInfo(_) | p2p_app::tui_tabs::TabContent::Settings => {}
    }
}

fn apply_mouse_scroll(
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    scroll_dir: &str,
    max_offset: usize,
) -> bool {
    // Nothing can scroll: don't flip auto-scroll or request a redraw.
    if max_offset == 0 {
        return false;
    }
    let before_offset = *scroll_offset;
    let before_auto = *auto_scroll;
    match scroll_dir {
        // Wheel up leaves auto-scroll anchored at the current bottom, then
        // steps up; wheel down steps toward the bottom and re-engages
        // auto-scroll once the newest item is reached. This mirrors the
        // keyboard handlers so the wheel can always return to auto-scroll.
        "up" => {
            p2p_app::disable_auto_scroll_to_max(auto_scroll, scroll_offset, max_offset);
            p2p_app::scroll_up_lines(scroll_offset, WHEEL_SCROLL_LINES);
        }
        "down" => {
            p2p_app::scroll_down_lines(scroll_offset, auto_scroll, WHEEL_SCROLL_LINES, max_offset);
        }
        _ => {}
    }
    *scroll_offset != before_offset || *auto_scroll != before_auto
}

/// Handle mouse wheel for Chat tab (broadcast)
fn mouse_scroll_chat_tab(state: &mut AppState, scroll_dir: &str) -> bool {
    let line_counts = display_line_counts(state.messages.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
    let was_auto_scroll = state.chat_auto_scroll;
    let changed = apply_mouse_scroll(
        &mut state.chat_scroll_offset,
        &mut state.chat_auto_scroll,
        scroll_dir,
        max_offset,
    );
    if state.chat_auto_scroll && !was_auto_scroll {
        state.chat_unread_count = 0;
    }
    changed
}

/// Handle mouse wheel for Log tab
fn mouse_scroll_log_tab(state: &mut AppState, scroll_dir: &str) -> bool {
    let logs = get_tui_logs();
    let line_counts = string_line_counts(logs.iter());
    let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
    apply_mouse_scroll(
        &mut state.log_scroll_offset,
        &mut state.log_auto_scroll,
        scroll_dir,
        max_offset,
    )
}

/// Handle mouse wheel for a DM section (broadcast or DM side)
fn mouse_scroll_dm_section(
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    scroll_dir: &str,
    max_offset: usize,
) -> bool {
    apply_mouse_scroll(scroll_offset, auto_scroll, scroll_dir, max_offset)
}

/// Handles mouse wheel scrolling with hover-based section targeting for split DM tabs
pub fn handle_mouse_scroll(state: &mut AppState, scroll_dir: &str, peer_id: Option<&str>) -> bool {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);

    match &tab_content {
        p2p_app::tui_tabs::TabContent::Direct(pid) => {
            let mid_row = dm_pane_first_row(state);
            let mouse_row = usize::from(state.last_mouse_row);
            let pid = peer_id.unwrap_or(pid);
            if mouse_row < mid_row {
                let broadcast_messages: Vec<DisplayMessage> = state
                    .messages
                    .iter()
                    .filter(|dm| dm.sender_peer_id.as_ref().is_some_and(|id| id == pid))
                    .cloned()
                    .collect();
                let line_counts = display_line_counts(broadcast_messages.iter());
                let max_offset = p2p_app::max_list_scroll_offset(
                    &line_counts,
                    broadcast_pane_visible_lines(state),
                );
                if let Some((scroll_offset, auto_scroll)) =
                    state.dm_broadcast_scroll_state.get_mut(pid)
                {
                    mouse_scroll_dm_section(scroll_offset, auto_scroll, scroll_dir, max_offset)
                } else {
                    false
                }
            } else {
                let max_offset = state.dm_messages.get(pid).map_or(0, |msgs| {
                    let line_counts = string_line_counts(msgs.iter());
                    p2p_app::max_list_scroll_offset(&line_counts, dm_pane_visible_lines(state))
                });
                if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(pid) {
                    mouse_scroll_dm_section(scroll_offset, auto_scroll, scroll_dir, max_offset)
                } else {
                    false
                }
            }
        }
        p2p_app::tui_tabs::TabContent::Log => mouse_scroll_log_tab(state, scroll_dir),
        p2p_app::tui_tabs::TabContent::GroupChat(group_id) => {
            let Some(msgs) = state.group_messages.get(group_id) else {
                return false;
            };
            let line_counts = string_line_counts(msgs.iter());
            let max_offset = p2p_app::max_list_scroll_offset(&line_counts, state.chat_area_height);
            if let Some((scroll_offset, auto_scroll)) = state.group_scroll_state.get_mut(group_id) {
                apply_mouse_scroll(scroll_offset, auto_scroll, scroll_dir, max_offset)
            } else {
                false
            }
        }
        p2p_app::tui_tabs::TabContent::PeerInfo(_)
        | p2p_app::tui_tabs::TabContent::Groups
        | p2p_app::tui_tabs::TabContent::Settings => false,
        _ => mouse_scroll_chat_tab(state, scroll_dir),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_scroll_handlers.rs"]
mod tests;
