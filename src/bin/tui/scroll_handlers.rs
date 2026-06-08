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
            state.active_tab = (state.active_tab + 1) % max_tabs;
            state.chat_scroll_offset = 0;
            state.cancel_nickname_edit();
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        crossterm::event::KeyCode::BackTab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = if state.active_tab == 0 {
                max_tabs - 1
            } else {
                state.active_tab - 1
            };
            state.chat_scroll_offset = 0;
            state.cancel_nickname_edit();
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

    if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
        let visible_count = state.dm_visible_counts.get(peer_id).map_or(1, |(b, _)| *b);
        let max_offset = broadcast_messages.len().saturating_sub(visible_count);
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handle scroll key for DM section of DM tab
fn scroll_dm_section(key_code: crossterm::event::KeyCode, state: &mut AppState, peer_id: &str) {
    if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id)
    {
        let visible_count = state.dm_visible_counts.get(peer_id).map_or(1, |(_, d)| *d);
        let max_offset = msgs.len().saturating_sub(visible_count);
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handle scroll key for Chat tab (broadcast)
fn scroll_chat_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let max_offset = state
        .messages
        .len()
        .saturating_sub(state.visible_message_count);
    handle_scroll_key_for_section(
        key_code,
        &mut state.chat_scroll_offset,
        &mut state.chat_auto_scroll,
        max_offset,
    );
}

/// Handle scroll key for Log tab
fn scroll_log_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let log_len = get_tui_logs().len();
    let max_offset = log_len.saturating_sub(state.visible_log_count);
    handle_scroll_key_for_section(
        key_code,
        &mut state.log_scroll_offset,
        &mut state.log_auto_scroll,
        max_offset,
    );
}

/// Handle scroll key for Peers tab
fn compute_new_peer_selection(
    key_code: crossterm::event::KeyCode,
    current_selection: usize,
    peer_count: usize,
) -> usize {
    match key_code {
        crossterm::event::KeyCode::Up => current_selection.saturating_sub(1),
        crossterm::event::KeyCode::Down if current_selection < peer_count.saturating_sub(1) => {
            current_selection + 1
        }
        _ => current_selection,
    }
}

fn scroll_peers_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    state.peer_selection =
        compute_new_peer_selection(key_code, state.peer_selection, state.peers.len());
}

/// Handles scroll keys (arrow keys, Page Up/Down, Home, End) with hover-aware targeting
pub async fn handle_scroll_key(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    match &tab_content {
        p2p_app::tui_tabs::TabContent::Peers => {
            scroll_peers_tab(key_code, state);
        }
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            let mid_row = 2 + (state.chat_area_height / 2);
            let mouse_row = state.last_mouse_row as usize;
            if mouse_row < mid_row {
                scroll_broadcast_section(key_code, state, peer_id);
            } else {
                scroll_dm_section(key_code, state, peer_id);
            }
        }
        p2p_app::tui_tabs::TabContent::Log => {
            scroll_log_tab(key_code, state);
        }
        _ => {
            scroll_chat_tab(key_code, state);
        }
    }
}

fn apply_mouse_scroll(
    scroll_offset: &mut usize,
    auto_scroll: bool,
    scroll_dir: &str,
    max_offset: usize,
) -> Option<usize> {
    if auto_scroll {
        return None;
    }
    let before = *scroll_offset;
    match scroll_dir {
        "up" => *scroll_offset = scroll_offset.saturating_sub(WHEEL_SCROLL_LINES),
        "down" => *scroll_offset = (*scroll_offset + WHEEL_SCROLL_LINES).min(max_offset),
        _ => {}
    }
    Some(before)
}

/// Handle mouse wheel for Chat tab (broadcast)
fn mouse_scroll_chat_tab(state: &mut AppState, scroll_dir: &str) -> bool {
    let max_offset = state
        .messages
        .len()
        .saturating_sub(state.visible_message_count);
    apply_mouse_scroll(
        &mut state.chat_scroll_offset,
        state.chat_auto_scroll,
        scroll_dir,
        max_offset,
    )
    .is_some_and(|before| state.chat_scroll_offset != before)
}

/// Handle mouse wheel for Log tab
fn mouse_scroll_log_tab(state: &mut AppState, scroll_dir: &str) -> bool {
    let max_offset = get_tui_logs().len().saturating_sub(state.visible_log_count);
    apply_mouse_scroll(
        &mut state.log_scroll_offset,
        state.log_auto_scroll,
        scroll_dir,
        max_offset,
    )
    .is_some_and(|before| state.log_scroll_offset != before)
}

/// Handle mouse wheel for a DM section (broadcast or DM side)
fn mouse_scroll_dm_section(
    scroll_offset: &mut usize,
    auto_scroll: bool,
    len: usize,
    scroll_dir: &str,
) -> bool {
    let max_offset = len.saturating_sub(1);
    apply_mouse_scroll(scroll_offset, auto_scroll, scroll_dir, max_offset)
        .is_some_and(|before| *scroll_offset != before)
}

/// Handles mouse wheel scrolling with hover-based section targeting for split DM tabs
pub fn handle_mouse_scroll(state: &mut AppState, scroll_dir: &str, peer_id: Option<&str>) -> bool {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);

    match &tab_content {
        p2p_app::tui_tabs::TabContent::Direct(pid) => {
            let mid_row = 2 + (state.chat_area_height / 2);
            let mouse_row = state.last_mouse_row as usize;
            let pid = peer_id.unwrap_or(pid);
            if mouse_row < mid_row {
                let msgs: Vec<_> = state
                    .messages
                    .iter()
                    .filter(|dm| dm.sender_peer_id.as_ref().is_some_and(|id| id == pid))
                    .collect();
                if let Some((scroll_offset, auto_scroll)) =
                    state.dm_broadcast_scroll_state.get_mut(pid)
                {
                    mouse_scroll_dm_section(scroll_offset, *auto_scroll, msgs.len(), scroll_dir)
                } else {
                    false
                }
            } else if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(pid)
                && let Some(msgs) = state.dm_messages.get(pid)
            {
                mouse_scroll_dm_section(scroll_offset, *auto_scroll, msgs.len(), scroll_dir)
            } else {
                false
            }
        }
        p2p_app::tui_tabs::TabContent::Log => mouse_scroll_log_tab(state, scroll_dir),
        _ => mouse_scroll_chat_tab(state, scroll_dir),
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_scroll_handlers.rs"]
mod tests;
