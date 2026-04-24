use super::constants::FRAME_TIME_MS;
use super::state::AppState;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};
use std::io::Stdout;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
    state: Arc<Mutex<AppState>>,
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(FRAME_TIME_MS)); // 60 FPS

        loop {
            interval.tick().await;

            let _ = terminal.draw(|f| {
                if let Ok(mut s) = state.lock() {
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

                    // Render tabs
                    let tab_titles = s.dynamic_tabs.all_titles();
                    let tabs = Tabs::new(tab_titles)
                        .style(Style::default().fg(Color::Cyan))
                        .select(s.active_tab);
                    f.render_widget(tabs, chunks[0]);

                    // Render peer count info
                    let peer_info = Paragraph::new(format!("Peers: {}", s.concurrent_peers));
                    f.render_widget(peer_info, chunks[1]);

                    // Render tab-specific content
                    let tab_content = s.dynamic_tabs.tab_index_to_content(s.active_tab);
                    match &tab_content {
                        TabContent::Chat => {
                            let messages_text = s.messages
                                .iter()
                                .map(|(msg, _)| msg.as_str())
                                .collect::<Vec<_>>()
                                .join("\n");
                            let messages_para = Paragraph::new(messages_text)
                                .block(Block::default().title("Broadcast Chat").borders(Borders::ALL));
                            f.render_widget(messages_para, chunks[2]);
                        }
                        TabContent::Peers => {
                            let peer_items: Vec<ListItem> = s.peers
                                .iter()
                                .enumerate()
                                .map(|(idx, (id, _first_seen, last_seen))| {
                                    let line = format!("{} ({})", id, last_seen);
                                    // Highlight selected peer
                                    if idx == s.peer_selection {
                                        ListItem::new(line).style(Style::default().bg(Color::DarkGray))
                                    } else {
                                        ListItem::new(line)
                                    }
                                })
                                .collect();
                            let peers_list = List::new(peer_items)
                                .block(Block::default().title("Connected Peers").borders(Borders::ALL));
                            f.render_widget(peers_list, chunks[2]);
                        }
                        TabContent::Direct(peer_id) => {
                            let dm_text = s.dm_messages
                                .get(peer_id)
                                .map(|msgs| msgs.iter().cloned().collect::<Vec<_>>().join("\n"))
                                .unwrap_or_else(|| "No messages yet".to_string());
                            // Show last 8 chars of peer ID (more distinctive than first 8)
                            let short_id = if peer_id.len() <= 8 {
                                peer_id.clone()
                            } else {
                                peer_id[peer_id.len()-8..].to_string()
                            };
                            let dm_para = Paragraph::new(dm_text)
                                .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
                            f.render_widget(dm_para, chunks[2]);
                        }
                        TabContent::Log => {
                            // Sync logs from centralized logging
                            s.sync_logs_from_centralized();
                            let log_text = s.logs
                                .lock()
                                .ok()
                                .map(|logs| logs.iter().cloned().collect::<Vec<_>>().join("\n"))
                                .unwrap_or_else(|| "No logs".to_string());
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
                        f.render_widget(&s.chat_input, inner_area);
                    } else {
                        f.render_widget(input_block, chunks[3]);
                    }

                    // Render keyboard shortcuts
                    let shortcuts = Paragraph::new("Tab: next tab | Shift+Tab: prev tab | ↑↓: scroll | Enter: send | F12: toggle mouse | Ctrl+Q: quit");
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
