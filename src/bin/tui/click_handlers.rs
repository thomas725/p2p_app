use super::constants::MAX_DM_HISTORY;
use super::state::AppState;
use p2p_app::p2plog_debug;
use p2p_app::row_to_visible_index;
use ratatui_textarea::TextArea;
use std::collections::{HashMap, VecDeque};

fn count_nicknames<'a>(
    peers: impl Iterator<Item = &'a (String, String, String)>,
    local_nicknames: &HashMap<String, String>,
    received_nicknames: &HashMap<String, String>,
) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for (peer_id, _, _) in peers {
        if let Some(nick) = local_nicknames
            .get(peer_id)
            .or_else(|| received_nicknames.get(peer_id))
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
    peers: &VecDeque<(String, String, String)>,
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
                let sender_display = msg.peer_id.as_ref().map_or_else(
                    || self_nick_for_peer.clone(),
                    |p| {
                        p2p_app::peer_display_name(
                            p,
                            &state.local_nicknames,
                            &state.received_nicknames,
                        )
                    },
                );
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
        p2plog_debug(format!("Opened DM with peer via mouse: {peer_id_clone}"));
    }
}

/// Handles clicks on messages in the chat view (non-DM tabs)
pub fn handle_message_click(state: &mut AppState, row: u16, column: u16) {
    let click_row = row as usize;
    let Some(message_idx) = row_to_visible_index(&state.chat_message_lines, 3, click_row) else {
        return;
    };

    let global_idx = state.chat_message_offset + message_idx;

    // If the user clicked the receipt marker prefix on one of our outgoing broadcast messages, show receipt details.
    if (column as usize) <= 1
        && state
            .messages
            .get(global_idx)
            .is_some_and(|(_, pid)| pid.is_none())
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
                "Opened DM with sender via message click: {sender_id}"
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
    p2plog_debug(format!("Started editing nickname for peer {peer_id}"));
}

/// Handles clicks on broadcast messages in DM tab's top section
pub fn handle_dm_broadcast_message_click(state: &mut AppState, row: u16, peer_id: &str) {
    let click_row = row as usize;

    if let Some(line_counts) = state.dm_broadcast_message_lines.get(peer_id) {
        let Some(message_idx_in_visible) = row_to_visible_index(line_counts, 3, click_row) else {
            return;
        };

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
            "Switched to Broadcast tab and scrolled to message at index {global_idx}"
        ));
    }
}

