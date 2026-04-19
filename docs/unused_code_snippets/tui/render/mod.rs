//! Rendering logic for the P2P chat TUI application.

use crate::tui::shared::{TuiState, MAX_MESSAGES, MAX_LOGS};
use crate::tui::tab::{Tab, TabContent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
    Frame,
};
use std::collections::{BTreeMap, VecDeque};

/// Renders the main TUI interface
pub fn render_ui(frame: &mut Frame, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(if state.notification_shown() { 1 } else { 0 }),
            Constraint::Min(1),
        ])
        .split(frame.size());

    // Render tabs
    let titles: Vec<String> = vec![
        "Chat".to_string(),
        "Peers".to_string(),
        format_dm_tab_title(&state),
        "Log".to_string(),
    ];
    
    let tab_widget = Tabs::new(titles.iter().map(|t| t.as_str()))
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .select(state.active_tab)
        .style(Style::default().fg(Color::White))
        .highlight_style(Style::default().fg(Color::Yellow));
    
    frame.render_widget(tab_widget, chunks[0]);

    // Render notifications area
    if state.notification_shown() {
        let notification_text = if state.unread_broadcasts > 0 {
            format!("{} broadcast(s) | ", state.unread_broadcasts)
        } else {
            String::new()
        };
        
        let dm_count = state.unread_dms.len();
        let dm_text = if dm_count > 0 {
            format!("{} DM(s)", dm_count)
        } else {
            String::new()
        };
        
        let full_text = format!("{}{}", notification_text, dm_text);
        let notification_block = Paragraph::new(full_text)
            .block(Block::default().borders(Borders::ALL).title("Notifications"))
            .style(Style::default().fg(Color::Cyan));
        
        frame.render_widget(notification_block, chunks[1]);
    }

    // Render content area based on active tab
    match state.tab_content() {
        TabContent::Chat => render_chat(frame, state, chunks[2]),
        TabContent::Peers => render_peers(frame, state, chunks[2]),
        TabContent::Direct(peer_id) => render_direct_message(frame, state, chunks[2], &peer_id),
        TabContent::Log => render_log(frame, state, chunks[2]),
    }
}

fn state.notification_shown(&self) -> bool {
    self.unread_broadcasts > 0 || !self.unread_dms.is_empty()
}

fn state.tab_content(&self) -> TabContent {
    match self.active_tab {
        0 => TabContent::Chat,
        1 => TabContent::Peers,
        idx if idx == 2 + self.dm_tabs.len() => TabContent::Log,
        idx if idx >= 2 && idx < 2 + self.dm_tabs.len() => {
            let dm_idx = idx - 2;
            if let Some(tab) = self.dm_tabs.get(dm_idx) {
                TabContent::Direct(tab.peer_id.clone())
            } else {
                TabContent::Chat
            }
        }
        _ => TabContent::Chat,
    }
}

fn format_dm_tab_title(state: &TuiState) -> String {
    let log_index = 2 + state.dm_tabs.len();
    match state.active_tab {
        0..=1 => "Direct Messages".to_string(),
        idx if idx == log_index => "Log".to_string(),
        idx if idx >= 2 && idx < log_index => {
            if let Some(tab) = state.dm_tabs.get(idx - 2) {
                format!("{} (X)", tab.short_id())
            } else {
                "Direct Messages".to_string()
            }
        }
        _ => "Direct Messages".to_string(),
    }
}

fn render_chat(frame: &mut Frame, state: &TuiState, area: ratatui::prelude::Rect) {
    // Chat rendering logic
    let items: Vec<ListItem> = state.messages.iter().rev().map(|msg| {
        let sender = match &msg.peer_id {
            Some(peer_id) => {
                let display_name = peer_display_name(
                    peer_id,
                    &state.local_nicknames,
                    &state.received_nicknames,
                );
                format!("[{}]", display_name)
            }
            None => format!("[{}]", state.own_nickname),
        };
        
        Paragraph::new(format("{} {}", msg.timestamp, msg.content))
            .block(Block::default().borders(Borders::NONE))
    }).collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Chat"));
    
    frame.render_widget(list, area);
}

fn render_peers(frame: &mut Frame, state: &TuiState, area: ratatui::prelude::Rect) {
    // Peers rendering logic
    let items: Vec<ListItem> = (0..state.peers.len()).map(|i| {
        let (peer_id, first_seen, last_seen) = &state.peers[i];
        let display_name = peer_display_name(
            peer_id,
            &state.local_nicknames,
            &state.received_nicknames,
        );
        ListItem::new(format!("{} - Last seen: {}", display_name, last_seen))
    }).collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Peers"));
    
    frame.render_widget(list, area);
}

fn render_direct_message(
    frame: &mut Frame,
    state: &TuiState,
    area: ratatui::prelude::Rect,
    peer_id: &str,
) {
    // Direct message rendering logic
    let mut dm_text = String::new();
    if let Some(dms) = state.dm_messages.get(peer_id) {
        for msg in dms {
            dm_text.push_str(&format!("{} {}\n", msg.timestamp, msg.content));
        }
    }
    
    let peer_display = peer_display_name(
        peer_id,
        &state.local_nicknames,
        &state.received_nicknames,
    );
    let title = format!("Direct Message: {}", peer_display);
    
    let paragraph = Paragraph::new(dm_text)
        .block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(paragraph, area);
}

fn render_log(frame: &mut Frame, state: &TuiState, area: ratatui::prelude::Rect) {
    // Log rendering logic
    let items: Vec<ListItem> = state.logs.iter().rev().map(|log| {
        ListItem::new(log.clone())
    }).collect();
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Log"));
    
    frame.render_widget(list, area);
}

/// Get display name for a peer
pub fn peer_display_name(
    peer_id: &str,
    local_nicknames: &std::collections::HashMap<String, String>,
    received_nicknames: &std::collections::HashMap<String, String>,
) -> String {
    if let Some(nick) = local_nicknames.get(peer_id) {
        let short = short_peer_id(peer_id);
        return format!("{} ({})", nick, &short[..3.min(short.len())]);
    }
    if let Some(nick) = received_nicknames.get(peer_id) {
        let short = short_peer_id(peer_id);
        return format!("{} ({})", nick, &short[..3.min(short.len())]);
    }
    short_peer_id(peer_id)
}

/// Get short peer ID for display
pub fn short_peer_id(id: &str) -> String {
    id.chars().rev().take(8.min(id.len())).collect::<String>().chars().rev().collect()
}