#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unreachable,
    clippy::todo,
    clippy::unimplemented,
    clippy::significant_drop_tightening
)]

use crate::tui::event_source::InputEvent;
use crate::tui::main_loop::RenderEvent;
use crate::tui::state::{AppState, SharedState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use p2p_app::{DisplayMessage, PeerRecord, SwarmCommand};
use ratatui::{Terminal, backend::TestBackend, buffer::Cell};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};

/// Text and selection-highlight rows from one rendered frame.
struct FrameSnapshot {
    lines: Vec<String>,
    highlighted: Vec<bool>,
}

/// A real key-dispatch session with drained channels and a render probe.
///
/// The helpers drive `process_input_event`, the same public entry point the TUI
/// event loop uses; no production visibility is widened for tests. Render
/// notifications are drained after every key so bounded channels never block a
/// multi-key session.
pub struct KeySession {
    pub state: SharedState,
    pub commands: mpsc::Receiver<SwarmCommand>,
    command_tx: mpsc::Sender<SwarmCommand>,
    render_tx: mpsc::Sender<RenderEvent>,
    render_rx: mpsc::Receiver<RenderEvent>,
}

impl KeySession {
    pub fn new(state: AppState) -> Self {
        let (command_tx, commands) = mpsc::channel(64);
        let (render_tx, render_rx) = mpsc::channel(64);
        Self {
            state: Arc::new(Mutex::new(state)),
            commands,
            command_tx,
            render_tx,
            render_rx,
        }
    }

    /// Press one key through the real input event processor. Returns whether
    /// the app asked to exit.
    pub async fn press_key(&mut self, key: KeyEvent) -> bool {
        let exited = crate::tui::input_processor::process_input_event(
            InputEvent::Key(key),
            &self.state,
            &self.command_tx,
            &self.render_tx,
        )
        .await;
        self.drain_renders();
        exited
    }

    pub async fn press_char(&mut self, c: char) -> bool {
        self.press_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE))
            .await
    }

    pub async fn press_ctrl_char(&mut self, c: char) -> bool {
        self.press_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL))
            .await
    }

    pub async fn press_code(&mut self, code: KeyCode) -> bool {
        self.press_key(KeyEvent::new(code, KeyModifiers::NONE))
            .await
    }

    /// Send one mouse event through the same real input event processor the
    /// event loop uses. Returns whether the app asked to exit.
    pub async fn press_mouse(&mut self, mouse_event: MouseEvent) -> bool {
        let exited = crate::tui::input_processor::process_input_event(
            InputEvent::Mouse(mouse_event),
            &self.state,
            &self.command_tx,
            &self.render_tx,
        )
        .await;
        self.drain_renders();
        exited
    }

    /// Type a string as individual key events. Any exit request is returned so
    /// a session never silently ignores Ctrl+Q embedded in input.
    pub async fn type_text(&mut self, text: &str) -> bool {
        let mut exited = false;
        for c in text.chars() {
            exited |= self.press_char(c).await;
        }
        exited
    }

    /// Clone of the session's swarm sender, so a test can inject a production
    /// swarm event through the same channel the real event loop uses.
    pub fn command_tx(&self) -> mpsc::Sender<SwarmCommand> {
        self.command_tx.clone()
    }

    /// Collect the commands queued by the session so a test can assert on the
    /// full sequence without racing a background task.
    pub fn drain_commands(&mut self) -> Vec<SwarmCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.commands.try_recv() {
            commands.push(command);
        }
        commands
    }

    /// Project the current `AppState` through the same conversion and renderer
    /// the live frame loop uses, then return the visible buffer as text.
    pub async fn frame_text(&self) -> String {
        self.render_frame_snapshot().await.lines.join("\n")
    }

    /// Text of every frame row containing the active palette's selection
    /// background. This observes the real renderer's `ListState` highlight, so
    /// a key-driven cursor test does not have to trust state alone.
    pub async fn frame_highlighted_lines(&self) -> Vec<String> {
        let snapshot = self.render_frame_snapshot().await;
        snapshot
            .lines
            .into_iter()
            .zip(snapshot.highlighted)
            .filter_map(|(line, highlighted)| highlighted.then_some(line))
            .collect()
    }

    /// Render one frame and extract both text and highlight rows from it.
    async fn render_frame_snapshot(&self) -> FrameSnapshot {
        let state = self.state.lock().await;
        let background = state.palette.selection_bg();
        let backend = TestBackend::new(160, 40);
        let mut terminal = Terminal::new(backend).expect("test terminal");
        terminal
            .draw(|frame| crate::tui::render_loop::render_frame_for_test(frame, &state))
            .expect("render frame");
        drop(state);
        let buffer = terminal.backend().buffer();
        let width = usize::from(buffer.area.width);
        let mut lines = Vec::with_capacity(buffer.content.len().div_ceil(width.max(1)));
        let mut highlighted = Vec::with_capacity(lines.capacity());
        for row in buffer.content.chunks(width) {
            lines.push(
                row.iter()
                    .map(Cell::symbol)
                    .collect::<String>()
                    .trim_end()
                    .to_owned(),
            );
            highlighted.push(row.iter().any(|cell| cell.bg == background));
        }
        FrameSnapshot { lines, highlighted }
    }

    fn drain_renders(&mut self) {
        while self.render_rx.try_recv().is_ok() {}
    }
}

pub fn test_app_state() -> AppState {
    AppState::new(
        "test-net".to_string(),
        "TestUser".to_string(),
        "test-peer-id-1234".to_string(),
        HashMap::new(),
        HashMap::new(),
        HashMap::new(),
        crate::tui::state::InitialStateData::default(),
    )
}

pub fn app_state_with_chat_messages(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        state.messages.push_back(DisplayMessage {
            text: format!("Message {i}"),
            sender_peer_id: Some(format!("peer{i}")),
        });
        state.message_ids.push_back(Some(format!("msg-{i}")));
        // Test rows are not DB-backed, so local deletion must be a no-op on
        // them (US-005). Keep the queue aligned even in pure in-memory tests.
        state.message_db_ids.push_back(None);
    }
    state
}

pub fn app_state_with_peers(count: usize) -> AppState {
    let mut state = test_app_state();
    for i in 0..count {
        let id = format!("peer{i}");
        state.peers.push_back(PeerRecord {
            peer_id: id,
            first_seen: "2024-01-01 12:00:00".into(),
            last_seen: "2024-01-01 12:00:00".into(),
        });
    }
    // Mirror the production peer-add path (`command_processor` pushes to the
    // live deque then calls `resort_peers`), so `contacts` is a faithful view
    // and the Contacts tab tests see the same sort the app does.
    state.resort_peers();
    state
}

pub fn app_state_with_dm_messages(peer_id: &str, count: usize) -> AppState {
    let mut state = test_app_state();
    state.dynamic_tabs.add_dm_tab(peer_id.to_string());
    let msgs: VecDeque<String> = (0..count).map(|i| format!("DM message {i}")).collect();
    state.dm_messages.insert(peer_id.to_string(), msgs);
    state.dm_message_ids.insert(
        peer_id.to_string(),
        (0..count).map(|i| Some(format!("dm-{i}"))).collect(),
    );
    state
        .dm_message_db_ids
        .insert(peer_id.to_string(), (0..count).map(|_| None).collect());
    state.dm_scroll_state.insert(peer_id.to_string(), (0, true));
    state
        .dm_broadcast_scroll_state
        .insert(peer_id.to_string(), (0, true));
    state
}
