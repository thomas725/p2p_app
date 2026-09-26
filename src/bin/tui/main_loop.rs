use super::state::CHANNEL_CAPACITY;
use p2p_app::SwarmCommand;
use p2p_app::generated::models_queryable::{MessageReceipt, Peer};
use p2p_app::get_tui_logs;
use p2p_app::p2plog_debug;
use p2p_app::peers::KnownPeer;
use p2p_app::release_db_lock;
use p2p_app::set_tui_redraw_hook;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

/// Pure: extract nickname maps from DB peers
// Type aliases for organized receipt data
type BroadcastReceipts = HashMap<String, HashMap<String, f64>>;
type DmReceipts = HashMap<String, (String, f64)>;

fn extract_nickname_maps(
    peers: &[Peer],
) -> (
    HashMap<String, String>,
    HashMap<String, String>,
    HashMap<String, String>,
) {
    let mut local = HashMap::new();
    let mut received = HashMap::new();
    let mut self_for_peer = HashMap::new();
    for p in peers {
        if let Some(n) = p.peer_local_nickname.clone() {
            local.insert(p.peer_id.clone(), n);
        }
        if let Some(n) = p.received_nickname.clone() {
            received.insert(p.peer_id.clone(), n);
        }
        if let Some(n) = p.self_nickname_for_peer.clone() {
            self_for_peer.insert(p.peer_id.clone(), n);
        }
    }
    (local, received, self_for_peer)
}

/// Pure: deduplicate peers and format timestamps
fn deduplicate_peers(peers: &[KnownPeer]) -> VecDeque<p2p_app::PeerRecord> {
    let mut result = VecDeque::new();
    let mut seen_ids = HashSet::new();
    for peer in peers {
        if !seen_ids.insert(peer.peer_id.clone()) {
            continue;
        }
        let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
        let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
        result.push_back(p2p_app::PeerRecord {
            peer_id: peer.peer_id.clone(),
            first_seen,
            last_seen,
        });
    }
    result
}

/// Pure: organize flat receipt list into broadcast and DM maps
fn organize_receipts(receipts: &[MessageReceipt]) -> (BroadcastReceipts, DmReceipts) {
    let mut broadcast: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut dm: HashMap<String, (String, f64)> = HashMap::new();
    for r in receipts {
        let msg_id = r.msg_id.clone();
        let peer_id = r.peer_id.clone();
        let confirmed_at = r.confirmed_at;
        if r.kind == 0 {
            broadcast
                .entry(msg_id)
                .or_default()
                .insert(peer_id, confirmed_at);
        } else {
            dm.insert(msg_id, (peer_id, confirmed_at));
        }
    }
    (broadcast, dm)
}

/// Run the new 4-task TUI architecture
///
/// Orchestrates the spawning and supervision of the four concurrent tasks:
/// - **`SwarmHandler`**: Translates libp2p events to `SwarmEvent`
/// - **`EventSource`**: Polls terminal for keyboard/mouse input
/// - **`CommandProcessor`**: Receives events, mutates shared `AppState`
/// - **`RenderLoop`**: Renders `AppState` to terminal when requested
///
/// All tasks communicate via bounded MPSC channels (capacity: 100 events).
/// State is shared behind `Arc<Mutex<AppState>>` for safe concurrent access.
///
/// The function sets up terminal mode (alternate screen, raw mode, mouse capture),
/// initializes the state from database, and waits for any task to exit (indicating error).
/// Signal to trigger a render (instead of busy-polling)
#[derive(Debug, Clone, Copy)]
pub struct RenderEvent;

/// Returns the last `limit` logs in original order.
fn recent_tui_logs(logs: &[String], limit: usize) -> Vec<String> {
    let start = logs.len().saturating_sub(limit);
    logs.get(start..).unwrap_or(&[]).to_vec()
}

