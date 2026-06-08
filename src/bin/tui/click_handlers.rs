use super::state::AppState;
use super::state::MAX_DM_HISTORY;
use p2p_app::PeerRecord;
use p2p_app::p2plog_debug;
use p2p_app::row_to_visible_index;
use ratatui_textarea::TextArea;
use std::collections::{HashMap, VecDeque};

fn count_nicknames<'a>(
    peers: impl Iterator<Item = &'a PeerRecord>,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for p in peers {
        if let Some(nick) = local_nicknames
            .get(&p.peer_id)
            .or_else(|| received_nicknames.get(&p.peer_id))
        {
            *counts.entry(nick.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn format_peer_display(
    peer_id: &str,
    nick: Option<&String>,
    nickname_counts: &HashMap<String, usize>,
    short_peer_id_fn: impl Fn(&str) -> String,
) -> String {
    if let Some(n) = nick {
        if nickname_counts.get(n).copied().unwrap_or(0) == 1 {
            return n.clone();
        }
        return format!("{} ({})", n, short_peer_id_fn(peer_id));
    }
    short_peer_id_fn(peer_id)
}

pub fn format_broadcast_receipt_popup_impl(
    receipts: &HashMap<String, f64>,
    peers: &VecDeque<PeerRecord>,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
    sent_at: Option<f64>,
) -> Option<String> {
    if receipts.is_empty() {
        return Some("No peers have confirmed receipt yet.".to_string());
    }

    let nickname_counts = count_nicknames(peers.iter(), local_nicknames, received_nicknames);
    let mut peer_list: Vec<_> = receipts.iter().collect();
    peer_list.sort_by(|a, b| a.0.cmp(b.0));

    let parts: Vec<String> = peer_list
        .iter()
        .map(|(peer, confirmed_at)| {
            let peer_str = peer.as_str();
            let nick = local_nicknames
                .get(peer_str)
                .or_else(|| received_nicknames.get(peer_str));
            let peer_display =
                format_peer_display(peer_str, nick, &nickname_counts, p2p_app::short_peer_id);
            let ms = sent_at.map(|s| (*confirmed_at - s) * 1000.0);
            if let Some(ms) = ms {
                format!("{}={:.0}ms", peer_display, ms.max(0.0))
            } else {
                format!("{peer_display}=confirmed")
            }
        })
        .collect();

    Some(format!("Broadcast receipts:\n{}", parts.join("\n")))
}

pub fn format_dm_receipt_popup_impl(
    confirm_peer: &str,
    confirmed_at: f64,
    sent_at: Option<f64>,
) -> String {
    let ms = sent_at.map(|s| (confirmed_at - s) * 1000.0);
    if let Some(ms) = ms {
        format!(
            "DM receipt:\npeer={}\ntime={:.0}ms",
            p2p_app::short_peer_id(confirm_peer),
            ms.max(0.0)
        )
    } else {
        format!(
            "DM receipt:\npeer={}\nconfirmed",
            p2p_app::short_peer_id(confirm_peer)
        )
    }
}

fn format_broadcast_receipt_popup(
    state: &AppState,
    msg_id: &str,
    sent_at: Option<f64>,
) -> Option<String> {
    let map = state.broadcast_receipts.get(msg_id)?;
    format_broadcast_receipt_popup_impl(
        map,
        &state.peers,
        &state.local_nicknames,
        &state.received_nicknames,
        sent_at,
    )
}

#[allow(dead_code)]
fn format_dm_receipt_popup(state: &AppState, msg_id: &str) -> Option<String> {
    let sent_at = state.sent_at_by_msg_id.get(msg_id).copied();
    let (confirm_peer, confirmed_at) = state.dm_receipts.get(msg_id)?;
    Some(format_dm_receipt_popup_impl(
        confirm_peer,
        *confirmed_at,
        sent_at,
    ))
}

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

/// Handles clicks on messages in the chat view (non-DM tabs)
pub fn handle_message_click(state: &mut AppState, row: u16, column: u16) -> bool {
    let click_row = row as usize;
    // chat_message_lines was never populated by the render loop; clicking messages is disabled.
    let Some(message_idx) = row_to_visible_index(&[], 3, click_row) else {
        return false;
    };

    let global_idx = message_idx;

    // If the user clicked the receipt marker prefix on one of our outgoing broadcast messages, show receipt details.
    if (column as usize) <= 1
        && state
            .messages
            .get(global_idx)
            .is_some_and(|dm| dm.sender_peer_id.is_none())
        && let Some(Some(msg_id)) = state.message_ids.get(global_idx)
    {
        if let Some(popup) = format_broadcast_receipt_popup(
            state,
            msg_id,
            state.sent_at_by_msg_id.get(msg_id).copied(),
        ) {
            state.popup = Some(popup);
        } else {
            state.popup = Some("No peers have confirmed receipt yet.".to_string());
        }
        return true;
    }

    let peer_id = state
        .messages
        .get(message_idx)
        .map(|dm| dm.sender_peer_id.clone());

    match peer_id {
        Some(Some(sender_id)) => {
            load_dm_messages(state, &sender_id);
            let tab_idx = state.dynamic_tabs.add_dm_tab(sender_id.clone());
            state.active_tab = tab_idx;
            state.cancel_nickname_edit();
            p2plog_debug(format!(
                "Opened DM with sender via message click: {sender_id}"
            ));
            true
        }
        Some(None) => {
            state.editing_nickname = true;
            state.editing_nickname_peer = None;
            let mut textarea = TextArea::default();
            textarea.insert_str(&state.own_nickname);
            state.chat_input = textarea;
            p2plog_debug("Started editing nickname".to_string());
            true
        }
        None => false,
    }
}

#[allow(dead_code)]
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
    p2plog_debug(format!("Started editing nickname for peer {peer_id}"));
}

/// Handles clicks on broadcast messages in DM tab's top section
pub fn handle_dm_broadcast_message_click(_state: &mut AppState, _row: u16, _peer_id: &str) -> bool {
    false
}

/// Handles clicks on DM messages in DM tab's bottom section.
pub fn handle_dm_message_click(
    _state: &mut AppState,
    _row: u16,
    _column: u16,
    peer_id: &str,
) -> bool {
    p2plog_debug(format!(
        "DM message click ignored: dm_message_lines was never populated (peer={peer_id})"
    ));
    false
}

/// Handles left mouse button clicks
pub fn handle_mouse_left_click(
    state: &mut AppState,
    mouse_row: u16,
    mouse_column: u16,
    is_peers_tab: bool,
    is_dm_tab: bool,
    peer_id: Option<&str>,
) -> bool {
    if mouse_row == 0 {
        let tab_titles = state.dynamic_tabs.all_titles();
        return handle_tab_click(state, mouse_column, &tab_titles);
    } else {
        let max_row = (state.chat_area_height as u16) + 2;
        if mouse_row > 2 && mouse_row < max_row {
            if is_peers_tab {
                return handle_peer_row_click(state, mouse_row);
            } else if is_dm_tab {
                if let Some(pid) = peer_id {
                    let mid_row = 2 + (state.chat_area_height / 2);
                    if (mouse_row as usize) < mid_row {
                        return handle_dm_broadcast_message_click(state, mouse_row, pid);
                    } else {
                        p2plog_debug(format!(
                            "DM click routed to DM section: peer={pid} row={mouse_row} mid_row={mid_row}"
                        ));
                        return handle_dm_message_click(state, mouse_row, mouse_column, pid);
                    }
                }
            } else {
                return handle_message_click(state, mouse_row, mouse_column);
            }
        }
    }
    false
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_click_handlers.rs"]
mod tests;
