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

/// Loads DM messages from database for a peer
pub fn load_dm_messages(state: &mut AppState, peer_id: &str) {
    if !state.dm_messages.contains_key(peer_id) {
        if let Ok(db_messages) = p2p_app::load_direct_messages(peer_id, MAX_DM_HISTORY) {
            let mut messages = VecDeque::new();
            let self_nick_for_peer = state
                .self_nicknames_for_peers
                .get(peer_id)
                .cloned()
                .unwrap_or_else(|| state.own_nickname.clone());
            for msg in db_messages.iter().rev() {
                let ts = p2p_app::format_peer_datetime(msg.created_at);
                let sender_display = msg
                    .peer_id
                    .as_ref()
                    .map(|p| {
                        p2p_app::peer_display_name(
                            p,
                            &state.local_nicknames,
                            &state.received_nicknames,
                        )
                    })
                    .unwrap_or_else(|| self_nick_for_peer.clone());
                messages.push_back(format!("{} [{}] {}", ts, sender_display, msg.content));
            }
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
            p2plog_debug(format!("Loaded {} DM messages for {}", msg_count, peer_id));
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
pub fn handle_peer_row_click(state: &mut AppState, row: u16) {
    let peer_row = (row as usize).saturating_sub(3);
    if peer_row < state.peers.len()
        && let Some((peer_id, _, _)) = state.peers.get(peer_row)
    {
        let peer_id_clone = peer_id.clone();
        load_dm_messages(state, &peer_id_clone);
        let tab_idx = state.dynamic_tabs.add_dm_tab(peer_id_clone.clone());
        state.active_tab = tab_idx;
        state.cancel_nickname_edit();
        p2plog_debug(format!("Opened DM with peer via mouse: {}", peer_id_clone));
    }
}

/// Handles clicks on messages in the chat view (non-DM tabs)
pub fn handle_message_click(state: &mut AppState, row: u16, column: u16) {
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

    let global_idx = state.chat_message_offset + message_idx;

    // If the user clicked the receipt marker prefix on one of our outgoing broadcast messages, show receipt details.
    if (column as usize) <= 1
        && state
            .messages
            .get(global_idx)
            .is_some_and(|(_, pid)| pid.is_none())
        && let Some(Some(msg_id)) = state.message_ids.get(global_idx)
    {
        let sent_at = state.sent_at_by_msg_id.get(msg_id).copied();
        if let Some(map) = state.broadcast_receipts.get(msg_id) {
            if map.is_empty() {
                state.popup = Some("No peers have confirmed receipt yet.".to_string());
            } else {
                let mut nickname_counts: std::collections::HashMap<String, usize> =
                    std::collections::HashMap::new();
                for peer_id in state.peers.iter().map(|(id, _, _)| id) {
                    if let Some(nick) = state
                        .local_nicknames
                        .get(peer_id)
                        .or_else(|| state.received_nicknames.get(peer_id))
                    {
                        *nickname_counts.entry(nick.clone()).or_insert(0) += 1;
                    }
                }
                let mut peers: Vec<_> = map.iter().collect();
                peers.sort_by(|a, b| a.0.cmp(b.0));
                let mut parts = Vec::new();
                for (peer, confirmed_at) in peers {
                    let nick = state
                        .local_nicknames
                        .get(peer)
                        .or_else(|| state.received_nicknames.get(peer));
                    let peer_display = if let Some(n) = nick
                        && nickname_counts.get(n).copied().unwrap_or(0) == 1
                    {
                        n.clone()
                    } else if let Some(n) = nick {
                        format!("{} ({})", n, p2p_app::short_peer_id(peer))
                    } else {
                        p2p_app::short_peer_id(peer).to_string()
                    };
                    let ms = sent_at.map(|s| ((*confirmed_at - s) * 1000.0).max(0.0));
                    if let Some(ms) = ms {
                        parts.push(format!("{}={:.0}ms", peer_display, ms));
                    } else {
                        parts.push(format!("{}=confirmed", peer_display));
                    }
                }
                state.popup = Some(format!("Broadcast receipts:\n{}", parts.join("\n")));
            }
        } else {
            state.popup = Some("No peers have confirmed receipt yet.".to_string());
        }
        return;
    }

    let peer_id = state
        .messages
        .iter()
        .skip(state.chat_message_offset)
        .nth(message_idx)
        .map(|(_, pid)| pid.clone());

    match peer_id {
        Some(Some(sender_id)) => {
            load_dm_messages(state, &sender_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(sender_id.clone());
            state.active_tab = tab_idx;
            state.cancel_nickname_edit();
            p2plog_debug(format!(
                "Opened DM with sender via message click: {}",
                sender_id
            ));
        }
        Some(None) => {
            state.editing_nickname = true;
            state.editing_nickname_peer = None;
            let mut textarea = TextArea::default();
            textarea.insert_str(&state.own_nickname);
            state.chat_input = textarea;
            p2plog_debug("Started editing nickname".to_string());
        }
        None => {}
    }
}

fn start_peer_specific_nickname_edit(state: &mut AppState, peer_id: &str) {
    state.editing_nickname = true;
    state.editing_nickname_peer = Some(peer_id.to_string());
    let initial = state
        .self_nicknames_for_peers
        .get(peer_id)
        .cloned()
        .unwrap_or_else(|| state.own_nickname.clone());
    let mut textarea = TextArea::default();
    textarea.insert_str(&initial);
    state.chat_input = textarea;
    p2plog_debug(format!("Started editing nickname for peer {}", peer_id));
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

        let peer_message_indices: Vec<usize> = state
            .messages
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
        state.cancel_nickname_edit();
        p2plog_debug(format!(
            "Switched to Broadcast tab and scrolled to message at index {}",
            global_idx
        ));
    }
}

/// Handles clicks on DM messages in DM tab's bottom section.
pub fn handle_dm_message_click(state: &mut AppState, row: u16, column: u16, peer_id: &str) {
    let dm_area_y = state.dm_area_y.get(peer_id).copied().unwrap_or(0);
    let click_row_local = row.saturating_sub(dm_area_y) as usize;

    let Some(line_counts) = state.dm_message_lines.get(peer_id) else {
        p2plog_debug(format!(
            "DM message click ignored: no dm_message_lines for peer {} (row={})",
            peer_id, row
        ));
        return;
    };

    // Coordinates are relative to the DM pane; with borders, the first content row is 1.
    let mut current_row = 1;
    let mut message_idx_in_visible = 0;
    for line_count in line_counts {
        let message_end_row = current_row + line_count;
        if click_row_local < message_end_row {
            break;
        }
        current_row = message_end_row;
        message_idx_in_visible += 1;
    }

    if message_idx_in_visible >= line_counts.len() {
        p2plog_debug(format!(
            "DM message click ignored: click below visible msgs (peer={}, row={}, visible_count={})",
            peer_id,
            row,
            line_counts.len()
        ));
        return;
    }

    let effective_offset = state.dm_offset.get(peer_id).copied().unwrap_or(0);
    let dm_message_idx = message_idx_in_visible + effective_offset;

    let Some(msgs) = state.dm_messages.get(peer_id) else {
        p2plog_debug(format!(
            "DM message click ignored: no dm_messages for peer {}",
            peer_id
        ));
        return;
    };
    if dm_message_idx >= msgs.len() {
        p2plog_debug(format!(
            "DM message click ignored: index out of range (peer={}, idx={}, len={}, offset={}, visible_idx={})",
            peer_id,
            dm_message_idx,
            msgs.len(),
            effective_offset,
            message_idx_in_visible
        ));
        return;
    }

    let msg = &msgs[dm_message_idx];
    // Receipt marker click: show confirmation timing for our outgoing DM messages.
    if (column as usize) <= 1
        && let Some(Some(msg_id)) = state
            .dm_message_ids
            .get(peer_id)
            .and_then(|ids| ids.get(dm_message_idx))
    {
        let sent_at = state.sent_at_by_msg_id.get(msg_id).copied();
        if let Some((confirm_peer, confirmed_at)) = state.dm_receipts.get(msg_id) {
            let ms = sent_at.map(|s| ((*confirmed_at - s) * 1000.0).max(0.0));
            if let Some(ms) = ms {
                state.popup = Some(format!(
                    "DM receipt:\npeer={}\ntime={:.0}ms",
                    p2p_app::short_peer_id(confirm_peer),
                    ms
                ));
            } else {
                state.popup = Some(format!(
                    "DM receipt:\npeer={}\nconfirmed",
                    p2p_app::short_peer_id(confirm_peer)
                ));
            }
        } else {
            state.popup = Some("DM not confirmed yet.".to_string());
        }
        return;
    }
    let self_nick = state
        .self_nicknames_for_peers
        .get(peer_id)
        .cloned()
        .unwrap_or_else(|| state.own_nickname.clone());

    // Only enable nickname edit when clicking on our own message line.
    let matches_self = msg.contains(&format!("[{}] ", self_nick));
    let matches_global = msg.contains(&format!("[{}] ", state.own_nickname));
    p2plog_debug(format!(
        "DM message click: peer={} row={} local_row={} idx={} self_nick='{}' global_nick='{}' matches_self={} matches_global={}",
        peer_id,
        row,
        click_row_local,
        dm_message_idx,
        self_nick,
        state.own_nickname,
        matches_self,
        matches_global
    ));
    if matches_self || matches_global {
        start_peer_specific_nickname_edit(state, peer_id);
    } else {
        // Avoid logging full message content; just hint why nickname edit didn't start.
        let snippet: String = msg.chars().take(80).collect();
        p2plog_debug(format!(
            "DM message click not on self message (peer={}, idx={}): '{}...'",
            peer_id, dm_message_idx, snippet
        ));
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
            } else if is_dm_tab {
                if let Some(pid) = peer_id {
                    let mid_row = 2 + (state.chat_area_height / 2);
                    if (mouse_row as usize) < mid_row {
                        handle_dm_broadcast_message_click(state, mouse_row, pid);
                    } else {
                        p2plog_debug(format!(
                            "DM click routed to DM section: peer={} row={} mid_row={}",
                            pid, mouse_row, mid_row
                        ));
                        handle_dm_message_click(state, mouse_row, mouse_column, pid);
                    }
                }
            } else {
                handle_message_click(state, mouse_row, mouse_column);
            }
        }
    }
}
