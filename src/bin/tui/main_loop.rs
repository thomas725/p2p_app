use super::constants::CHANNEL_CAPACITY;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Run the new 4-task TUI architecture
///
/// Orchestrates the spawning and supervision of the four concurrent tasks:
/// - **SwarmHandler**: Translates libp2p events to SwarmEvent
/// - **InputHandler**: Polls terminal for keyboard/mouse input
/// - **CommandProcessor**: Receives events, mutates shared AppState
/// - **RenderLoop**: Renders AppState to terminal at ~60 FPS
///
/// All tasks communicate via bounded MPSC channels (capacity: 100 events).
/// State is shared behind Arc<Mutex<AppState>> for safe concurrent access.
///
/// The function sets up terminal mode (alternate screen, raw mode, mouse capture),
/// initializes the state from database, and waits for any task to exit (indicating error).
pub async fn run_new_tui(
    swarm: libp2p::swarm::Swarm<p2p_app::AppBehaviour>,
    topic_str: String,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> color_eyre::Result<()> {
    use ratatui::crossterm::{
        event::PushKeyboardEnhancementFlags,
        execute,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
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
    let own_nickname = p2p_app::ensure_self_nickname().unwrap_or_else(|_| "Anonymous".to_string());

    // Get database info by attempting a connection to log the path
    let db_info = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "SQLite database (auto-selected in current directory)".to_string());
    p2p_app::log_debug(&logs, format!("Database: {}", db_info));
    p2p_app::log_debug(&logs, format!("Loading data for topic: {}", topic_str));

    // Load initial messages from database
    let initial_messages = super::state::load_and_format_messages(
        &topic_str,
        super::constants::MAX_MESSAGE_HISTORY,
        &logs,
        &std::collections::HashMap::new(),
        &std::collections::HashMap::new(),
        &own_nickname,
    );
    p2p_app::log_debug(
        &logs,
        format!("Loaded {} messages from database", initial_messages.len()),
    );

    // Load initial peers from database (deduplicated by peer_id)
    let initial_peers = if let Ok(db_peers) = p2p_app::load_peers() {
        let mut peers = VecDeque::new();
        let mut seen_ids = std::collections::HashSet::new();

        // Deduplicate first, then apply limit
        for peer in db_peers.iter() {
            // Skip duplicate peer IDs (keep first occurrence with most recent last_seen)
            if !seen_ids.insert(peer.peer_id.clone()) {
                continue;
            }

            // Stop if we've reached MAX_PEERS
            if peers.len() >= super::constants::MAX_PEERS {
                break;
            }

            let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
            let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
            peers.push_back((peer.peer_id.clone(), first_seen, last_seen));
        }
        p2p_app::log_debug(
            &logs,
            format!(
                "Loaded {} unique peers from {} total database entries",
                peers.len(),
                db_peers.len()
            ),
        );
        peers
    } else {
        p2p_app::log_debug(&logs, "No peers found in database".to_string());
        VecDeque::new()
    };

    let state = Arc::new(Mutex::new(super::state::AppState::new(
        topic_str.clone(),
        logs.clone(),
        own_nickname.clone(),
        std::collections::HashMap::new(),
        std::collections::HashMap::new(),
        initial_messages,
        initial_peers,
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
    let exit_reason = tokio::select! {
        result = swarm_handler => {
            if result.is_err() {
                p2p_app::log_debug(&logs, "SwarmHandler panicked".to_string());
                "SwarmHandler task error"
            } else {
                p2p_app::log_debug(&logs, "SwarmHandler exited".to_string());
                "SwarmHandler completed"
            }
        }
        result = input_handler => {
            if result.is_err() {
                p2p_app::log_debug(&logs, "InputHandler panicked".to_string());
                "InputHandler task error"
            } else {
                p2p_app::log_debug(&logs, "InputHandler exited".to_string());
                "InputHandler completed"
            }
        }
        result = command_processor => {
            if result.is_err() {
                p2p_app::log_debug(&logs, "CommandProcessor panicked".to_string());
                "CommandProcessor task error"
            } else {
                p2p_app::log_debug(&logs, "CommandProcessor exited".to_string());
                "CommandProcessor completed"
            }
        }
        result = render_loop => {
            if result.is_err() {
                p2p_app::log_debug(&logs, "RenderLoop panicked".to_string());
                "RenderLoop task error"
            } else {
                p2p_app::log_debug(&logs, "RenderLoop exited".to_string());
                "RenderLoop completed"
            }
        }
    };

    // Signal graceful shutdown to remaining tasks
    p2p_app::log_debug(&logs, format!("Initiating shutdown: {}", exit_reason));

    // Cleanup terminal state
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
    let _ = execute!(std::io::stdout(), crossterm::event::DisableMouseCapture);

    Ok(())
}
