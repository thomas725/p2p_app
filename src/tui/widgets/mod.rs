//! UI widgets for the P2P chat TUI application.

use crate::tui::tab::{Tab, TabContent};
use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, List, ListItem},
};

/// Renders the peer list widget
pub fn render_peer_list(area: Rect, state: &crate::tui::state::TuiState) -> List {
    let items: Vec<ListItem> = (0..state.peers.len())
        .map(|i| {
            let (peer_id, first_seen, last_seen) = &state.peers[i];
            let display_name = crate::tui::render::peer_display_name(
                peer_id,
                &state.local_nicknames,
                &state.received_nicknames,
            );
            ListItem::new(format!("{} - Last seen: {}", display_name, last_seen))
        })
        .collect();

    List::new(items).block(Block::default().borders(Borders::ALL).title("Peers"))
}

/// Renders the message list widget
pub fn render_message_list(area: Rect, state: &crate::tui::state::TuiState) -> List {
    let items: Vec<ListItem> = state
        .messages
        .iter()
        .rev()
        .map(|msg| {
            let sender = match &msg.peer_id {
                Some(peer_id) => {
                    let display_name = crate::tui::render::peer_display_name(
                        peer_id,
                        &state.local_nicknames,
                        &state.received_nicknames,
                    );
                    format!("[{}]", display_name)
                }
                None => format!("[{}]", state.own_nickname),
            };

            ListItem::new(format!("{} {}", msg.timestamp, msg.content))
        })
        .collect();

    List::new(items).block(Block::default().borders(Borders::ALL).title("Chat"))
}

/// Renders the direct message widget
pub fn render_direct_message(area: Rect, state: &crate::tui::state::TuiState, peer_id: &str) {
    // Direct message widget rendering
}

/// Renders the log widget
pub fn render_log(area: Rect, state: &crate::tui::state::TuiState) -> List {
    let items: Vec<ListItem> = state
        .logs
        .iter()
        .rev()
        .map(|log| ListItem::new(log.clone()))
        .collect();

    List::new(items).block(Block::default().borders(Borders::ALL).title("Log"))
}

/// Handles widget interaction
pub fn handle_widget_interaction(event: crate::Event, state: &mut crate::tui::state::TuiState) {
    // Widget interaction handling
}
