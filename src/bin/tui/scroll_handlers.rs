use super::constants::WHEEL_SCROLL_LINES;
use super::state::AppState;
use p2p_app::get_tui_logs;
use p2p_app::p2plog_debug;
use p2p_app::tui_helpers::key_code_to_scroll_action;

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
    let broadcast_messages: Vec<(String, Option<String>)> = state
        .messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
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

/// Handle mouse wheel for broadcast section of DM tab
fn mouse_scroll_broadcast_section(state: &mut AppState, scroll_dir: &str, peer_id: &str) {
    let broadcast_messages: Vec<(String, Option<String>)> = state
        .messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .cloned()
        .collect();

    if broadcast_messages.is_empty() {
        return;
    }

    if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
        // If auto-scroll is enabled, do nothing (user is viewing latest messages)
        if *auto_scroll {
            return;
        }
        let max_offset = broadcast_messages.len().saturating_sub(1);
        match scroll_dir {
            "up" => {
                if *scroll_offset >= WHEEL_SCROLL_LINES {
                    *scroll_offset -= WHEEL_SCROLL_LINES;
                } else {
                    *scroll_offset = 0;
                }
            }
            "down" => {
                *scroll_offset = (*scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
            }
            _ => {}
        }
    }
}

/// Handle mouse wheel for DM section of DM tab
fn mouse_scroll_dm_section(state: &mut AppState, scroll_dir: &str, peer_id: &str) {
    if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id)
    {
        // If auto-scroll is enabled, do nothing (user is viewing latest messages)
        if *auto_scroll {
            return;
        }
        let max_offset = msgs.len().saturating_sub(1);
        match scroll_dir {
            "up" => {
                if *scroll_offset >= WHEEL_SCROLL_LINES {
                    *scroll_offset -= WHEEL_SCROLL_LINES;
                } else {
                    *scroll_offset = 0;
                }
            }
            "down" => {
                *scroll_offset = (*scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
            }
            _ => {}
        }
    }
}

/// Handle mouse wheel for Chat tab (broadcast)
fn mouse_scroll_chat_tab(state: &mut AppState, scroll_dir: &str) {
    if state.chat_auto_scroll {
        return;
    }
    let max_offset = state
        .messages
        .len()
        .saturating_sub(state.visible_message_count);
    state.chat_scroll_offset = p2p_app::tui_helpers::handle_mouse_wheel_scroll(
        scroll_dir,
        state.chat_scroll_offset,
        max_offset,
    );
}

