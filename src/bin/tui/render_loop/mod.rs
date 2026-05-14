mod layout;

use super::constants::FRAME_TIME_MS;
use super::main_loop::RenderEvent;
use super::state::{AppState, SharedState};
use p2p_app::tui_render;
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
};
use std::io::Stdout;
use std::time::Duration;
use tokio::sync::mpsc;

/// Convert `AppState` to `TuiRenderState` for library rendering
fn app_state_to_render_state(state: &AppState) -> p2p_app::TuiRenderState {
    use std::collections::{BTreeMap, VecDeque};

    let tab_titles: Vec<String> = state.dynamic_tabs.all_titles();

    let dm_messages: BTreeMap<String, VecDeque<String>> = state
        .dm_messages
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let dm_message_ids: BTreeMap<String, VecDeque<Option<String>>> = state
        .dm_message_ids
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    let dm_scroll_state: BTreeMap<String, (usize, bool)> = state
        .dm_scroll_state
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    let dm_broadcast_scroll_state: BTreeMap<String, (usize, bool)> = state
        .dm_broadcast_scroll_state
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();

    p2p_app::TuiRenderState {
        tab_titles,
        active_tab: state.active_tab,
        messages: state.messages.iter().map(|(m, _)| m.clone()).collect(),
        message_ids: state.message_ids.clone(),
        broadcast_receipts: state.broadcast_receipts.clone(),
        peers: state
            .peers
            .iter()
            .map(|(a, b, c)| (a.clone(), b.clone(), c.clone()))
            .collect(),
        dm_messages,
        dm_message_ids,
        dm_receipts: state.dm_receipts.clone(),
        input_text: state.chat_input.lines().join("\n"),
        editing_nickname: state.editing_nickname,
        nickname_peer_id: state.editing_nickname_peer.clone().unwrap_or_default(),
        connected: true,
        peer_count: state.concurrent_peers,
        mouse_capture: state.mouse_capture,
        popup: state.popup.clone(),
        chat_scroll_offset: state.chat_scroll_offset,
        chat_auto_scroll: state.chat_auto_scroll,
        dm_scroll_state,
        dm_broadcast_scroll_state,
        broadcast_selection: state.broadcast_selection,
        peer_selection: state.peer_selection,
    }
}

/// Orchestrate the frame layout and dispatch to appropriate tab renderers
pub fn render_frame(f: &mut Frame, state: &mut AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // tabs
            Constraint::Length(1), // peer info
            Constraint::Min(0),    // messages
            Constraint::Length(5), // input area
            Constraint::Length(1), // shortcuts
            Constraint::Length(1), // status
        ])
        .split(f.area());

    let mut render_state = app_state_to_render_state(state);

    tui_render::render_tabs(f, chunks[0], &render_state);
    tui_render::render_peer_info(f, chunks[1], &render_state);

    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    match &tab_content {
        TabContent::Chat => {
            tui_render::render_chat_content(f, chunks[2], &mut render_state);
        }
        TabContent::Peers => {
            tui_render::render_peers_content(f, chunks[2], &render_state);
        }
        TabContent::Direct(peer_id) => {
            tui_render::render_dm_content(f, chunks[2], peer_id, &mut render_state);
        }
        TabContent::Log => {
            tui_render::render_log_content(f, chunks[2], &render_state);
        }
    }

    layout::render_input_section(f, chunks[3], state, &tab_content);
    layout::render_shortcuts(f, chunks[4]);
    layout::render_status_bar(f, chunks[5], state);

    if let Some(text) = state.popup.clone() {
        render_popup(f, text);
    }
}