/// Handles clicks on DM messages in DM tab's bottom section.
pub fn handle_dm_message_click(state: &mut AppState, row: u16, column: u16, peer_id: &str) {
    let dm_area_y = state.dm_area_y.get(peer_id).copied().unwrap_or(0);
    let click_row_local = row.saturating_sub(dm_area_y) as usize;

    let Some(line_counts) = state.dm_message_lines.get(peer_id) else {
        p2plog_debug(format!(
            "DM message click ignored: no dm_message_lines for peer {peer_id} (row={row})"
        ));
        return;
    };

    let Some(message_idx_in_visible) = row_to_visible_index(line_counts, 1, click_row_local) else {
        p2plog_debug(format!(
            "DM message click ignored: click below visible msgs (peer={}, row={}, visible_count={})",
            peer_id,
            row,
            line_counts.len()
        ));
        return;
    };

    let effective_offset = state.dm_offset.get(peer_id).copied().unwrap_or(0);
    let dm_message_idx = message_idx_in_visible + effective_offset;

    let Some(msgs) = state.dm_messages.get(peer_id) else {
        p2plog_debug(format!(
            "DM message click ignored: no dm_messages for peer {peer_id}"
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
        if let Some(popup) = format_dm_receipt_popup(state, msg_id) {
            state.popup = Some(popup);
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
    let matches_self = msg.contains(&format!("[{self_nick}] "));
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
            "DM message click not on self message (peer={peer_id}, idx={dm_message_idx}): '{snippet}...'"
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
                            "DM click routed to DM section: peer={pid} row={mouse_row} mid_row={mid_row}"
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

#[cfg(test)]
mod tests {
    use super::{handle_dm_broadcast_message_click, handle_dm_message_click, handle_message_click};
    use crate::tui::state::AppState;
    use std::collections::{HashMap, VecDeque};

    fn empty_state() -> AppState {
        AppState::new(
            "topic".to_string(),
            "me".to_string(),
            "local-peer".to_string(),
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            VecDeque::new(),
            VecDeque::new(),
            HashMap::new(),
            VecDeque::new(),
            HashMap::new(),
            HashMap::new(),
        )
    }

    #[test]
    fn message_click_on_broadcast_receipt_prefix_opens_popup() {
        let mut state = empty_state();
        state.messages.push_back(("hello".to_string(), None));
        state.message_ids.push_back(Some("msg-1".to_string()));
        state.chat_message_lines = vec![1];
        state.chat_message_offset = 0;
        state.broadcast_receipts.insert(
            "msg-1".to_string(),
            HashMap::from([("peer-1".to_string(), 2.0)]),
        );
        state.sent_at_by_msg_id.insert("msg-1".to_string(), 1.0);

        handle_message_click(&mut state, 3, 0);

        assert_eq!(
            state.popup.as_deref(),
            Some("Broadcast receipts:\npeer-1=1000ms")
        );
    }

    #[test]
    fn dm_broadcast_click_selects_original_broadcast_message() {
        let mut state = empty_state();
        state.messages = VecDeque::from([
            ("from peer".to_string(), Some("peer-1".to_string())),
            ("other".to_string(), Some("peer-2".to_string())),
        ]);
        state.visible_message_count = 6;
        state.chat_auto_scroll = true;
        state.active_tab = 2;
        state
            .dm_broadcast_message_lines
            .insert("peer-1".to_string(), vec![1]);
        state.dm_broadcast_offset.insert("peer-1".to_string(), 0);

        handle_dm_broadcast_message_click(&mut state, 3, "peer-1");

        assert_eq!(state.active_tab, 0);
        assert_eq!(state.broadcast_selection, Some(0));
        assert!(!state.chat_auto_scroll);
        assert_eq!(state.chat_scroll_offset, 0);
    }

    #[test]
    fn dm_message_click_on_receipt_prefix_opens_popup() {
        let mut state = empty_state();
        state.dm_messages.insert(
            "peer-1".to_string(),
            VecDeque::from(["[me] hi".to_string()]),
        );
        state.dm_message_ids.insert(
            "peer-1".to_string(),
            VecDeque::from([Some("dm-1".to_string())]),
        );
        state
            .dm_receipts
            .insert("dm-1".to_string(), ("peer-1".to_string(), 2.5));
        state.sent_at_by_msg_id.insert("dm-1".to_string(), 1.0);
        state.dm_message_lines.insert("peer-1".to_string(), vec![1]);
        state.dm_area_y.insert("peer-1".to_string(), 10);
        state.dm_offset.insert("peer-1".to_string(), 0);

        handle_dm_message_click(&mut state, 11, 0, "peer-1");

        assert_eq!(
            state.popup.as_deref(),
            Some("DM receipt:\npeer=peer-1\ntime=1500ms")
        );
    }

    #[test]
    fn test_count_nicknames() {
        let peers: VecDeque<_> = VecDeque::from(vec![
            (
                "peer1".to_string(),
                "seen1".to_string(),
                "last1".to_string(),
            ),
            (
                "peer2".to_string(),
                "seen2".to_string(),
                "last2".to_string(),
            ),
        ]);
        let local = HashMap::from([("peer1".to_string(), "Alice".to_string())]);
        let received = HashMap::from([("peer2".to_string(), "Bob".to_string())]);

        let counts = super::count_nicknames(peers.iter(), &local, &received);
        assert_eq!(counts.get("Alice"), Some(&1));
        assert_eq!(counts.get("Bob"), Some(&1));
    }

    #[test]
    fn test_count_nicknames_duplicates() {
        let peers: VecDeque<_> = VecDeque::from(vec![
            (
                "peer1".to_string(),
                "seen1".to_string(),
                "last1".to_string(),
            ),
            (
                "peer2".to_string(),
                "seen2".to_string(),
                "last2".to_string(),
            ),
        ]);
        let local = HashMap::from([
            ("peer1".to_string(), "Alice".to_string()),
            ("peer2".to_string(), "Alice".to_string()),
        ]);
        let received = HashMap::new();

        let counts = super::count_nicknames(peers.iter(), &local, &received);
        assert_eq!(counts.get("Alice"), Some(&2));
    }

    #[test]
    fn test_format_peer_display_with_nickname_unique() {
        let counts = HashMap::from([("Alice".to_string(), 1usize)]);
        let result =
            super::format_peer_display("peer1", Some(&"Alice".to_string()), &counts, |id| {
                id.chars().rev().take(8).collect()
            });
        assert_eq!(result, "Alice");
    }

    #[test]
    fn test_format_peer_display_with_nickname_duplicate() {
        let counts = HashMap::from([("Alice".to_string(), 2usize)]);
        let result =
            super::format_peer_display("peer1", Some(&"Alice".to_string()), &counts, |id| {
                id.chars().rev().take(8).collect()
            });
        assert!(result.contains("Alice"));
        // The short_peer_id function reverses and takes last 8 chars, so "peer1" -> "1reep"
        assert!(result.contains("1reep"));
    }

    #[test]
    fn test_format_peer_display_no_nickname() {
        let counts = HashMap::new();
        let result = super::format_peer_display("peer1", None, &counts, |id| {
            id.chars().rev().take(8).collect()
        });
        // The short_peer_id function reverses and takes last 8 chars
        assert_eq!(result, "1reep");
    }

    #[test]
    fn test_format_broadcast_receipt_popup_impl_empty() {
        let receipts = HashMap::new();
        let peers: VecDeque<(String, String, String)> = VecDeque::new();
        let local = HashMap::new();
        let received = HashMap::new();
        let result =
            super::format_broadcast_receipt_popup_impl(&receipts, &peers, &local, &received, None);
        assert_eq!(
            result,
            Some("No peers have confirmed receipt yet.".to_string())
        );
    }

    #[test]
    fn test_format_broadcast_receipt_popup_impl_with_data() {
        let receipts = HashMap::from([("peer1".to_string(), 2.0), ("peer2".to_string(), 3.0)]);
        let peers: VecDeque<_> = VecDeque::from(vec![
            ("peer1".to_string(), "s1".to_string(), "l1".to_string()),
            ("peer2".to_string(), "s2".to_string(), "l2".to_string()),
        ]);
        let local = HashMap::new();
        let received = HashMap::new();
        let result = super::format_broadcast_receipt_popup_impl(
            &receipts,
            &peers,
            &local,
            &received,
            Some(1.0),
        );
        assert!(result.is_some());
        let s = result.unwrap();
        assert!(s.contains("peer1"));
        assert!(s.contains("peer2"));
        assert!(s.contains("1000ms"));
    }

    #[test]
    fn test_format_dm_receipt_popup_impl_with_time() {
        let result = super::format_dm_receipt_popup_impl("peer1", 2.0, Some(1.0));
        assert!(result.contains("peer1"));
        assert!(result.contains("1000ms"));
    }

    #[test]
    fn test_format_dm_receipt_popup_impl_confirmed() {
        let result = super::format_dm_receipt_popup_impl("peer1", 2.0, None);
        assert!(result.contains("peer1"));
        assert!(result.contains("confirmed"));
    }
}