/// Bring up the raw-mode terminal and push the minimal keyboard-enhancement
/// flag.
///
/// Only `DISAMBIGUATE_ESCAPE_CODES` is needed to tell Ctrl+I apart from bare
/// Tab (Ctrl+letter is then reported as its true key code plus CONTROL) — the
/// `REPORT_ALL_KEYS` flag is deliberately NOT pushed: it makes the terminal
/// report every key (letters, shift presses) as CSI-u events, which is
/// unnecessary and leaves the shell deluged in CSI-u garbage if the keyboard
/// encoding is ever left active on exit. The push is sent AFTER raw mode is
/// enabled (some terminals only apply kitty encoding to keys read once raw mode
/// is active). The returned flag is the synchronous kitty-capability probe,
/// read BEFORE the input-handler task starts reading the tty so there is no
/// race (`WezTerm` reports supported; Konsole collapses Ctrl+I to 0x09). It
/// selects which peer-info shortcut is active on a Direct tab: Ctrl+I (kitty)
/// vs Ctrl+P (non-kitty).
fn setup_terminal() -> color_eyre::Result<(Terminal<CrosstermBackend<std::io::Stdout>>, bool)> {
    use ratatui::crossterm::{
        event::PushKeyboardEnhancementFlags,
        execute,
        terminal::{EnterAlternateScreen, enable_raw_mode},
    };

    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(
            crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        ),
        crossterm::event::EnableMouseCapture,
    )?;

    let keyboard_enhanced = crossterm::terminal::supports_keyboard_enhancement().unwrap_or(false);
    p2plog_debug(format!(
        "Keyboard enhancement (kitty) protocol: {} — Ctrl+I {} be distinct from Tab",
        if keyboard_enhanced {
            "supported"
        } else {
            "NOT supported"
        },
        if keyboard_enhanced {
            "will"
        } else {
            "will NOT"
        }
    ));

    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok((terminal, keyboard_enhanced))
}

/// Load the full initial application state from the database (nickname maps,
/// message history, known peers, receipts, groups) and set the fixed runtime
/// fields. Returns the state, the DB URL/label shown in the status bar, and the
/// group list (reused for re-subscription on startup).
fn initialize_state(
    topic_str: &str,
    own_nickname: String,
    keyboard_enhanced: bool,
) -> (
    super::state::AppState,
    String,
    Vec<p2p_app::groups::GroupSummary>,
) {
    let db_info = p2p_app::get_database_url();
    p2plog_debug(format!("Database: {db_info}"));
    p2plog_debug(format!("Loading data for topic: {topic_str}"));

    let (local_nicknames, received_nicknames, self_nicknames_for_peers) = p2p_app::load_peers()
        .map_or_else(
            |_| (HashMap::new(), HashMap::new(), HashMap::new()),
            |db_peers| extract_nickname_maps(&db_peers),
        );

    let (initial_messages, initial_message_ids) = super::state::load_and_format_messages(
        topic_str,
        super::state::MAX_MESSAGE_HISTORY,
        &local_nicknames,
        &received_nicknames,
        &own_nickname,
    );
    p2plog_debug(format!(
        "Loaded {} messages from database",
        initial_messages.len()
    ));

    // Load initial peers from database (including peer IDs that only exist in
    // messages), sorted by last_seen (latest first) — the format is
    // "YYYY-MM-DD HH:MM:SS" so lexicographic ordering works.
    let mut initial_peers = p2p_app::load_known_peers().map_or_else(
        |_| {
            p2plog_debug("No peers found in database".to_string());
            VecDeque::new()
        },
        |db_peers| {
            let peers = deduplicate_peers(&db_peers);
            p2plog_debug(format!(
                "Loaded {} unique peers from {} total database entries",
                peers.len(),
                db_peers.len()
            ));
            peers
        },
    );
    let mut peers_vec: Vec<_> = initial_peers.into_iter().collect();
    peers_vec.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
    initial_peers = peers_vec.into();

    let (loaded_broadcast_receipts, loaded_dm_receipts) = p2p_app::load_receipts().map_or_else(
        |_| (HashMap::new(), HashMap::new()),
        |receipts| {
            let (br, dr) = organize_receipts(&receipts);
            p2plog_debug(format!(
                "Loaded {} broadcast receipts and {} DM receipts from database",
                br.len(),
                dr.len()
            ));
            (br, dr)
        },
    );

    let local_peer_id =
        p2p_app::get_local_peer_id().map_or_else(|_| "unknown".to_string(), |id| id.to_string());

    // Load group list (known groups + member counts) from the database.
    let initial_groups = p2p_app::groups::list_groups_with_member_counts().unwrap_or_default();
    let mut state = super::state::AppState::new(
        topic_str.to_string(),
        own_nickname,
        local_peer_id,
        local_nicknames,
        received_nicknames,
        self_nicknames_for_peers,
        super::state::InitialStateData {
            messages: initial_messages,
            message_ids: initial_message_ids,
            peers: initial_peers,
            broadcast_receipts: loaded_broadcast_receipts,
            dm_receipts: loaded_dm_receipts,
            groups: initial_groups.clone(),
        },
    );
    state.env.kitty_keyboard_active = keyboard_enhanced;
    state.db_url.clone_from(&db_info);
    state.platform = format!("Desktop ({})", std::env::consts::OS);
    state.network_size =
        p2p_app::get_network_size().map_or_else(|_| "Unknown".to_string(), |n| n.to_string());
    (state, db_info, initial_groups)
}