fn render_popup(f: &mut Frame, text: String) {
    use ratatui::layout::{Alignment, Rect};
    use ratatui::style::{Color, Style};
    use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

    let area = f.area();
    let w = (f32::from(area.width) * 0.70) as u16;
    let h = (f32::from(area.height) * 0.40) as u16;
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(w)) / 2,
        y: area.y + (area.height.saturating_sub(h)) / 2,
        width: w.max(20).min(area.width),
        height: h.max(6).min(area.height),
    };

    f.render_widget(Clear, popup);
    let p = Paragraph::new(text)
        .block(
            Block::default()
                .title("Details (press any key / click to close)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: false });
    f.render_widget(p, popup);
}

/// Spawns the render loop task that continuously renders the TUI
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
                    let para = ratatui::widgets::Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::state::AppState;
    use std::collections::{HashMap, VecDeque};

    fn app_state() -> AppState {
        AppState::new(
            "topic".to_string(),
            "TestUser".to_string(),
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
    fn test_app_state_to_render_state_defaults() {
        let state = app_state();
        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.tab_titles.len(), 3); // Chat, Peers, Log
        assert_eq!(rs.active_tab, 0);
        assert!(rs.messages.is_empty());
        assert!(rs.peers.is_empty());
        assert_eq!(rs.input_text, "");
        assert!(!rs.editing_nickname);
        assert!(rs.connected);
        assert_eq!(rs.peer_count, 0);
        assert!(rs.mouse_capture);
        assert_eq!(rs.popup, None);
        assert_eq!(rs.chat_scroll_offset, 0);
        assert!(rs.chat_auto_scroll);
        assert_eq!(rs.peer_selection, 0);
    }

    #[test]
    fn test_app_state_to_render_state_messages() {
        let mut state = app_state();
        state
            .messages
            .push_back(("hello".to_string(), Some("p1".to_string())));
        state.messages.push_back(("world".to_string(), None));
        state.message_ids.push_back(Some("m1".to_string()));
        state.message_ids.push_back(Some("m2".to_string()));

        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.messages.len(), 2);
        assert_eq!(rs.messages[0], "hello");
        assert_eq!(rs.messages[1], "world");
        assert_eq!(rs.message_ids.len(), 2);
    }

    #[test]
    fn test_app_state_to_render_state_peers() {
        let mut state = app_state();
        state.peers.push_back((
            "p1".into(),
            "2024-01-01 12:00:00".into(),
            "2024-01-02 12:00:00".into(),
        ));
        state.peers.push_back((
            "p2".into(),
            "2024-01-03 12:00:00".into(),
            "2024-01-04 12:00:00".into(),
        ));
        state.peer_selection = 1;
        state.concurrent_peers = 5;

        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.peers.len(), 2);
        assert_eq!(rs.peers[0].0, "p1");
        assert_eq!(rs.peer_selection, 1);
        assert_eq!(rs.peer_count, 5);
    }

    #[test]
    fn test_app_state_to_render_state_dm_messages() {
        let mut state = app_state();
        state
            .dm_messages
            .insert("peer-a".to_string(), VecDeque::from(["dm msg".to_string()]));
        state.dm_message_ids.insert(
            "peer-a".to_string(),
            VecDeque::from([Some("dm1".to_string())]),
        );
        state
            .dm_scroll_state
            .insert("peer-a".to_string(), (2, true));
        state
            .dm_broadcast_scroll_state
            .insert("peer-a".to_string(), (1, false));

        let rs = app_state_to_render_state(&state);
        assert!(rs.dm_messages.contains_key("peer-a"));
        assert_eq!(rs.dm_messages["peer-a"][0], "dm msg");
        assert!(rs.dm_message_ids.contains_key("peer-a"));
        assert_eq!(rs.dm_scroll_state.get("peer-a"), Some(&(2, true)));
        assert_eq!(
            rs.dm_broadcast_scroll_state.get("peer-a"),
            Some(&(1, false))
        );
    }

    #[test]
    fn test_app_state_to_render_state_input_text() {
        let mut state = app_state();
        let mut ta = ratatui_textarea::TextArea::default();
        ta.insert_str("hello\nworld");
        state.chat_input = ta;
        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.input_text, "hello\nworld");
    }

    #[test]
    fn test_app_state_to_render_state_editing_nickname() {
        let mut state = app_state();
        state.editing_nickname = true;
        state.editing_nickname_peer = Some("peer-edit".to_string());
        let rs = app_state_to_render_state(&state);
        assert!(rs.editing_nickname);
        assert_eq!(rs.nickname_peer_id, "peer-edit");
    }

    #[test]
    fn test_app_state_to_render_state_popup() {
        let mut state = app_state();
        state.popup = Some("popup text".to_string());
        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.popup, Some("popup text".to_string()));
    }

    #[test]
    fn test_app_state_to_render_state_scroll_and_selection() {
        let mut state = app_state();
        state.chat_scroll_offset = 10;
        state.chat_auto_scroll = false;
        state.broadcast_selection = Some(3);
        state.mouse_capture = false;

        let rs = app_state_to_render_state(&state);
        assert_eq!(rs.chat_scroll_offset, 10);
        assert!(!rs.chat_auto_scroll);
        assert_eq!(rs.broadcast_selection, Some(3));
        assert!(!rs.mouse_capture);
    }

    #[test]
    fn test_app_state_to_render_state_broadcast_receipts() {
        let mut state = app_state();
        let mut inner = HashMap::new();
        inner.insert("p1".to_string(), 100.0);
        state.broadcast_receipts.insert("msg-1".to_string(), inner);
        state
            .dm_receipts
            .insert("dm-1".to_string(), ("p2".to_string(), 200.0));

        let rs = app_state_to_render_state(&state);
        assert!(rs.broadcast_receipts.contains_key("msg-1"));
        assert_eq!(rs.dm_receipts.get("dm-1"), Some(&("p2".to_string(), 200.0)));
    }
}
