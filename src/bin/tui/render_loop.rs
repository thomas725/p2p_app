use p2p_app::DynamicTabs;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Tabs},
};
use std::io::Stdout;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::time::Duration;
use super::state::AppState;

/// Spawns the render loop task that continuously renders the TUI
pub fn spawn_render_loop(
    state: Arc<Mutex<AppState>>,
    mut terminal: Terminal<CrosstermBackend<Stdout>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(16)); // 60 FPS

        loop {
            interval.tick().await;

            let _ = terminal.draw(|f| {
                if let Ok(s) = state.lock() {
                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(1),
                            Constraint::Length(1),
                            Constraint::Min(0),
                            Constraint::Length(5),
                            Constraint::Length(1),
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

                    // Render messages block
                    let messages_block = Block::default()
                        .title("Messages")
                        .borders(Borders::ALL);
                    f.render_widget(messages_block, chunks[2]);

                    // Render input area with chat input
                    let input_block = Block::default()
                        .title("Input")
                        .borders(Borders::ALL);
                    f.render_widget(input_block, chunks[3]);

                    // Render status line
                    let status = Paragraph::new("Connected");
                    f.render_widget(status, chunks[4]);
                } else {
                    let para = Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}