/// Provision a user identity for this install (idempotent): creates the users
/// row keyed by a generated account keypair and self-binds the local device, so
/// the TUI is an account holder that can accept device-binding intros ('a' on
/// the prompt). Mirrors the mobile auto-provision at start. The announced
/// user-level nickname is seeded with the install's self nickname (ensured
/// earlier) so fresh accounts resolve to a name on the receiver side. Best
/// effort — a failure only degrades the account features, not the chat.
async fn provision_account(state: &super::state::SharedState) {
    let self_nickname = p2p_app::get_self_nickname().ok().flatten();
    if let Err(e) = p2p_app::identity::provision_my_user(None, self_nickname.as_deref()) {
        p2plog_debug(format!("Failed to provision user identity: {e}"));
    }
    state.lock().await.refresh_my_user();
}

/// Re-join every group persisted in the local database: the swarm only receives
/// group traffic for topics it is subscribed to, so saved groups must be
/// re-subscribed on every startup.
async fn resubscribe_saved_groups(
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    groups: &[p2p_app::groups::GroupSummary],
) {
    for summary in groups {
        p2plog_debug(format!(
            "Subscribing to saved group {} ({})",
            summary.group.display_name, summary.group.group_id
        ));
        let _ = swarm_cmd_tx
            .send(SwarmCommand::SubscribeGroup {
                group_id: summary.group.group_id.clone(),
            })
            .await;
    }
}

/// Wait for any of the four tasks to complete (indicating error or exit) and
/// log which one it was.
async fn await_first_task_exit(
    swarm_handler: tokio::task::JoinHandle<()>,
    event_source: tokio::task::JoinHandle<()>,
    command_processor: tokio::task::JoinHandle<()>,
    render_loop: tokio::task::JoinHandle<()>,
) -> &'static str {
    tokio::select! {
        result = swarm_handler => {
            if result.is_err() {
                p2plog_debug( "SwarmHandler panicked".to_string());
                "SwarmHandler task error"
            } else {
                p2plog_debug( "SwarmHandler exited".to_string());
                "SwarmHandler completed"
            }
        }
        result = event_source => {
            if result.is_err() {
                p2plog_debug( "EventSource panicked".to_string());
                "EventSource task error"
            } else {
                p2plog_debug( "EventSource exited".to_string());
                "EventSource completed"
            }
        }
        result = command_processor => {
            if result.is_err() {
                p2plog_debug( "CommandProcessor panicked".to_string());
                "CommandProcessor task error"
            } else {
                p2plog_debug( "CommandProcessor exited".to_string());
                "CommandProcessor completed"
            }
        }
        result = render_loop => {
            if result.is_err() {
                p2plog_debug( "RenderLoop panicked".to_string());
                "RenderLoop task error"
            } else {
                p2plog_debug( "RenderLoop exited".to_string());
                "RenderLoop completed"
            }
        }
    }
}

