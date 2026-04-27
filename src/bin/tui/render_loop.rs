use super::constants::FRAME_TIME_MS;
use super::main_loop::RenderEvent;
use super::state::{SharedState, AppState};
use p2p_app::get_tui_logs;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, ListItem, Paragraph, Tabs},
    Frame,
};
use std::io::Stdout;
use std::time::Duration;
use tokio::sync::mpsc;

fn calc_visible_tuples(
    messages: &std::collections::VecDeque<(String, Option<String>)>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let mut used = 0;
        let mut count = 0;
        for (msg, _) in messages.iter().rev() {
            let msg_lines = count_lines(msg, text_width);
            if used + msg_lines > usable_height {
                break;
            }
            used += msg_lines;
            count += 1;
        }
        let visible = count;
        let offset = total_items.saturating_sub(visible);
        (visible, offset)
    } else {
        let visible = if scroll_offset < total_items {
            let mut used = 0;
            let mut count = 0;
            for (msg, _) in messages.iter().skip(scroll_offset) {
                let msg_lines = count_lines(msg, text_width);
                if used > 0 && used + msg_lines > usable_height {
                    break;
                }
                used += msg_lines;
                count += 1;
            }
            count.max(1)
        } else {
            1
        };
        (visible, scroll_offset)
    }
}

fn calc_visible_strings(
    messages: &std::collections::VecDeque<String>,
    auto_scroll: bool,
    scroll_offset: usize,
    text_width: usize,
    usable_height: usize,
) -> (usize, usize) {
    let total_items = messages.len();
    if total_items == 0 {
        return (0, 0);
    }

    if auto_scroll {
        let mut used = 0;
        let mut count = 0;
        for msg in messages.iter().rev() {
            let msg_lines = count_lines(msg, text_width);
            if used > 0 && used + msg_lines > usable_height {
                break;
            }
            used += msg_lines;
            count += 1;
        }
        let visible = count;
        let offset = total_items.saturating_sub(visible);
        (visible, offset)
    } else {
        let visible = if scroll_offset < total_items {
            let mut used = 0;
            let mut count = 0;
            for msg in messages.iter().skip(scroll_offset) {
                let msg_lines = count_lines(msg, text_width);
                if used > 0 && used + msg_lines > usable_height {
                    break;
                }
                used += msg_lines;
                count += 1;
            }
            count.max(1)
        } else {
            1
        };
        (visible, scroll_offset)
    }
}

fn count_lines(text: &str, text_width: usize) -> usize {
    let clean_text = p2p_app::strip_ansi_codes(text);
    let lines: Vec<&str> = clean_text.split('\n').collect();

    if lines.is_empty() {
        return 1;
    }

    let mut total = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.is_empty() {
            if i < lines.len() - 1 {
                total += 1;
            }
        } else {
            total += (line.len() + text_width - 1) / text_width;
        }
    }
    total.max(1)
}

fn render_chat_tab(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    text_width: usize,
    usable_height: usize,
) {
    state.chat_area_height = area.height as usize;
    let (visible, effective_offset) = calc_visible_tuples(
        &state.messages,
        state.chat_auto_scroll,
        state.chat_scroll_offset,
        text_width,
        usable_height,
    );
    state.visible_message_count = visible;
    state.chat_message_offset = effective_offset;

    let visible_messages: Vec<ListItem> = state.messages
        .iter()
        .skip(effective_offset)
        .take(visible)
        .map(|(msg, _)| ListItem::new(msg.as_str()))
        .collect();

    state.chat_message_lines = state.messages
        .iter()
        .skip(effective_offset)
        .take(visible)
        .map(|(msg, _)| count_lines(msg, text_width))
        .collect();

    let messages_list = ratatui::widgets::List::new(visible_messages)
        .block(Block::default().title("Broadcast Chat").borders(Borders::ALL));
    frame.render_widget(messages_list, area);
}

fn render_peers_tab(
    frame: &mut Frame,
    area: Rect,
    state: &AppState,
) {
    let peer_items: Vec<ListItem> = state.peers
        .iter()
        .enumerate()
        .map(|(idx, (id, _first_seen, last_seen))| {
            let line = format!("{} ({})", id, last_seen);
            if idx == state.peer_selection {
                ListItem::new(line).style(Style::default().bg(Color::DarkGray))
            } else {
                ListItem::new(line)
            }
        })
        .collect();
    let peers_list = ratatui::widgets::List::new(peer_items)
        .block(Block::default().title("Connected Peers").borders(Borders::ALL));
    frame.render_widget(peers_list, area);
}

