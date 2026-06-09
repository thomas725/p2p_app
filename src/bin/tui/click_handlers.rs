use super::state::AppState;
use super::state::MAX_DM_HISTORY;
use p2p_app::p2plog_debug;
use std::collections::{HashMap, VecDeque};

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
                    p2plog_debug(format!("Closed DM tab via mouse: {peer_id}"));
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
pub fn format_dm_messages_from_db(
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
pub fn handle_peer_row_click(state: &mut AppState, row: u16) -> bool {
    let peer_row = (row as usize).saturating_sub(3);
    if peer_row < state.peers.len()
        && let Some(p) = state.peers.get(peer_row)
    {
        let peer_id_clone = p.peer_id.clone();
        load_dm_messages(state, &peer_id_clone);
        let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
        state.active_tab = tab_idx;
        state.cancel_nickname_edit();
        p2plog_debug(format!("Opened DM with peer via mouse: {peer_id_clone}"));
        return true;
    }
    false
}

/// Handles left mouse button clicks
pub fn handle_mouse_left_click(
    state: &mut AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_peers_tab: bool,
    _is_dm_tab: bool,
    _peer_id: Option<&str>,
) -> bool {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        return handle_tab_click(state, mouse_column, &tab_titles);
    } else if is_peers_tab {
        let max_row = (state.chat_area_height as u16) + 2;
        if mouse_row > 2 && mouse_row < max_row {
            return handle_peer_row_click(state, mouse_row);
        }
    }
    false
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_click_handlers.rs"]
mod tests;
