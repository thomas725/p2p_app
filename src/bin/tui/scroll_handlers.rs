use super::constants::{PAGE_SIZE, WHEEL_SCROLL_LINES};
use super::state::AppState;
use p2p_app::p2plog_debug;

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

/// Handles scroll keys (arrow keys, Page Up/Down, Home, End) with hover-aware targeting
pub async fn handle_scroll_key(key_code: crossterm::event::KeyCode, state: &mut AppState) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    if matches!(tab_content, p2p_app::tui_tabs::TabContent::Peers) {
        match key_code {
            crossterm::event::KeyCode::Up => {
                state.peer_selection = state.peer_selection.saturating_sub(1);
            }
            crossterm::event::KeyCode::Down => {
                if state.peer_selection < state.peers.len().saturating_sub(1) {
                    state.peer_selection += 1;
                }
            }
            _ => {}
        }
    } else if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = &tab_content {
        // For DM tabs, determine which section to scroll based on mouse hover position
        let mid_row = 2 + (state.chat_area_height / 2);
        let mouse_row = state.last_mouse_row as usize;
        let in_broadcast_section = mouse_row < mid_row;

        if in_broadcast_section {
            // Scroll broadcast messages (top half)
            let broadcast_messages: Vec<(String, Option<String>)> = state.messages
                .iter()
                .filter(|(_, sender_id)| sender_id.as_ref().map_or(false, |id| id == peer_id))
                .cloned()
                .collect();

            if !broadcast_messages.is_empty() {
                if let Some((scroll_offset, auto_scroll)) = state.dm_broadcast_scroll_state.get_mut(peer_id) {
                    let visible_count = state.dm_visible_counts.get(peer_id).map(|(b, _)| *b).unwrap_or(1);
                    let max_offset = broadcast_messages.len().saturating_sub(visible_count);
                    match key_code {
                        crossterm::event::KeyCode::Up => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            if *scroll_offset > 0 {
                                *scroll_offset -= 1;
                            }
                        }
                        crossterm::event::KeyCode::Down => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            if *scroll_offset < max_offset {
                                *scroll_offset += 1;
                                if *scroll_offset >= max_offset {
                                    *auto_scroll = true;
                                }
                            }
                        }
                        crossterm::event::KeyCode::PageUp => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            *scroll_offset = scroll_offset.saturating_sub(PAGE_SIZE);
                        }
                        crossterm::event::KeyCode::PageDown => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            *scroll_offset = (*scroll_offset + PAGE_SIZE).min(max_offset);
                            if *scroll_offset >= max_offset {
                                *auto_scroll = true;
                            }
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
            }
        } else {
            // Scroll DM messages (bottom half)
            if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id) {
                if let Some(msgs) = state.dm_messages.get(peer_id) {
                    let visible_count = state.dm_visible_counts.get(peer_id).map(|(_, d)| *d).unwrap_or(1);
                    let max_offset = msgs.len().saturating_sub(visible_count);
                    match key_code {
                        crossterm::event::KeyCode::Up => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            if *scroll_offset > 0 {
                                *scroll_offset -= 1;
                            }
                        }
                        crossterm::event::KeyCode::Down => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            if *scroll_offset < max_offset {
                                *scroll_offset += 1;
                                if *scroll_offset >= max_offset {
                                    *auto_scroll = true;
                                }
                            }
                        }
                        crossterm::event::KeyCode::PageUp => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            *scroll_offset = scroll_offset.saturating_sub(PAGE_SIZE);
                        }
                        crossterm::event::KeyCode::PageDown => {
                            if *auto_scroll {
                                *auto_scroll = false;
                                *scroll_offset = max_offset;
                            }
                            *scroll_offset = (*scroll_offset + PAGE_SIZE).min(max_offset);
                            if *scroll_offset >= max_offset {
                                *auto_scroll = true;
                            }
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
            }
        }
    } else {
        // For Chat tab (broadcast)
        match key_code {
            crossterm::event::KeyCode::Up => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                if state.chat_scroll_offset > 0 {
                    state.chat_scroll_offset -= 1;
                }
            }
            crossterm::event::KeyCode::Down => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
                if state.chat_scroll_offset < max_offset {
                    state.chat_scroll_offset += 1;
                }
            }
            crossterm::event::KeyCode::PageUp => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                state.chat_scroll_offset = state.chat_scroll_offset.saturating_sub(PAGE_SIZE);
            }
            crossterm::event::KeyCode::PageDown => {
                if state.chat_auto_scroll {
                    state.chat_auto_scroll = false;
                    state.chat_scroll_offset = state.messages.len().saturating_sub(state.visible_message_count);
                }
                let max_offset = state.messages.len().saturating_sub(state.visible_message_count);
                state.chat_scroll_offset = (state.chat_scroll_offset + PAGE_SIZE).min(max_offset);
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
}

/// Handles mouse wheel scrolling with hover-based section targeting for split DM tabs
pub fn handle_mouse_scroll(state: &mut AppState, scroll_dir: &str, _peer_id: Option<&str>) {
    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);

    if let p2p_app::tui_tabs::TabContent::Direct(peer_id) = &tab_content {
        // For DM tabs, determine which section to scroll based on mouse hover position
        let mid_row = 2 + (state.chat_area_height / 2);
        let mouse_row = state.last_mouse_row as usize;
        let in_broadcast_section = mouse_row < mid_row;

        if in_broadcast_section {
            // Scroll broadcast messages (top half)
            let broadcast_messages: Vec<(String, Option<String>)> = state.messages
                .iter()
                .filter(|(_, sender_id)| sender_id.as_ref().map_or(false, |id| id == peer_id))
                .cloned()
                .collect();

            if !broadcast_messages.is_empty() {
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
        } else {
            // Scroll DM messages (bottom half)
            if let Some((scroll_offset, auto_scroll)) = state.dm_scroll_state.get_mut(peer_id) {
                if let Some(msgs) = state.dm_messages.get(peer_id) {
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
        }
    } else {
        // For Chat tab (broadcast)
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
}