fn render_dm_tab(
    frame: &mut Frame,
    area: Rect,
    state: &mut AppState,
    peer_id: &str,
    text_width: usize,
    _usable_height: usize,
) {
    let short_id = if peer_id.len() <= 8 {
        peer_id.to_string()
    } else {
        peer_id[peer_id.len()-8..].to_string()
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let broadcast_area = chunks[0];
    let dm_area = chunks[1];

    let broadcast_usable_height = broadcast_area.height.saturating_sub(2) as usize;
    let dm_usable_height = dm_area.height.saturating_sub(2) as usize;

    let broadcast_messages: Vec<(String, Option<String>)> = state.messages
        .iter()
        .filter(|(_, sender_id)| sender_id.as_ref().map_or(false, |id| id == peer_id))
        .cloned()
        .collect();

    if !broadcast_messages.is_empty() {
        let broadcast_strings: std::collections::VecDeque<String> = broadcast_messages
            .iter()
            .map(|(msg, _)| msg.clone())
            .collect();

        let (broadcast_scroll_offset, broadcast_auto_scroll) = {
            let (offset, auto_scroll) = state.dm_broadcast_scroll_state
                .entry(peer_id.to_string())
                .or_insert((0, true));
            (*offset, *auto_scroll)
        };

        let (visible, effective_offset) = calc_visible_strings(
            &broadcast_strings,
            broadcast_auto_scroll,
            broadcast_scroll_offset,
            text_width,
            broadcast_usable_height,
        );
        // Store visible count for this peer's broadcast section
        let (_, dm_visible) = state.dm_visible_counts.get(peer_id).copied().unwrap_or((0, 0));
        state.dm_visible_counts.insert(peer_id.to_string(), (visible, dm_visible));

        let visible_broadcast: Vec<ListItem> = broadcast_strings
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();
        let broadcast_list = ratatui::widgets::List::new(visible_broadcast)
            .block(Block::default().title(format!("Broadcast from {}", short_id)).borders(Borders::ALL));
        frame.render_widget(broadcast_list, broadcast_area);
    } else {
        let broadcast_para = Paragraph::new("No broadcast messages")
            .block(Block::default().title(format!("Broadcast from {}", short_id)).borders(Borders::ALL));
        frame.render_widget(broadcast_para, broadcast_area);
    }

    let (scroll_offset_val, auto_scroll_val) = {
        let (offset, auto_scroll) = state.dm_scroll_state
            .entry(peer_id.to_string())
            .or_insert((0, true));
        (*offset, *auto_scroll)
    };

    if let Some(msgs) = state.dm_messages.get(peer_id) {
        let (visible, effective_offset) = calc_visible_strings(
            msgs,
            auto_scroll_val,
            scroll_offset_val,
            text_width,
            dm_usable_height,
        );
        // Store visible count for this peer's DM section
        let (broadcast_visible, _) = state.dm_visible_counts.get(peer_id).copied().unwrap_or((0, 0));
        state.dm_visible_counts.insert(peer_id.to_string(), (broadcast_visible, visible));

        let visible_msgs: Vec<ListItem> = msgs
            .iter()
            .skip(effective_offset)
            .take(visible)
            .map(|m| ListItem::new(m.as_str()))
            .collect();

        let dm_list = ratatui::widgets::List::new(visible_msgs)
            .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
        frame.render_widget(dm_list, dm_area);
    } else {
        let dm_para = Paragraph::new("No direct messages")
            .block(Block::default().title(format!("DM: {}", short_id)).borders(Borders::ALL));
        frame.render_widget(dm_para, dm_area);
    }
}

fn render_tabs(f: &mut Frame, tab_area: Rect, state: &AppState) {
    let tab_titles = state.dynamic_tabs.all_titles();
    let tabs = Tabs::new(tab_titles)
        .style(Style::default().fg(Color::Cyan))
        .select(state.active_tab);
    f.render_widget(tabs, tab_area);
}

fn render_peer_info(f: &mut Frame, peer_area: Rect, state: &AppState) {
    let peer_info = Paragraph::new(format!("Peers: {}", state.concurrent_peers));
    f.render_widget(peer_info, peer_area);
}

fn render_input_section(f: &mut Frame, input_area: Rect, state: &AppState, tab_content: &TabContent) {
    let title = if state.editing_nickname {
        "Edit Nickname (Enter to save, Esc to cancel)"
    } else {
        "Input"
    };
    let input_block = Block::default()
        .title(title)
        .borders(Borders::ALL);
    if tab_content.is_input_enabled() || state.editing_nickname {
        let inner_area = input_block.inner(input_area);
        f.render_widget(input_block, input_area);
        let mut textarea = state.chat_input.clone();
        textarea.set_cursor_line_style(Style::default());
        f.render_widget(&textarea, inner_area);
    } else {
        f.render_widget(input_block, input_area);
    }
}

fn render_shortcuts(f: &mut Frame, shortcuts_area: Rect) {
    let shortcuts = Paragraph::new("Tab: next tab | PgUp/PgDn: scroll | Home/End: jump | Enter: send | F12: mouse | Ctrl+Q: quit");
    f.render_widget(shortcuts, shortcuts_area);
}

fn render_status_bar(f: &mut Frame, status_area: Rect, state: &AppState) {
    let mouse_mode = if state.mouse_capture { "ON" } else { "OFF" };
    let status = Paragraph::new(format!("Connected [Mouse: {}]", mouse_mode));
    f.render_widget(status, status_area);
}

fn render_frame(f: &mut Frame, state: &mut AppState) {
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

    let avail_width = chunks[2].width as usize;
    let avail_height = chunks[2].height as usize;
    let text_width = avail_width.saturating_sub(4);
    let usable_height = avail_height.saturating_sub(2);

    render_tabs(f, chunks[0], state);
    render_peer_info(f, chunks[1], state);

    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    match &tab_content {
        TabContent::Chat => {
            render_chat_tab(f, chunks[2], state, text_width, usable_height);
        }
        TabContent::Peers => {
            render_peers_tab(f, chunks[2], state);
        }
        TabContent::Direct(peer_id) => {
            render_dm_tab(f, chunks[2], state, peer_id, text_width, usable_height);
        }
        TabContent::Log => {
            let log_text = get_tui_logs().join("\n");
            let log_para = Paragraph::new(log_text)
                .block(Block::default().title("Logs").borders(Borders::ALL));
            f.render_widget(log_para, chunks[2]);
        }
    }

    render_input_section(f, chunks[3], state, &tab_content);
    render_shortcuts(f, chunks[4]);
    render_status_bar(f, chunks[5], state);
}

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
                biased;
                Some(_) = render_rx.recv() => {}
                _ = interval.tick() => {}
                else => break,
            }

            let _ = terminal.draw(|f| {
                if let Ok(mut s) = state.try_lock() {
                    render_frame(f, &mut s);
                } else {
                    let para = Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}