/// Restore the terminal after shutdown: pop the keyboard-enhancement flag and
/// defensively clear every keyboard mode while raw mode is still on (so the
/// escape sequences transmit cleanly), then leave the alternate screen and
/// restore termios.
///
/// We pushed only `DISAMBIGUATE_ESCAPE_CODES` at startup; on exit we pop it and
/// also emit `CSI > 4;0m` (xterm modifyOtherKeys off) in case any terminal left
/// it on. Otherwise the shell keeps receiving CSI-u/modified-key sequences
/// (arrow keys print as `A`..`D`, or junk like `;1u;5u…`) and is unusable.
fn restore_terminal(mut stdout: std::io::Stdout) {
    use ratatui::crossterm::{
        execute,
        terminal::{LeaveAlternateScreen, disable_raw_mode},
    };

    let _ = execute!(stdout, crossterm::event::PopKeyboardEnhancementFlags);
    let _ = write!(stdout, "\x1b[>4;0m");
    let _ = stdout.flush();
    let _ = execute!(stdout, LeaveAlternateScreen);
    let _ = execute!(stdout, crossterm::event::DisableMouseCapture);
    let _ = disable_raw_mode();
}

/// Main TUI loop: set up the terminal, initialize state, spawn the task
/// pipelines, and wait for any task to exit (indicating error or exit). The
/// function sets up terminal mode (alternate screen, raw mode, mouse capture)
/// and waits for the pipeline to finish, printing cached logs after restoring
/// the terminal.
///
/// Signal to trigger a render (instead of busy-polling) is carried by the
/// [`RenderEvent`] unit struct.
pub async fn run_new_tui(
    swarm: libp2p::swarm::Swarm<p2p_app::AppBehaviour>,
    topic_str: String,
) -> color_eyre::Result<()> {
    let (mut terminal, keyboard_enhanced) = setup_terminal()?;
    terminal.clear()?;

    // Initialize state
    let own_nickname = p2p_app::ensure_self_nickname().unwrap_or_else(|_| "Anonymous".to_string());
    let (state, _db_info, initial_groups) =
        initialize_state(&topic_str, own_nickname, keyboard_enhanced);
    let state = Arc::new(Mutex::new(state));

    // Setup channels
    let (input_tx, input_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let (render_tx, render_rx) = mpsc::channel(CHANNEL_CAPACITY);
    let render_tx_for_logs = render_tx.clone();
    set_tui_redraw_hook(move || {
        let _ = render_tx_for_logs.try_send(RenderEvent);
    });
    let _ = render_tx.send(RenderEvent).await;

    // Spawn tasks: SwarmHandler returns handle, event receiver, command sender
    let (swarm_handler, swarm_event_rx, swarm_cmd_tx) =
        p2p_app::spawn_swarm_handler(swarm, topic_str.clone());

    provision_account(&state).await;

    resubscribe_saved_groups(&swarm_cmd_tx, &initial_groups).await;

    // EventSource sends InputEvent to this channel
    let event_source = super::event_source::spawn_input_handler(input_tx);

    // CommandProcessor receives both InputEvent and SwarmEvent, sends RenderEvent and SwarmCommand
    let (command_processor, _) = super::command_processor::spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        render_tx,
        swarm_cmd_tx,
    );

    // RenderLoop reads state and renders
    let render_loop = super::render_loop::spawn_render_loop(state.clone(), terminal, render_rx);

    p2plog_debug("Started 4-task TUI architecture".to_string());

    // Wait for any task to complete (indicates error or exit)
    let exit_reason =
        await_first_task_exit(swarm_handler, event_source, command_processor, render_loop).await;
    p2plog_debug(format!("Initiating shutdown: {exit_reason}"));

    // Cleanup terminal state, then print cached logs after restoring the
    // terminal, then release the database lock file.
    restore_terminal(std::io::stdout());
    for log in recent_tui_logs(&get_tui_logs(), 100) {
        println!("{log}");
    }
    release_db_lock();

    Ok(())
}

#[cfg(test)]
#[path = "../../../tests/unit/unit_bin_tui_main_loop.rs"]
mod tests;
