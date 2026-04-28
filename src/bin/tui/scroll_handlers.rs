use super::constants::{PAGE_SIZE, WHEEL_SCROLL_LINES};
use super::state::AppState;
use p2p_app::p2plog_debug;
use p2p_app::get_tui_logs;

/// Handles tab navigation (Tab and BackTab keys)
pub async fn handle_navigation_key(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    match key_code {
        crossterm::event::KeyCode::Tab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = (state.active_tab + 1) % max_tabs;
            state.chat_scroll_offset = 0;
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        crossterm::event::KeyCode::BackTab => {
            let max_tabs = state.dynamic_tabs.total_tab_count();
            state.active_tab = if state.active_tab == 0 { max_tabs - 1 } else { state.active_tab - 1 };
            state.chat_scroll_offset = 0;
            p2plog_debug(format!("Switched to tab {}", state.active_tab));
        }
        _ => {}
    }
}

/// Disables auto-scroll and sets offset to max if auto-scroll was enabled
fn disable_auto_scroll_to_max(auto_scroll: &mut bool, scroll_offset: &mut usize, max_offset: usize) {
    if *auto_scroll {
        *auto_scroll = false;
        *scroll_offset = max_offset;
    }
}

/// Scroll up by one line or page
fn scroll_up_lines(scroll_offset: &mut usize, lines: usize) {
    *scroll_offset = scroll_offset.saturating_sub(lines);
}

/// Scroll down to target, enabling auto-scroll if reaching max
fn scroll_down_lines(scroll_offset: &mut usize, auto_scroll: &mut bool, lines: usize, max_offset: usize) {
    *scroll_offset = (*scroll_offset + lines).min(max_offset);
    if *scroll_offset >= max_offset {
        *auto_scroll = true;
    }
}

/// Handle a single scroll key press for broadcast/DM section with mutable state
fn handle_scroll_key_for_section(
    key_code: crossterm::event::KeyCode,
    scroll_offset: &mut usize,
    auto_scroll: &mut bool,
    max_offset: usize,
) {
    match key_code {
        crossterm::event::KeyCode::Up => {
            disable_auto_scroll_to_max(auto_scroll, scroll_offset, max_offset);
            scroll_up_lines(scroll_offset, 1);
        }
        crossterm::event::KeyCode::Down => {
            disable_auto_scroll_to_max(auto_scroll, scroll_offset, max_offset);
            scroll_down_lines(scroll_offset, auto_scroll, 1, max_offset);
        }
        crossterm::event::KeyCode::PageUp => {
            disable_auto_scroll_to_max(auto_scroll, scroll_offset, max_offset);
            scroll_up_lines(scroll_offset, PAGE_SIZE);
        }
        crossterm::event::KeyCode::PageDown => {
            disable_auto_scroll_to_max(auto_scroll, scroll_offset, max_offset);
            scroll_down_lines(scroll_offset, auto_scroll, PAGE_SIZE, max_offset);
        }
        crossterm::event::KeyCode::Home => {
            *auto_scroll = false;
            *scroll_offset = 0;
        }
        crossterm::event::KeyCode::End => {
            *auto_scroll = true;
        }
        _ => {}
    }
}

/// Handle scroll key for broadcast section of DM tab
fn scroll_broadcast_section(key_code: crossterm::event::KeyCode, state: &mut AppState, peer_id: &str) {
    let broadcast_messages: Vec<(String, Option<String>)> = state.messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .cloned()
        .collect();

    if broadcast_messages.is_empty() {
        return;
    }

    if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
        let visible_count = state.dm_visible_counts.get(peer_id).map(|(b, _)| *b).unwrap_or(1);
        let max_offset = broadcast_messages.len().saturating_sub(visible_count);
        handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
    }
}

/// Handle scroll key for DM section of DM tab
fn scroll_dm_section(key_code: crossterm::event::KeyCode, state: &mut AppState, peer_id: &str) {
    if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id) {
            let visible_count = state.dm_visible_counts.get(peer_id).map(|(_, d)| *d).unwrap_or(1);
            let max_offset = msgs.len().saturating_sub(visible_count);
            handle_scroll_key_for_section(key_code, scroll_offset, auto_scroll, max_offset);
        }
}