/// Handle mouse wheel for Log tab
fn mouse_scroll_log_tab(state: &mut AppState, scroll_dir: &str) {
    // If auto-scroll is enabled, do nothing (user is viewing latest logs)
    if state.log_auto_scroll {
        return;
    }
    let max_offset = get_tui_logs().len().saturating_sub(state.visible_log_count);
    match scroll_dir {
        "up" => {
            if state.log_scroll_offset >= WHEEL_SCROLL_LINES {
                state.log_scroll_offset -= WHEEL_SCROLL_LINES;
            } else {
                state.log_scroll_offset = 0;
            }
        }
        "down" => {
            state.log_scroll_offset =
                (state.log_scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
        }
        _ => {}
    }
}

/// Handles mouse wheel scrolling with hover-based section targeting for split DM tabs
pub fn handle_mouse_scroll(state: &mut AppState, scroll_dir: &str, _peer_id: Option<&str>) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);

    match &tab_content {
        p2p_app::tui_tabs::TabContent::Direct(peer_id) => {
            let mid_row = 2 + (state.chat_area_height / 2);
            let mouse_row = state.last_mouse_row as usize;
            if mouse_row < mid_row {
                mouse_scroll_broadcast_section(state, scroll_dir, peer_id);
            } else {
                mouse_scroll_dm_section(state, scroll_dir, peer_id);
            }
        }
        p2p_app::tui_tabs::TabContent::Log => {
            mouse_scroll_log_tab(state, scroll_dir);
        }
        _ => {
            mouse_scroll_chat_tab(state, scroll_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::AppState;
    use crate::tui::test_helpers::{
        app_state_with_chat_messages, app_state_with_dm_messages, app_state_with_peers,
        test_app_state,
    };
    use crossterm::event::KeyCode;

    // ── handle_navigation_key ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_nav_tab_cycles_forward() {
        let mut state = test_app_state();
        assert_eq!(state.active_tab, 0);
        handle_navigation_key(KeyCode::Tab, &mut state).await;
        assert_eq!(state.active_tab, 1);
        handle_navigation_key(KeyCode::Tab, &mut state).await;
        assert_eq!(state.active_tab, 2);
    }

    #[tokio::test]
    async fn test_nav_tab_wraps_to_first() {
        let mut state = test_app_state();
        state.active_tab = state.dynamic_tabs.total_tab_count() - 1;
        handle_navigation_key(KeyCode::Tab, &mut state).await;
        assert_eq!(state.active_tab, 0);
    }

    #[tokio::test]
    async fn test_nav_back_tab_cycles_backward() {
        let mut state = test_app_state();
        state.active_tab = 2;
        handle_navigation_key(KeyCode::BackTab, &mut state).await;
        assert_eq!(state.active_tab, 1);
    }

    #[tokio::test]
    async fn test_nav_back_tab_wraps_to_last() {
        let mut state = test_app_state();
        handle_navigation_key(KeyCode::BackTab, &mut state).await;
        assert_eq!(state.active_tab, state.dynamic_tabs.total_tab_count() - 1);
    }

    #[tokio::test]
    async fn test_nav_resets_scroll_offset() {
        let mut state = test_app_state();
        state.chat_scroll_offset = 5;
        handle_navigation_key(KeyCode::Tab, &mut state).await;
        assert_eq!(state.chat_scroll_offset, 0);
    }

    #[tokio::test]
    async fn test_nav_cancels_nickname_edit() {
        let mut state = test_app_state();
        state.editing_nickname = true;
        handle_navigation_key(KeyCode::Tab, &mut state).await;
        assert!(!state.editing_nickname);
    }

    // ── scroll_peers_tab ──────────────────────────────────────────────────────

    #[test]
    fn test_scroll_peers_down() {
        let mut state = app_state_with_peers(3);
        scroll_peers_tab(KeyCode::Down, &mut state);
        assert_eq!(state.peer_selection, 1);
    }

    #[test]
    fn test_scroll_peers_up() {
        let mut state = app_state_with_peers(3);
        state.peer_selection = 2;
        scroll_peers_tab(KeyCode::Up, &mut state);
        assert_eq!(state.peer_selection, 1);
    }

    #[test]
    fn test_scroll_peers_no_overflow_below_zero() {
        let mut state = app_state_with_peers(3);
        scroll_peers_tab(KeyCode::Up, &mut state);
        assert_eq!(state.peer_selection, 0);
    }

    #[test]
    fn test_scroll_peers_no_overflow_above_max() {
        let mut state = app_state_with_peers(3);
        for _ in 0..5 {
            scroll_peers_tab(KeyCode::Down, &mut state);
        }
        assert_eq!(state.peer_selection, 2);
    }

    #[test]
    fn test_scroll_peers_noop_on_other_keys() {
        let mut state = app_state_with_peers(3);
        scroll_peers_tab(KeyCode::Home, &mut state);
        assert_eq!(state.peer_selection, 0);
    }

    #[test]
    fn test_scroll_peers_empty_list() {
        let mut state = test_app_state();
        scroll_peers_tab(KeyCode::Down, &mut state);
        assert_eq!(state.peer_selection, 0);
    }

    // ── compute_new_peer_selection ─────────────────────────────────────────────

    #[test]
    fn test_compute_peer_selection_up() {
        assert_eq!(compute_new_peer_selection(KeyCode::Up, 5, 10), 4);
    }

    #[test]
    fn test_compute_peer_selection_up_clamps_at_zero() {
        assert_eq!(compute_new_peer_selection(KeyCode::Up, 0, 10), 0);
    }

    #[test]
    fn test_compute_peer_selection_down() {
        assert_eq!(compute_new_peer_selection(KeyCode::Down, 0, 10), 1);
    }

    #[test]
    fn test_compute_peer_selection_down_clamps_at_max() {
        assert_eq!(compute_new_peer_selection(KeyCode::Down, 9, 10), 9);
    }

    #[test]
    fn test_compute_peer_selection_down_on_empty_list() {
        assert_eq!(compute_new_peer_selection(KeyCode::Down, 0, 0), 0);
    }

    #[test]
    fn test_compute_peer_selection_other_key_noop() {
        assert_eq!(compute_new_peer_selection(KeyCode::Home, 3, 10), 3);
    }

    // ── scroll_chat_tab ─────────────────────────────────────────────────────

    #[test]
    fn test_scroll_chat_down() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        scroll_chat_tab(KeyCode::Down, &mut state);
        assert_eq!(state.chat_scroll_offset, 1);
        assert!(!state.chat_auto_scroll);
    }

    #[test]
    fn test_scroll_chat_up() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        state.chat_scroll_offset = 3;
        scroll_chat_tab(KeyCode::Up, &mut state);
        assert_eq!(state.chat_scroll_offset, 2);
    }

    #[test]
    fn test_scroll_chat_no_scroll_when_no_messages() {
        let mut state = test_app_state();
        scroll_chat_tab(KeyCode::Down, &mut state);
        assert_eq!(state.chat_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_chat_pgdn() {
        let mut state = app_state_with_chat_messages(20);
        state.chat_auto_scroll = false;
        // PAGE_SIZE = 8
        scroll_chat_tab(KeyCode::PageDown, &mut state);
        assert_eq!(state.chat_scroll_offset, 8);
    }

    #[test]
    fn test_scroll_chat_home() {
        let mut state = app_state_with_chat_messages(20);
        state.chat_scroll_offset = 10;
        scroll_chat_tab(KeyCode::Home, &mut state);
        assert_eq!(state.chat_scroll_offset, 0);
    }

    #[test]
    fn test_scroll_chat_end() {
        let mut state = app_state_with_chat_messages(20);
        scroll_chat_tab(KeyCode::End, &mut state);
        assert_eq!(state.chat_scroll_offset, 15);
    }

    #[test]
    fn test_scroll_chat_auto_scroll_stays_at_bottom_on_down() {
        let mut state = app_state_with_chat_messages(10);
        assert!(state.chat_auto_scroll);
        scroll_chat_tab(KeyCode::Down, &mut state);
        // auto_scroll = true -> Down: disable_auto sets offset=max, then scroll down keeps at max
        assert_eq!(state.chat_scroll_offset, 5);
        assert!(state.chat_auto_scroll);
    }

    #[test]
    fn test_scroll_chat_up_disables_auto_scroll() {
        let mut state = app_state_with_chat_messages(10);
        assert!(state.chat_auto_scroll);
        scroll_chat_tab(KeyCode::Up, &mut state);
        // Up from auto-scrolled: disable_auto sets offset=max=5, auto=false; then scroll_up to 4
        assert_eq!(state.chat_scroll_offset, 4);
        assert!(!state.chat_auto_scroll);
    }

    // ── scroll_log_tab ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_log_scroll_all() {
        // All log tests in one function because get_tui_logs() is global state shared across tests
        p2p_app::init_logging(); // ensure TUI_LOGS OnceLock is initialized
        p2p_app::logging::clear_tui_logs();

        // scroll_log_tab: down
        let mut state = test_app_state();
        let pre_len = p2p_app::get_tui_logs().len();
        p2p_app::push_log("a".to_string());
        p2p_app::push_log("b".to_string());
        p2p_app::push_log("c".to_string());
        let log_len = p2p_app::get_tui_logs().len();
        assert!(log_len >= pre_len + 3);
        state.log_auto_scroll = false;
        scroll_log_tab(KeyCode::Down, &mut state);
        assert_eq!(state.log_scroll_offset, 1);
        // up
        scroll_log_tab(KeyCode::Up, &mut state);
        assert_eq!(state.log_scroll_offset, 0);

        // mouse_scroll_log: auto_scroll blocks, then manual down
        let mut state2 = test_app_state();
        p2p_app::push_log("log".to_string());
        state2.log_auto_scroll = true;
        mouse_scroll_log_tab(&mut state2, "up");
        assert_eq!(state2.log_scroll_offset, 0);

        for i in 0..10 {
            p2p_app::push_log(format!("log {i}"));
        }
        state2.log_auto_scroll = false;
        mouse_scroll_log_tab(&mut state2, "down");
        assert_eq!(state2.log_scroll_offset, 3);

        // handle_scroll_key dispatch to log tab
        let mut state3 = test_app_state();
        p2p_app::push_log("log entry".to_string());
        state3.log_auto_scroll = false;
        state3.active_tab = 2;
        handle_scroll_key(KeyCode::Down, &mut state3).await;
        assert_eq!(state3.log_scroll_offset, 1);

        // handle_mouse_scroll dispatch to log tab
        let mut state4 = test_app_state();
        for i in 0..10 {
            p2p_app::push_log(format!("log {i}"));
        }
        state4.active_tab = 2;
        state4.log_auto_scroll = false;
        handle_mouse_scroll(&mut state4, "down", None);
        assert_eq!(state4.log_scroll_offset, 3);
    }

    // ── mouse_scroll_chat_tab ────────────────────────────────────────────────

    #[test]
    fn test_mouse_scroll_chat_up_disables_auto_scroll() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        mouse_scroll_chat_tab(&mut state, "up");
        assert!(!state.chat_auto_scroll);
    }

    #[test]
    fn test_mouse_scroll_chat_auto_scroll_blocks() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = true;
        mouse_scroll_chat_tab(&mut state, "up");
        assert_eq!(state.chat_scroll_offset, 0);
    }

    #[test]
    fn test_mouse_scroll_chat_down() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        mouse_scroll_chat_tab(&mut state, "down");
        assert_eq!(state.chat_scroll_offset, 3);
    }

    #[test]
    fn test_mouse_scroll_chat_down_clamps() {
        let mut state = app_state_with_chat_messages(3);
        state.chat_auto_scroll = false;
        // max offset = 3 - 5 = 0 (saturating)
        mouse_scroll_chat_tab(&mut state, "down");
        assert_eq!(state.chat_scroll_offset, 0);
    }

    // (log tests consolidated in test_log_scroll_all above — avoids parallel global state conflicts)

    // ── DM tab scroll helpers ────────────────────────────────────────────────

    /// Helper: create state with multiple messages from the same peer for broadcast section testing
    fn state_with_broadcasts_from_peer(peer_id: &str, count: usize) -> AppState {
        let mut state = test_app_state();
        for i in 0..count {
            state
                .messages
                .push_back((format!("msg {i}"), Some(peer_id.to_string())));
            state.message_ids.push_back(Some(format!("id-{i}")));
        }
        state
    }

    #[test]
    fn test_scroll_broadcast_section_down() {
        let mut state = state_with_broadcasts_from_peer("peer-b", 10);
        state
            .dm_broadcast_scroll_state
            .insert("peer-b".to_string(), (0, false));
        state.dm_visible_counts.insert("peer-b".to_string(), (5, 5));
        scroll_broadcast_section(KeyCode::Down, &mut state, "peer-b");
        let (offset, auto) = state.dm_broadcast_scroll_state.get("peer-b").unwrap();
        assert_eq!(*offset, 1);
        assert!(!auto);
    }

    #[test]
    fn test_scroll_dm_section_down() {
        let mut state = app_state_with_dm_messages("peer-dm", 10);
        state
            .dm_visible_counts
            .insert("peer-dm".to_string(), (5, 5));
        state
            .dm_scroll_state
            .insert("peer-dm".to_string(), (0, false));
        scroll_dm_section(KeyCode::Down, &mut state, "peer-dm");
        let (offset, auto) = state.dm_scroll_state.get("peer-dm").unwrap();
        assert_eq!(*offset, 1);
        assert!(!auto);
    }

    #[test]
    fn test_scroll_broadcast_section_noop_when_empty() {
        let mut state = test_app_state();
        let peer_id = "peer-empty";
        scroll_broadcast_section(KeyCode::Down, &mut state, peer_id);
        // no panic, no crash
    }

    #[test]
    fn test_scroll_dm_section_noop_when_no_state() {
        let mut state = test_app_state();
        scroll_dm_section(KeyCode::Down, &mut state, "peer-none");
        // no panic, no crash
    }

    // ── Mouse scroll DM helpers ──────────────────────────────────────────────

    #[test]
    fn test_mouse_scroll_broadcast_section_up() {
        let mut state = state_with_broadcasts_from_peer("peer-b", 10);
        state
            .dm_broadcast_scroll_state
            .insert("peer-b".to_string(), (5, false));
        mouse_scroll_broadcast_section(&mut state, "up", "peer-b");
        let (offset, _) = state.dm_broadcast_scroll_state.get("peer-b").unwrap();
        assert_eq!(*offset, 2);
    }

    #[test]
    fn test_mouse_scroll_broadcast_section_auto_scroll_blocks() {
        let mut state = state_with_broadcasts_from_peer("peer-b", 10);
        state
            .dm_broadcast_scroll_state
            .insert("peer-b".to_string(), (0, true));
        mouse_scroll_broadcast_section(&mut state, "up", "peer-b");
        let (offset, _) = state.dm_broadcast_scroll_state.get("peer-b").unwrap();
        assert_eq!(*offset, 0);
    }

    #[test]
    fn test_mouse_scroll_dm_section_auto_scroll_blocks() {
        let mut state = app_state_with_dm_messages("peer-dm", 10);
        state
            .dm_scroll_state
            .insert("peer-dm".to_string(), (0, true));
        mouse_scroll_dm_section(&mut state, "up", "peer-dm");
        let (offset, _) = state.dm_scroll_state.get("peer-dm").unwrap();
        assert_eq!(*offset, 0);
    }

    #[test]
    fn test_mouse_scroll_broadcast_section_noop_when_empty() {
        let mut state = test_app_state();
        mouse_scroll_broadcast_section(&mut state, "up", "peer-empty");
    }

    #[test]
    fn test_mouse_scroll_dm_section_noop_when_no_state() {
        let mut state = test_app_state();
        mouse_scroll_dm_section(&mut state, "up", "peer-none");
    }

    // ── handle_scroll_key dispatch ──────────────────────────────────────────

    #[tokio::test]
    async fn test_handle_scroll_key_chat_tab() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        state.active_tab = 0;
        handle_scroll_key(KeyCode::Down, &mut state).await;
        assert_eq!(state.chat_scroll_offset, 1);
    }

    #[tokio::test]
    async fn test_handle_scroll_key_peers_tab() {
        let mut state = app_state_with_peers(5);
        state.active_tab = 1;
        handle_scroll_key(KeyCode::Down, &mut state).await;
        assert_eq!(state.peer_selection, 1);
    }

    // (log tab dispatch tested in test_log_scroll_all above)

    // ── handle_mouse_scroll dispatch ─────────────────────────────────────────

    #[test]
    fn test_handle_mouse_scroll_chat_tab() {
        let mut state = app_state_with_chat_messages(10);
        state.chat_auto_scroll = false;
        handle_mouse_scroll(&mut state, "down", None);
        assert_eq!(state.chat_scroll_offset, 3);
    }
}
