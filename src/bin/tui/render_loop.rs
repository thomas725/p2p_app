use super::constants::FRAME_TIME_MS;
use super::main_loop::RenderEvent;
use super::state::SharedState;
use p2p_app::get_tui_logs;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, ListItem, Paragraph, Tabs},
};
use std::io::Stdout;
use std::time::Duration;
use tokio::sync::mpsc;

/// Spawns the render loop task that continuously renders the TUI
///
/// This task reads the shared AppState on a fixed interval and draws it to the terminal.
///
/// **Rendering strategy:**
/// - Fixed 60 FPS (~16ms per frame) via tokio::time::interval
/// - Always renders regardless of changes (time-based, not event-driven)
/// - Acquires AppState lock briefly for each frame, then releases
/// - Handles partial redraws via ratatui library
///
/// **Future optimization:**
/// Could implement event-driven rendering by using a notification channel:
/// CommandProcessor would send a "state changed" signal, and RenderLoop
/// would only redraw on updates or timeout. This would reduce CPU usage.
///
/// **Layout:**
/// - Top tabs: Broadcast channel + DM tabs (for peer conversations)
/// - Chat area: Messages from selected tab
/// - Input box: Current message being typed
/// - Status bar: Peer list, connection status, debug logs
pub fn spawn_render_loop(
    state: SharedState,
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
    mut render_rx: mpsc::Receiver<RenderEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(FRAME_TIME_MS));

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    // Timeout - still render to handle terminal resize or keepalive
                }
                Some(_) = render_rx.recv() => {
                    // RenderEvent received - state changed, render immediately
                }
                else => {
                    // Channel closed - exit
                    break;
                }
            }

            let _ = terminal.draw(|f| {
                if let Ok(mut s) = state.try_lock() {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1),  // tabs
                            Constraint::Length(1),  // peer info
                            Constraint::Min(0),     // messages
                            Constraint::Length(5),  // input area
                            Constraint::Length(1),  // shortcuts
                            Constraint::Length(1),  // status
                        ])
                        .split(f.area());

                    // Calculate visible messages accounting for text wrapping
                    let avail_width = chunks[2].width as usize;
                    let avail_height = chunks[2].height as usize;
                    let text_width = avail_width.saturating_sub(4); // -4 for block borders
                    let usable_height = avail_height.saturating_sub(2); // -2 for block borders

                    // Render tabs
                    let tab_titles = s.dynamic_tabs.all_titles();
                    let tabs = Tabs::new(tab_titles)
                        .style(Style::default().fg(Color::Cyan))
                        .select(s.active_tab);
                    f.render_widget(tabs, chunks[0]);

                    // Render peer count info
                    let peer_info = Paragraph::new(format!("Peers: {}", s.concurrent_peers));
                    f.render_widget(peer_info, chunks[1]);

                    // Render tab-specific content with scrolling
                    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                    match &tab_content {
                        TabContent::Chat => {
                            let total_items = s.messages.len();

                            let (visible, effective_offset) = if s.chat_auto_scroll {
                                // For auto_scroll: count backwards from newest to see how many fit
                                let mut used = 0;
                                let mut count = 0;
                                for (msg, _) in s.messages.iter().rev() {
                                    let mut msg_lines = 0;
                                    for line in msg.split('\n') {
                                        if line.is_empty() {
                                            msg_lines += 1;
                                        } else {
                                            msg_lines += (line.len() + text_width - 1) / text_width;
                                        }
                                    }
                                    // Keep adding messages until we significantly exceed height
                                    // This fills empty space while allowing partial last messages
                                    if count > 0 && used > usable_height {
                                        break;
                                    }
                                    used += msg_lines;
                                    count += 1;
                                }
                                let visible = count.max(1);
                                let offset = total_items.saturating_sub(visible);
                                (visible, offset)
                            } else {
                                // For manual scroll: calculate visible from current offset
                                // Use conservative approach: only add messages that fit
                                let visible = if s.chat_scroll_offset < total_items {
                                    let mut used = 0;
                                    let mut count = 0;
                                    for (msg, _) in s.messages.iter().skip(s.chat_scroll_offset) {
                                        let mut msg_lines = 0;
                                        for line in msg.split('\n') {
                                            if line.is_empty() {
                                                msg_lines += 1;
                                            } else {
                                                msg_lines += (line.len() + text_width - 1) / text_width;
                                            }
                                        }
                                        // Break before adding if next message would overflow
                                        if used + msg_lines > usable_height && count > 0 {
                                            break;
                                        }
                                        used += msg_lines;
                                        count += 1;
                                    }
                                    count.max(1)
                                } else {
                                    1
                                };
                                (visible, s.chat_scroll_offset)
                            };

                            s.visible_message_count = visible;

                            // Get messages starting from offset position
                            let visible_messages: Vec<ListItem> = s.messages
                                .iter()
                                .skip(effective_offset)
                                .take(visible)
                                .map(|(msg, _)| ListItem::new(msg.as_str()))
                                .collect();

                            let messages_list = ratatui::widgets::List::new(visible_messages)
                                .block(Block::default().title("Broadcast Chat").borders(Borders::ALL));
                            f.render_widget(messages_list, chunks[2]);
                        }
                        TabContent::Peers => {
                            let peer_items: Vec<ListItem> = s.peers
                                .iter()
                                .enumerate()
                                .map(|(idx, (id, _first_seen, last_seen))| {
                                    let line = format!("{} ({})", id, last_seen);
                                    if idx == s.peer_selection {
                                        ListItem::new(line).style(Style::default().bg(Color::DarkGray))
                                    } else {
                                        ListItem::new(line)
                                    }
                                })
                                .collect();
                            let peers_list = ratatui::widgets::List::new(peer_items)
                                .block(Block::default().title("Connected Peers").borders(Borders::ALL));
                            f.render_widget(peers_list, chunks[2]);
                        }
                        TabContent::Direct(peer_id) => {
                            let (scroll_offset_val, auto_scroll_val) = {
                                let (scroll_offset, auto_scroll) = s.dm_scroll_state
                                    .entry(peer_id.clone())
                                    .or_insert((0, true));
                                (*scroll_offset, *auto_scroll)
                            };

                            if let Some(msgs) = s.dm_messages.get(peer_id) {
                                let total_items = msgs.len();

                                let (visible, effective_offset) = if auto_scroll_val {
                                    // For auto_scroll: count backwards from newest, aggressive fill
                                    let mut used = 0;
                                    let mut count = 0;
                                    for msg in msgs.iter().rev() {
                                        let mut msg_lines = 0;
                                        for line in msg.split('\n') {
                                            if line.is_empty() {
                                                msg_lines += 1;
                                            } else {
                                                msg_lines += (line.len() + text_width - 1) / text_width;
                                            }
                                        }
                                        if count > 0 && used > usable_height {
                                            break;
                                        }
                                        used += msg_lines;
                                        count += 1;
                                    }
                                    let visible = count.max(1);
                                    let offset = total_items.saturating_sub(visible);
                                    (visible, offset)
                                } else {
                                    // For manual scroll: conservative approach
                                    let visible = if scroll_offset_val < total_items {
                                        let mut used = 0;
                                        let mut count = 0;
                                        for msg in msgs.iter().skip(scroll_offset_val) {
                                            let mut msg_lines = 0;
                                            for line in msg.split('\n') {
                                                if line.is_empty() {
                                                    msg_lines += 1;
                                                } else {
                                                    msg_lines += (line.len() + text_width - 1) / text_width;
                                                }
                                            }
                                            if used + msg_lines > usable_height && count > 0 {
                                                break;
                                            }
                                            used += msg_lines;
                                            count += 1;
                                        }
                                        count.max(1)
                                    } else {
                                        1
                                    };
                                    (visible, scroll_offset_val)
                                };

                                let short_id = if peer_id.len() <= 8 {
                                    peer_id.clone()
                                } else {
                                    peer_id[peer_id.len()-8..].to_string()
                                };

                                let visible_msgs: Vec<ListItem> = msgs
                                    .iter()
                                    .skip(effective_offset)
                                    .take(visible)
                                    .map(|m| ListItem::new(m.as_str()))
                                    .collect();

                                let dm_list = ratatui::widgets::List::new(visible_msgs)
                                    .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
                                f.render_widget(dm_list, chunks[2]);
                            } else {
                                let dm_para = Paragraph::new("No messages yet");
                                f.render_widget(dm_para, chunks[2]);
                            }
                        }
                        TabContent::Log => {
                            let log_text = get_tui_logs().join("\n");
                            let log_para = Paragraph::new(log_text)
                                .block(Block::default().title("Logs").borders(Borders::ALL));
                            f.render_widget(log_para, chunks[2]);
                        }
                    }

                    // Render input area (only for chat/DM tabs)
                    let input_block = Block::default()
                        .title("Input")
                        .borders(Borders::ALL);
                    if tab_content.is_input_enabled() {
                        // Create a wrapper that combines textarea with block styling
                        let inner_area = input_block.inner(chunks[3]);
                        f.render_widget(input_block, chunks[3]);
                        // Set cursor style without underline
                        let mut textarea = s.chat_input.clone();
                        textarea.set_cursor_line_style(Style::default());
                        f.render_widget(&textarea, inner_area);
                    } else {
                        f.render_widget(input_block, chunks[3]);
                    }

                    // Render keyboard shortcuts
                    let shortcuts = Paragraph::new("Tab: next tab | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit");
                    f.render_widget(shortcuts, chunks[4]);

                    // Render status line with mouse mode
                    let mouse_mode = if s.mouse_capture { "ON" } else { "OFF" };
                    let status = Paragraph::new(format!("Connected [Mouse: {}]", mouse_mode));
                    f.render_widget(status, chunks[5]);
                } else {
                    let para = Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}