/// Handle scroll key for Chat tab (broadcast)
fn scroll_chat_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
    match key_code {
        crossterm::event::KeyCode::Up => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            }
            scroll_up_lines(&mut state.chat_scroll_offset, 1);
        }
        crossterm::event::KeyCode::Down => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            }
            scroll_down_lines(&mut state.chat_scroll_offset, &mut state.chat_auto_scroll, 1, max_offset);
        }
        crossterm::event::KeyCode::PageUp => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            }
            scroll_up_lines(&mut state.chat_scroll_offset, PAGE_SIZE);
        }
        crossterm::event::KeyCode::PageDown => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            }
            scroll_down_lines(&mut state.chat_scroll_offset, &mut state.chat_auto_scroll, PAGE_SIZE, max_offset);
        }
        crossterm::event::KeyCode::Home => {
            state.chat_auto_scroll = false;
            state.chat_scroll_offset = 0;
        }
        crossterm::event::KeyCode::End => {
            state.chat_auto_scroll = true;
        }
        _ => {}
    }
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
fn scroll_peers_tab(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    match key_code {
        crossterm::event::KeyCode::Up => {
            state.peer_selection = state.peer_selection.saturating_sub(1);
        }
        crossterm::event::KeyCode::Down
            if state.peer_selection < state.peers.len().saturating_sub(1) => {
                state.peer_selection += 1;
            }
        _ => {}
    }
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
    let broadcast_messages: Vec<(String, Option<String>)> = state.messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().is_some_and(|id| id == peer_id))
        .cloned()
        .collect();

    if broadcast_messages.is_empty() {
        return;
    }

    if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
        let max_offset = broadcast_messages.len().saturating_sub(1);
        match scroll_dir {
            "up" => {
                if *auto_scroll {
                    *auto_scroll = false;
                    *scroll_offset = max_offset;
                } else if *scroll_offset >= WHEEL_SCROLL_LINES {
                    *scroll_offset -= WHEEL_SCROLL_LINES;
                } else {
                    *scroll_offset = 0;
                }
            }
            "down" => {
                *scroll_offset = (*scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
                if *scroll_offset >= max_offset {
                    *auto_scroll = true;
                }
            }
            _ => {}
        }
    }
}

/// Handle mouse wheel for DM section of DM tab
fn mouse_scroll_dm_section(state: &mut AppState, scroll_dir: &str, peer_id: &str) {
    if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id)
        && let Some(msgs) = state.dm_messages.get(peer_id) {
            let max_offset = msgs.len().saturating_sub(1);
            match scroll_dir {
                "up" => {
                    if *auto_scroll {
                        *auto_scroll = false;
                        *scroll_offset = max_offset;
                    } else if *scroll_offset >= WHEEL_SCROLL_LINES {
                        *scroll_offset -= WHEEL_SCROLL_LINES;
                    } else {
                        *scroll_offset = 0;
                    }
                }
                "down" => {
                    *scroll_offset = (*scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
                    if *scroll_offset >= max_offset {
                        *auto_scroll = true;
                    }
                }
                _ => {}
            }
        }
}

/// Handle mouse wheel for Chat tab (broadcast)
fn mouse_scroll_chat_tab(state: &mut AppState, scroll_dir: &str) {
    let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
    match scroll_dir {
        "up" => {
            if state.chat_auto_scroll {
                state.chat_auto_scroll = false;
                state.chat_scroll_offset = max_offset;
            } else if state.chat_scroll_offset >= WHEEL_SCROLL_LINES {
                state.chat_scroll_offset -= WHEEL_SCROLL_LINES;
            } else {
                state.chat_scroll_offset = 0;
            }
        }
        "down" => {
            state.chat_auto_scroll = false;
            state.chat_scroll_offset = (state.chat_scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
        }
        _ => {}
    }
}

/// Handle mouse wheel for Log tab
fn mouse_scroll_log_tab(state: &mut AppState, scroll_dir: &str) {
    let max_offset = get_tui_logs().len().saturating_sub(state.visible_log_count);
    match scroll_dir {
        "up" => {
            if state.log_auto_scroll {
                state.log_auto_scroll = false;
                state.log_scroll_offset = max_offset;
            } else if state.log_scroll_offset >= WHEEL_SCROLL_LINES {
                state.log_scroll_offset -= WHEEL_SCROLL_LINES;
            } else {
                state.log_scroll_offset = 0;
            }
        }
        "down" => {
            state.log_auto_scroll = false;
            state.log_scroll_offset = (state.log_scroll_offset + WHEEL_SCROLL_LINES).min(max_offset);
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
