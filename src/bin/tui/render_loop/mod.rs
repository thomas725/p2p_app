mod visibility;
mod layout;
mod tab_renderers;

use super::constants::FRAME_TIME_MS;
use super::main_loop::RenderEvent;
use super::{state::{SharedState, AppState}};
use p2p_app::tui_tabs::TabContent;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    Frame,
};
use std::io::Stdout;
use std::time::Duration;
use tokio::sync::mpsc;


/// Orchestrate the frame layout and dispatch to appropriate tab renderers
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

    layout::render_tabs(f, chunks[0], state);
    layout::render_peer_info(f, chunks[1], state);

    let tab_content = state.dynamic_tabs.tab_index_to_content(state.active_tab);
    match &tab_content {
        TabContent::Chat => {
            tab_renderers::render_chat_tab(f, chunks[2], state, text_width, usable_height);
        }
        TabContent::Peers => {
            tab_renderers::render_peers_tab(f, chunks[2], state);
        }
        TabContent::Direct(peer_id) => {
            tab_renderers::render_dm_tab(f, chunks[2], state, peer_id, text_width, usable_height);
        }
        TabContent::Log => {
            tab_renderers::render_log_tab(f, chunks[2], state, text_width, usable_height);
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
    let w = (area.width as f32 * 0.70) as u16;
    let h = (area.height as f32 * 0.40) as u16;
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
                    let para = ratatui::widgets::Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}
