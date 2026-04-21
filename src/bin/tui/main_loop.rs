use super::*;
use super::constants::CHANNEL_CAPACITY;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

/// Run the new 4-task TUI architecture
pub async fn run_new_tui(
    swarm: libp2p::swarm::Swarm<p2p_app::AppBehaviour>,
    topic_str: String,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> color_eyre::Result<()> {
    use ratatui::crossterm::{
        execute,
        terminal::{EnterAlternateScreen, disable_raw_mode, enable_raw_mode, LeaveAlternateScreen},
        event::PushKeyboardEnhancementFlags,
    };

    // Setup terminal
    let mut stdout = std::io::stdout();
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        EnterAlternateScreen,
    )?;
    enable_raw_mode()?;
    execute!(stdout, crossterm::event::EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Initialize state
    let own_nickname = p2p_app::ensure_self_nickname()
        .unwrap_or_else(|_| "Anonymous".to_string());

    let state = Arc::new(Mutex::new(super::state::AppState::new(
        topic_str.clone(),
        logs.clone(),
        own_nickname.clone(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        VecDeque::new(),
        VecDeque::new(),
    )));

    // Setup channels
    let (input_tx, input_rx) = mpsc::channel(CHANNEL_CAPACITY);

    // Spawn tasks
    // SwarmHandler returns both a handle and a receiver of SwarmEvent
    let (swarm_handler, swarm_event_rx) = p2p_app::spawn_swarm_handler(swarm, logs.clone());

    // InputHandler sends InputEvent to this channel
    let input_handler = super::input_handler::spawn_input_handler(input_tx);

    // CommandProcessor receives both InputEvent and SwarmEvent
    let (command_processor, _) = super::command_processor::spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        logs.clone(),
    );

    // RenderLoop reads state and renders
    let render_loop = super::render_loop::spawn_render_loop(state.clone(), terminal);

    p2p_app::log_debug(&logs, "Started 4-task TUI architecture".to_string());

    // Wait for any task to complete (indicates error or exit)
    tokio::select! {
        _ = swarm_handler => { p2p_app::log_debug(&logs, "SwarmHandler exited".to_string()); }
        _ = input_handler => { p2p_app::log_debug(&logs, "InputHandler exited".to_string()); }
        _ = command_processor => { p2p_app::log_debug(&logs, "CommandProcessor exited".to_string()); }
        _ = render_loop => { p2p_app::log_debug(&logs, "RenderLoop exited".to_string()); }
    }

    // Cleanup
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);

    Ok(())
}
