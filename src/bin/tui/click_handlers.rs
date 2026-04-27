use super::constants::MAX_DM_HISTORY;
use super::state::AppState;
use p2p_app::p2plog_debug;
use ratatui_textarea::TextArea;
use std::collections::VecDeque;

/// Handles tab bar clicks and close button
pub fn handle_tab_click(state: &mut AppState, mouse_column: u16, tab_titles: &[String]) -> bool {
    let mut col_pos = 0;
    for (idx, title) in tab_titles.iter().enumerate() {
        let tab_width = title.len() + 3;
        let tab_end = col_pos + tab_width;
        if (mouse_column as usize) >= col_pos && (mouse_column as usize) < tab_end {
            let close_start = tab_end - 4;
            if (mouse_column as usize) >= close_start && title.contains("(X)") {
                let tab_content = state.dynamic_tabs.tab_index_to_content(idx);
                if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = tab_content
                    && let Some(closed_idx) = state.dynamic_tabs.remove_dm_tab(&peer_id)
                {
                    state.active_tab = if closed_idx > 0 { closed_idx - 1 } else { 0 };
                    p2plog_debug(format!("Closed DM tab via mouse: {}", peer_id));
                }
                return true;
            } else if idx != state.active_tab {
                state.active_tab = idx;
                state.chat_scroll_offset = 0;
                p2plog_debug(format!("Switched to tab {} via mouse click", state.active_tab));
                return true;
            }
            break;
        }
        col_pos = tab_end;
    }
    false
}

/// Loads DM messages from database for a peer
pub fn load_dm_messages(state: &mut AppState, peer_id: &str) {
    if !state.dm_messages.contains_key(peer_id) {
        if let Ok(db_messages) = p2p_app::load_direct_messages(peer_id, MAX_DM_HISTORY) {
            let mut messages = VecDeque::new();
            for msg in db_messages.iter().rev() {
                let ts = p2p_app::format_peer_datetime(msg.created_at);
                let sender_display = msg
                    .peer_id
                    .as_ref()
                    .map(|p| p2p_app::peer_display_name(p, &state.local_nicknames, &state.received_nicknames))
                    .unwrap_or_else(|| state.own_nickname.clone());
                messages.push_back(format!("{} [{}] {}", ts, sender_display, msg.content));
            }
            state.dm_messages.insert(peer_id.to_string(), messages);
            let msg_count = db_messages.len();
            state.dm_scroll_state.entry(peer_id.to_string()).or_insert((msg_count, true));
            p2plog_debug(format!("Loaded {} DM messages for {}", msg_count, peer_id));
        }
    } else if !state.dm_scroll_state.contains_key(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id) {
            state.dm_scroll_state.insert(peer_id.to_string(), (msgs.len(), true));
        }
}

/// Handles peer row clicks in the Peers tab
pub fn handle_peer_row_click(state: &mut AppState, row: u16) {
    let peer_row = (row as usize).saturating_sub(3);
    if peer_row < state.peers.len()
        && let Some((peer_id, _, _)) = state.peers.get(peer_row) {
            let peer_id_clone = peer_id.clone();
            load_dm_messages(state, &peer_id_clone);
            let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with peer via mouse: {}", peer_id_clone));
        }
}

/// Handles clicks on messages in the chat view (non-DM tabs)
pub fn handle_message_click(state: &mut AppState, row: u16) {
    let click_row = row as usize;
    let mut current_row = 3;
    let mut message_idx = 0;

    for line_count in &state.chat_message_lines {
        let message_end_row = current_row + line_count;
        if click_row < message_end_row {
            break;
        }
        current_row = message_end_row;
        message_idx += 1;
    }

    if message_idx >= state.chat_message_lines.len() {
        return;
    }

    let peer_id = state.messages.iter().skip(state.chat_message_offset).nth(message_idx).map(|(_, pid)| pid.clone());

    match peer_id {
        Some(Some(sender_id)) => {
            load_dm_messages(state, &sender_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(sender_id.clone());
            state.active_tab = tab_idx;
            p2plog_debug(format!("Opened DM with sender via message click: {}", sender_id));
        }
        Some(None) => {
            state.editing_nickname = true;
            let mut textarea = TextArea::default();
            textarea.insert_str(&state.own_nickname);
            state.chat_input = textarea;
            p2plog_debug("Started editing nickname".to_string());
        }
        None => {}
    }
}

/// Handles clicks on broadcast messages in DM tab's top section
pub fn handle_dm_broadcast_message_click(state: &mut AppState, row: u16, peer_id: &str) {
    let click_row = row as usize;

    if let Some(line_counts) = state.dm_broadcast_message_lines.get(peer_id) {
        let mut current_row = 3;
        let mut message_idx_in_visible = 0;

        for line_count in line_counts {
            let message_end_row = current_row + line_count;
            if click_row < message_end_row {
                break;
            }
            current_row = message_end_row;
            message_idx_in_visible += 1;
        }

        if message_idx_in_visible >= line_counts.len() {
            return;
        }

        let effective_offset = state.dm_broadcast_offset.get(peer_id).copied().unwrap_or(0);
        let broadcast_message_idx = message_idx_in_visible + effective_offset;

        let peer_message_indices: Vec<usize> = state.messages
            .iter()
            .enumerate()
            .filter(|(_, (_, sender_id))| sender_id.as_ref().is_some_and(|id| id == peer_id))
            .map(|(idx, _)| idx)
            .collect();

        if broadcast_message_idx >= peer_message_indices.len() {
            return;
        }

        let global_idx = peer_message_indices[broadcast_message_idx];

        state.active_tab = 0;
        state.broadcast_selection = Some(global_idx);
        state.chat_auto_scroll = false;
        let offset_padding = (state.visible_message_count / 3).max(1);
        state.chat_scroll_offset = global_idx.saturating_sub(offset_padding);
        p2plog_debug(format!("Switched to Broadcast tab and scrolled to message at index {}", global_idx));
    }
}

/// Handles left mouse button clicks
pub fn handle_mouse_left_click(
    state: &mut AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_peers_tab: bool,
    is_dm_tab: bool,
    peer_id: Option<&str>,
) {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        handle_tab_click(state, mouse_column, &tab_titles);
    } else {
        let max_row = (state.chat_area_height as u16) + 2;
        if mouse_row > 2 && mouse_row < max_row {
            if is_peers_tab {
                handle_peer_row_click(state, mouse_row);
            } else if is_dm_tab && peer_id.is_some() {
                handle_dm_broadcast_message_click(state, mouse_row, peer_id.unwrap());
            } else {
                handle_message_click(state, mouse_row);
            }
        }
    }
}
