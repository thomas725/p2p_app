# TUI Task Architecture Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace monolithic `tokio::select!` with 4 independent async tasks (SwarmHandler, CommandProcessor, InputHandler, RenderLoop) communicating via channels and Arc<Mutex<AppState>>.

**Architecture:** SwarmHandler processes libp2p events. CommandProcessor receives input + swarm events, updates shared AppState, sends network commands. InputHandler polls terminal. RenderLoop reads state and renders. Each task runs independently in tokio::spawn.

**Tech Stack:** tokio (async tasks), mpsc channels, Arc<Mutex<>> for shared state, libp2p, ratatui

---

## Phase 1: Define Shared Types

### Task 1: Create types module with SwarmEvent and SwarmCommand

**Files:**
- Create: `src/types.rs`
- Modify: `src/lib.rs` (add `pub mod types` and exports)

- [ ] **Step 1: Create src/types.rs with SwarmEvent enum**

```rust
use libp2p::Multiaddr;

/// High-level application events from the swarm
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Broadcast message received from peer
    BroadcastMessage {
        content: String,
        peer_id: String,
        latency: Option<String>,
    },
    /// Direct message received from peer
    DirectMessage {
        content: String,
        peer_id: String,
        latency: Option<String>,
    },
    /// Peer connected to the network
    PeerConnected(String),
    /// Peer disconnected from the network
    PeerDisconnected(String),
    /// Local address established for listening
    ListenAddrEstablished(String),
    /// Peer discovered via mDNS
    #[cfg(feature = "mdns")]
    PeerDiscovered {
        peer_id: String,
        addresses: Vec<Multiaddr>,
    },
    /// Peer expired via mDNS
    #[cfg(feature = "mdns")]
    PeerExpired { peer_id: String },
}

/// Commands sent to the swarm task
#[derive(Debug, Clone)]
pub enum SwarmCommand {
    /// Publish a message to the broadcast topic
    Publish(String),
    /// Send a direct message to a peer
    SendDm { peer_id: String, content: String },
}
```

- [ ] **Step 2: Add pub mod types to src/lib.rs**

Read `src/lib.rs` and find the line with `pub mod` declarations (around line 30-40). Add:

```rust
pub mod types;
```

And export the types at the end of the `pub use` block:

```rust
pub use types::{SwarmCommand, SwarmEvent};
```

- [ ] **Step 3: Compile and verify**

```bash
cargo check 2>&1 | grep -E "(error|warning: unused|Finished|Compiling)"
```

Expected: `Finished` or warnings only (no errors)

- [ ] **Step 4: Commit**

```bash
git add src/types.rs src/lib.rs
git commit -m "feat: define SwarmEvent and SwarmCommand types for inter-task communication"
```

---

## Phase 2: Extract SwarmHandler Task

### Task 2: Create SwarmHandler that translates libp2p events to SwarmEvent

**Files:**
- Create: `src/swarm_handler.rs`
- Modify: `src/lib.rs` (add `pub mod swarm_handler`)

- [ ] **Step 1: Create src/swarm_handler.rs with spawn_swarm_handler function**

```rust
use crate::types::SwarmEvent;
use crate::{AppBehaviour, AppBehaviourEvent as AppEv};
use libp2p::swarm::{Swarm, SwarmEvent as Libp2pSwarmEvent};
use libp2p::gossipsub;
use libp2p::futures::StreamExt;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Spawns the swarm handler task that processes libp2p events
/// and translates them to app-level SwarmEvent messages.
pub fn spawn_swarm_handler(
    mut swarm: Swarm<AppBehaviour>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Receiver<SwarmEvent>) {
    let (tx, rx) = mpsc::channel(100);

    let handle = tokio::spawn(async move {
        loop {
            match swarm.select_next_some().await {
                Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message,
                        ..
                    },
                )) => {
                    let peer_id_str = peer_id.to_string();
                    let raw = String::from_utf8_lossy(&message.data).to_string();

                    // Try to parse as BroadcastMessage
                    if let Ok(bcast) = serde_json::from_str::<crate::BroadcastMessage>(&raw) {
                        let content = bcast.content.clone();
                        let latency = bcast.sent_at.map(|sent| {
                            let now = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64();
                            let elapsed = now - sent;
                            crate::format_latency(elapsed)
                        });

                        let _ = tx
                            .send(SwarmEvent::BroadcastMessage {
                                content,
                                peer_id: peer_id_str.clone(),
                                latency,
                            })
                            .await;
                    }
                }
                Libp2pSwarmEvent::Behaviour(AppEv::RequestResponse(
                    libp2p::request_response::Event::Message {
                        peer,
                        message: libp2p::request_response::Message::Request {
                            request,
                            channel,
                            ..
                        },
                    },
                )) => {
                    let peer_id_str = peer.to_string();
                    if let Ok(dm) = serde_json::from_slice::<crate::DirectMessage>(&request) {
                        let content = dm.content.clone();
                        let latency = dm.sent_at.map(|sent| {
                            let now = SystemTime::now()
                                .duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs_f64();
                            let elapsed = now - sent;
                            crate::format_latency(elapsed)
                        });

                        let _ = tx
                            .send(SwarmEvent::DirectMessage {
                                content,
                                peer_id: peer_id_str.clone(),
                                latency,
                            })
                            .await;

                        // Send ACK response
                        let response = crate::DirectMessage {
                            content: "ok".to_string(),
                            timestamp: chrono::Utc::now().timestamp(),
                            sent_at: Some(
                                SystemTime::now()
                                    .duration_since(SystemTime::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs_f64()
                            ),
                            nickname: None,
                        };
                        let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                    }
                }
                #[cfg(feature = "mdns")]
                Libp2pSwarmEvent::Behaviour(AppEv::Mdns(
                    libp2p::mdns::Event::Discovered(list),
                )) => {
                    for (peer_id, multiaddr) in list {
                        let _ = tx
                            .send(SwarmEvent::PeerDiscovered {
                                peer_id: peer_id.to_string(),
                                addresses: vec![multiaddr.clone()],
                            })
                            .await;
                        swarm.dial(multiaddr.clone()).ok();
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    }
                }
                Libp2pSwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    let _ = tx.send(SwarmEvent::PeerConnected(peer_id.to_string())).await;
                }
                Libp2pSwarmEvent::ConnectionClosed { peer_id, .. } => {
                    let _ = tx
                        .send(SwarmEvent::PeerDisconnected(peer_id.to_string()))
                        .await;
                }
                Libp2pSwarmEvent::NewListenAddr { address, .. } => {
                    let _ = tx
                        .send(SwarmEvent::ListenAddrEstablished(address.to_string()))
                        .await;
                }
                _ => {}
            }
        }
    });

    (handle, rx)
}
```

- [ ] **Step 2: Add pub mod swarm_handler to src/lib.rs**

Find the `pub mod` declarations section and add:

```rust
pub mod swarm_handler;
```

Export the spawner:

```rust
pub use swarm_handler::spawn_swarm_handler;
```

- [ ] **Step 3: Compile and verify**

```bash
cargo check 2>&1 | head -50
```

Expected: Compiles successfully or shows fixable issues

- [ ] **Step 4: Commit**

```bash
git add src/swarm_handler.rs src/lib.rs
git commit -m "feat: extract swarm handler task that translates libp2p events to app-level SwarmEvent"
```

---

## Phase 3: Create Shared AppState

### Task 3: Define AppState struct

**Files:**
- Create: `bin/tui/state.rs`
- Modify: `bin/p2p_chat_tui.rs` (add `mod tui; use tui::state::AppState`)

- [ ] **Step 1: Create bin/tui/state.rs with AppState struct**

```rust
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use p2p_app::DynamicTabs;
use ratatui_textarea::TextArea;

/// Shared application state for all tasks
pub struct AppState {
    // Messages & Chat
    pub messages: VecDeque<(String, Option<String>)>,
    pub dm_messages: HashMap<String, VecDeque<String>>,

    // Peer Management
    pub peers: VecDeque<(String, String, String)>, // (id, first_seen, last_seen)
    pub concurrent_peers: usize,
    pub local_nicknames: HashMap<String, String>,
    pub received_nicknames: HashMap<String, String>,

    // UI State (TUI-specific)
    pub active_tab: usize,
    pub dynamic_tabs: DynamicTabs,
    pub chat_input: TextArea<'static>,
    pub dm_inputs: HashMap<String, TextArea<'static>>,
    pub peer_selection: usize,
    pub mouse_capture: bool,

    // Scroll State
    pub debug_scroll_offset: usize,
    pub debug_auto_scroll: bool,
    pub chat_scroll_offset: usize,
    pub chat_auto_scroll: bool,

    // Unread Counts
    pub unread_broadcasts: u32,
    pub unread_dms: HashMap<String, u32>,

    // Runtime Context
    pub own_nickname: String,
    pub topic_str: String,
    pub logs: Arc<Mutex<VecDeque<String>>>,
}

impl AppState {
    /// Create a new AppState with loaded data from database
    pub fn new(
        own_nickname: String,
        topic_str: String,
        logs: Arc<Mutex<VecDeque<String>>>,
    ) -> Self {
        let mut state = Self {
            messages: VecDeque::new(),
            dm_messages: HashMap::new(),
            peers: VecDeque::new(),
            concurrent_peers: 0,
            local_nicknames: HashMap::new(),
            received_nicknames: HashMap::new(),
            active_tab: 0,
            dynamic_tabs: DynamicTabs::new(),
            chat_input: TextArea::default(),
            dm_inputs: HashMap::new(),
            peer_selection: 0,
            mouse_capture: false,
            debug_scroll_offset: 0,
            debug_auto_scroll: true,
            chat_scroll_offset: 0,
            chat_auto_scroll: true,
            unread_broadcasts: 0,
            unread_dms: HashMap::new(),
            own_nickname,
            topic_str,
            logs,
        };

        // Load persisted data from database
        if let Ok(messages) = p2p_app::load_messages() {
            for msg in messages {
                state.messages.push_back((msg.content, msg.peer_id));
            }
        }

        if let Ok(peers) = p2p_app::load_peers() {
            let mut peer_indices: Vec<usize> = (0..peers.len()).collect();
            peer_indices
                .sort_by(|&a, &b| peers[b].last_seen.cmp(&peers[a].last_seen));
            for &idx in peer_indices.iter().take(10) {
                let peer = &peers[idx];
                let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                if let Some(ref local_nick) = peer.peer_local_nickname {
                    state
                        .local_nicknames
                        .insert(peer.peer_id.clone(), local_nick.clone());
                }
                if let Some(ref recv_nick) = peer.received_nickname {
                    state
                        .received_nicknames
                        .insert(peer.peer_id.clone(), recv_nick.clone());
                }
                state
                    .peers
                    .push_back((peer.peer_id.to_string(), first_seen, last_seen));
            }
        }

        if let Ok(dms) = p2p_app::load_direct_messages() {
            for (peer_id, messages) in dms {
                let mut msgs = VecDeque::new();
                for msg in messages {
                    msgs.push_back(msg.content);
                }
                state.dm_messages.insert(peer_id, msgs);
            }
        }

        state
    }
}
```

- [ ] **Step 2: Modify bin/p2p_chat_tui.rs to declare tui module and import AppState**

At the top of the `#[cfg(feature = "tui")]` mod tui block (around line 17), verify this exists:

```rust
#[cfg(feature = "tui")]
mod tui {
    use super::*;
    // ... existing imports ...
    mod state;
    pub use state::AppState;
```

If `mod state;` doesn't exist, add it. The `pub use state::AppState;` makes it available in the tui module.

- [ ] **Step 3: Compile and verify**

```bash
cargo check 2>&1 | head -50
```

Expected: Compiles or shows import/type errors to fix

- [ ] **Step 4: Commit**

```bash
git add bin/tui/state.rs src/bin/p2p_chat_tui.rs
git commit -m "feat: create AppState struct for shared application state"
```

---

## Phase 4: Extract CommandProcessor Task

### Task 4: Create CommandProcessor that handles business logic

**Files:**
- Create: `src/command_processor.rs`
- Modify: `src/lib.rs` (add `pub mod command_processor`)

**Note:** This is the most complex task. The processor receives InputEvent (from bin) and SwarmEvent (from lib), updates AppState, and sends SwarmCommand back to SwarmHandler.

- [ ] **Step 1: Create src/command_processor.rs with spawn_command_processor function**

```rust
use crate::types::{SwarmCommand, SwarmEvent};
use libp2p::gossipsub;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;
use tokio::sync::mpsc;

/// Marker for input events (defined in bin, but we use a generic channel)
/// This is sent via mpsc::Receiver by the InputHandler
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(String),       // Simplified for now; bin will expand this
    Mouse(String),     // Simplified for now
}

/// Spawns the command processor task that:
/// - Receives InputEvent (from InputHandler) and SwarmEvent (from SwarmHandler)
/// - Updates AppState accordingly
/// - Sends SwarmCommand back to SwarmHandler when needed
pub fn spawn_command_processor(
    state: Arc<Mutex<crate::TuiAppState>>,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let (swarm_cmd_tx, swarm_cmd_rx) = mpsc::channel(100);
    let swarm_cmd_tx_inner = swarm_cmd_tx.clone();

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(_input_event) = input_rx.recv() => {
                    // Input events handled here
                    // For now, just log that we received an input
                    if let Ok(mut l) = logs.lock() {
                        l.push_back("Input event received".to_string());
                        if l.len() > 100 {
                            l.pop_front();
                        }
                    }
                }
                Some(swarm_event) = swarm_event_rx.recv() => {
                    // Process swarm events and update state
                    match swarm_event {
                        SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = SystemTime::now();
                                let ts = crate::format_system_time(now);
                                let sender_display = crate::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                s.messages.push_back((msg.clone(), Some(peer_id.clone())));
                                if s.messages.len() > 1000 {
                                    s.messages.pop_front();
                                }
                                if s.active_tab != 0 {
                                    s.unread_broadcasts += 1;
                                }
                                if let Err(e) = crate::save_message(&content, Some(&peer_id), &s.topic_str, false, None) {
                                    crate::log_debug(&logs, format!("Failed to save message: {}", e));
                                }
                            }
                        }
                        SwarmEvent::DirectMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = SystemTime::now();
                                let ts = crate::format_system_time(now);
                                let sender_display = crate::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                dm_msgs.push_back(msg.clone());
                                if dm_msgs.len() > 1000 {
                                    dm_msgs.pop_front();
                                }
                                *s.unread_dms.entry(peer_id.clone()).or_insert(0) += 1;
                                s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                if let Err(e) = crate::save_message(&content, Some(&peer_id), &s.topic_str, true, Some(&peer_id)) {
                                    crate::log_debug(&logs, format!("Failed to save DM: {}", e));
                                }
                            }
                        }
                        SwarmEvent::PeerConnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers += 1;
                                crate::log_debug(&logs, format!("Peer connected: {}", peer_id));
                            }
                        }
                        SwarmEvent::PeerDisconnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
                                crate::log_debug(&logs, format!("Peer disconnected: {}", peer_id));
                            }
                        }
                        SwarmEvent::ListenAddrEstablished(addr) => {
                            crate::log_debug(&logs, format!("Listening on: {}", addr));
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerDiscovered { peer_id, addresses } => {
                            if let Ok(mut s) = state.lock() {
                                if !s.peers.iter().any(|(id, _, _)| id == &peer_id) {
                                    s.peers.push_front((peer_id.clone(), crate::now_timestamp(), crate::now_timestamp()));
                                }
                            }
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerExpired { peer_id } => {
                            crate::log_debug(&logs, format!("Peer expired: {}", peer_id));
                        }
                    }
                }
            }
        }
    });

    (handle, swarm_cmd_tx_inner)
}
```

Note: This is a simplified version. The actual task will be expanded in later work to handle all InputEvent types.

- [ ] **Step 2: Add pub mod command_processor to src/lib.rs**

Find the `pub mod` declarations and add:

```rust
pub mod command_processor;
```

Export:

```rust
pub use command_processor::spawn_command_processor;
```

- [ ] **Step 3: Add TuiAppState type alias in src/lib.rs**

At the top of the file (after imports), add a re-export for the state type so other modules can reference it. Add this after the imports:

```rust
// Type alias for TUI app state (defined in bin but referenced from lib)
pub type TuiAppState = std::collections::VecDeque<String>; // Placeholder; will be properly typed
```

Actually, this is tricky because AppState is defined in bin, not lib. Let me revise:

- [ ] **Step 3 (Revised): Don't add TuiAppState to lib yet**

The CommandProcessor will be generic over state type. For now, we'll use a concrete type in the bin code. Skip this step.

- [ ] **Step 4: Compile and check for errors**

```bash
cargo check --bin p2p_chat_tui 2>&1 | head -100
```

Expected: May show errors about undefined types; we'll fix those when we wire up the tasks

- [ ] **Step 5: Commit**

```bash
git add src/command_processor.rs src/lib.rs
git commit -m "feat: extract command processor task for handling business logic"
```

---

## Phase 5: Create InputHandler Task

### Task 5: Create InputHandler that polls terminal events

**Files:**
- Create: `bin/tui/input_handler.rs`
- Modify: `bin/p2p_chat_tui.rs` (add `mod input_handler; use tui::input_handler::spawn_input_handler`)

- [ ] **Step 1: Create bin/tui/input_handler.rs**

```rust
use crossterm::event::{poll, read, Event, KeyEvent, MouseEvent};
use std::time::Duration;
use tokio::sync::mpsc;

/// Input event type for terminal I/O
#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// Spawns the input handler task that polls terminal events
pub fn spawn_input_handler(
    input_tx: mpsc::Sender<InputEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            // Poll with 16ms timeout (60 FPS)
            if poll(Duration::from_millis(16)).ok() == Some(true) {
                if let Ok(event) = read() {
                    match event {
                        Event::Key(key) => {
                            let _ = input_tx.send(InputEvent::Key(key)).await;
                        }
                        Event::Mouse(mouse) => {
                            let _ = input_tx.send(InputEvent::Mouse(mouse)).await;
                        }
                        _ => {}
                    }
                }
            }
            // Yield to async runtime
            tokio::task::yield_now().await;
        }
    })
}
```

- [ ] **Step 2: Update bin/p2p_chat_tui.rs to declare input_handler module**

In the `#[cfg(feature = "tui")] mod tui { ... }` block, add:

```rust
mod input_handler;
pub use input_handler::{spawn_input_handler, InputEvent};
```

- [ ] **Step 3: Compile and verify**

```bash
cargo check --bin p2p_chat_tui 2>&1 | head -50
```

Expected: Should compile or show minimal errors

- [ ] **Step 4: Commit**

```bash
git add bin/tui/input_handler.rs src/bin/p2p_chat_tui.rs
git commit -m "feat: create input handler task that polls terminal events"
```

---

## Phase 6: Create RenderLoop Task

### Task 6: Create RenderLoop that renders terminal UI

**Files:**
- Create: `bin/tui/render_loop.rs`
- Modify: `bin/p2p_chat_tui.rs` (add `mod render_loop; use tui::render_loop::spawn_render_loop`)

- [ ] **Step 1: Create bin/tui/render_loop.rs (simplified version)**

```rust
use p2p_app::DynamicTabs;
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph, Tabs},
};
use std::io::Stdout;
use std::sync::{Arc, Mutex};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

/// Spawns the render loop task that continuously renders the TUI
pub fn spawn_render_loop(
    state: Arc<Mutex<crate::state::AppState>>,
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
                    let tabs = Tabs::new(tab_titles.clone())
                        .style(Style::default().fg(Color::Cyan))
                        .select(s.active_tab);
                    f.render_widget(tabs, chunks[0]);

                    // Render main content (simplified)
                    let content_block = Block::default().title("Messages");
                    f.render_widget(content_block, chunks[2]);

                    // Render input area
                    let input_block = Block::default().title("Input");
                    f.render_widget(input_block, chunks[3]);
                } else {
                    // Render error state
                    let para = Paragraph::new("Failed to acquire state lock");
                    f.render_widget(para, f.area());
                }
            });
        }
    })
}
```

This is a simplified render loop. In Phase 7, we'll flesh out the actual rendering based on existing code.

- [ ] **Step 2: Update bin/p2p_chat_tui.rs to declare render_loop module**

In the `#[cfg(feature = "tui")] mod tui { ... }` block, add:

```rust
mod render_loop;
pub use render_loop::spawn_render_loop;
```

- [ ] **Step 3: Compile and verify**

```bash
cargo check --bin p2p_chat_tui 2>&1 | head -50
```

Expected: Should compile or show minimal errors

- [ ] **Step 4: Commit**

```bash
git add bin/tui/render_loop.rs src/bin/p2p_chat_tui.rs
git commit -m "feat: create render loop task that renders TUI at 60 FPS"
```

---

## Phase 7: Wire Up Main Function

### Task 7: Rewrite main to spawn 4 tasks and coordinate them

**Files:**
- Modify: `bin/p2p_chat_tui.rs` (rewrite run_tui function and main)

- [ ] **Step 1: Replace the main tui::run_tui function**

Find the `pub async fn run_tui` function (around line 330) and replace it with:

```rust
pub async fn run_tui(
    swarm: Swarm<AppBehaviour>,
    topic_str: String,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> color_eyre::Result<()> {
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
    let own_nickname = get_self_nickname()?.unwrap_or_else(|_| "Anonymous".to_string());
    let state = Arc::new(Mutex::new(AppState::new(
        own_nickname.clone(),
        topic_str.clone(),
        logs.clone(),
    )));

    // Setup channels
    let (input_tx, input_rx) = mpsc::channel(100);
    let (swarm_event_tx, swarm_event_rx) = mpsc::channel(100);

    // Spawn tasks
    let (swarm_handler, _) = spawn_swarm_handler(swarm, logs.clone());
    let (input_handler, _) = spawn_input_handler(input_tx);
    let (command_processor, _) = spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        logs.clone(),
    );
    let (render_loop, _) = spawn_render_loop(state.clone(), terminal);

    // TODO: Wire up channels between tasks

    // Wait for all tasks
    tokio::select! {
        _ = swarm_handler => { log_debug(&logs, "SwarmHandler exited".to_string()); }
        _ = input_handler => { log_debug(&logs, "InputHandler exited".to_string()); }
        _ = command_processor => { log_debug(&logs, "CommandProcessor exited".to_string()); }
        _ = render_loop => { log_debug(&logs, "RenderLoop exited".to_string()); }
    }

    // Cleanup
    exit_tui()?;
    Ok(())
}
```

- [ ] **Step 2: Update imports at top of tui module**

Verify these imports exist:

```rust
use spawn_swarm_handler;
use spawn_command_processor;
use input_handler::spawn_input_handler;
use render_loop::spawn_render_loop;
use state::AppState;
```

Actually, since these are in modules, use:

```rust
use crate::spawn_swarm_handler;
use crate::spawn_command_processor;
```

Add these to the imports section (around line 42-43 in the tui module).

- [ ] **Step 3: Compile and check errors**

```bash
cargo check --bin p2p_chat_tui 2>&1
```

This will likely fail because we haven't properly connected everything. Expected errors about:
- Undefined functions
- Type mismatches on state

- [ ] **Step 4: Commit (even if not fully working yet)**

```bash
git add src/bin/p2p_chat_tui.rs
git commit -m "WIP: wire up main to spawn 4 tasks (incomplete)"
```

---

## Phase 8: Fix Type Mismatches and Connect Channels

### Task 8: Fix remaining compilation errors

**Files:**
- Modify: `src/lib.rs` (add proper type definitions)
- Modify: `src/command_processor.rs` (use correct state type)
- Modify: `bin/p2p_chat_tui.rs` (fix imports and types)

This task focuses on getting the code to compile. The exact errors will depend on what the compiler reports.

- [ ] **Step 1: Run cargo check and capture full errors**

```bash
cargo check --bin p2p_chat_tui 2>&1 > /tmp/errors.txt
cat /tmp/errors.txt
```

Review the errors and determine:
1. Missing type definitions
2. Incorrect function signatures
3. Missing imports

- [ ] **Step 2: Fix command_processor.rs to work with TUI's AppState**

The CommandProcessor currently uses a generic InputEvent. We need to connect it to the actual InputEvent from input_handler. The key insight: since AppState is TUI-specific (defined in bin), the CommandProcessor needs to be generic or accept Arc<Mutex<dyn Any>>.

For simplicity, let's make CommandProcessor work with the concrete TUI state. Modify src/command_processor.rs spawn_command_processor signature:

```rust
pub fn spawn_command_processor<S>(
    state: Arc<Mutex<S>>,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>)
where
    S: Send + Sync + 'static,
```

Actually, this gets complex. For now, let's make a simpler version that works with the TUI state directly. Remove the generic and use a concrete type.

Better approach: Define a trait or just pass `Arc<Mutex<dyn Any>>` and downcast. But that's also complex.

**Simplest approach:** Since AppState is TUI-specific anyway, let's just not use CommandProcessor from lib initially. Instead, move the command processing logic to the bin code.

Actually, let me reconsider the architecture per the design: CommandProcessor should be in lib but take a generic state. Let's use downcast_any:

- [ ] **Step 2 (Revised): Create a trait for state that CommandProcessor can work with**

Add to src/command_processor.rs (or new src/app_state_trait.rs):

```rust
pub trait AppStateT: Send + Sync {
    // Methods to get/set state values
}
```

But this is getting complex. Let's simplify: for this implementation phase, move CommandProcessor to bin/tui/command_processor.rs instead, working with the concrete AppState type.

- [ ] **Step 2 (Further Revised): Move command_processor to bin for now**

Delete src/command_processor.rs and recreate as bin/tui/command_processor.rs with proper types. This is a tactical simplification.

```bash
rm src/command_processor.rs
```

Create bin/tui/command_processor.rs:

```rust
use p2p_app::{SwarmCommand, SwarmEvent};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub enum InputEvent {
    Key(String),
    Mouse(String),
}

pub fn spawn_command_processor(
    state: Arc<Mutex<AppState>>,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(100);

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(_input_event) = input_rx.recv() => {
                    // Input handling
                }
                Some(swarm_event) = swarm_event_rx.recv() => {
                    // Swarm event handling
                    match swarm_event {
                        SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let ts = p2p_app::format_system_time(std::time::SystemTime::now());
                                let msg = format!("{} [{}] {}", ts, peer_id, content);
                                s.messages.push_back((msg, Some(peer_id.clone())));
                                if s.messages.len() > 1000 {
                                    s.messages.pop_front();
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    (handle, swarm_cmd_tx)
}
```

- [ ] **Step 3: Remove src/command_processor.rs from lib.rs**

Remove from src/lib.rs:
```rust
pub mod command_processor;
pub use command_processor::spawn_command_processor;
```

- [ ] **Step 4: Update bin/p2p_chat_tui.rs to import from bin::tui::command_processor**

In the tui module, add:

```rust
mod command_processor;
pub use command_processor::{spawn_command_processor, InputEvent as InputEventCmd};
```

- [ ] **Step 5: Update run_tui to use correct imports**

```rust
pub async fn run_tui(
    swarm: Swarm<AppBehaviour>,
    topic_str: String,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> color_eyre::Result<()> {
    // ... setup code ...

    let (input_tx, input_rx) = mpsc::channel(100);
    let (swarm_event_tx, swarm_event_rx) = mpsc::channel(100);

    let (swarm_handler, _) = crate::spawn_swarm_handler(swarm, logs.clone());
    let (input_handler, _) = spawn_input_handler(input_tx);
    let (command_processor, _) = spawn_command_processor(state.clone(), input_rx, swarm_event_rx, logs.clone());
    let (render_loop, _) = spawn_render_loop(state.clone(), terminal);

    // ...
}
```

- [ ] **Step 6: Compile and verify**

```bash
cargo check --bin p2p_chat_tui 2>&1 | head -100
```

Expected: Should get closer to compiling; may have remaining errors

- [ ] **Step 7: Commit**

```bash
git add bin/tui/command_processor.rs src/bin/p2p_chat_tui.rs src/lib.rs
git commit -m "fix: move command processor to bin, fix type mismatches"
```

---

## Phase 9: Complete Channel Wiring

### Task 9: Connect swarm handler output to command processor input

**Files:**
- Modify: `bin/p2p_chat_tui.rs` (connect swarm_event_rx from swarm handler)

The current code spawns tasks but doesn't wire the channels together. SwarmHandler produces SwarmEvent that should go to CommandProcessor.

- [ ] **Step 1: Update run_tui to handle swarm handler channel**

```rust
pub async fn run_tui(
    swarm: Swarm<AppBehaviour>,
    topic_str: String,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> color_eyre::Result<()> {
    // ... setup code ...

    let (input_tx, input_rx) = mpsc::channel(100);

    // Spawn swarm handler and get its event receiver
    let (swarm_handler, swarm_event_rx) = crate::spawn_swarm_handler(swarm, logs.clone());

    let (input_handler, _) = spawn_input_handler(input_tx);
    let (command_processor, _) = spawn_command_processor(state.clone(), input_rx, swarm_event_rx, logs.clone());
    let (render_loop, _) = spawn_render_loop(state.clone(), terminal);

    // Wait for all tasks
    tokio::select! {
        _ = swarm_handler => { log_debug(&logs, "SwarmHandler exited".to_string()); }
        _ = input_handler => { log_debug(&logs, "InputHandler exited".to_string()); }
        _ = command_processor => { log_debug(&logs, "CommandProcessor exited".to_string()); }
        _ = render_loop => { log_debug(&logs, "RenderLoop exited".to_string()); }
    }

    exit_tui()?;
    Ok(())
}
```

The key change: pass `swarm_event_rx` (from swarm handler) directly to CommandProcessor.

- [ ] **Step 2: Compile and test**

```bash
cargo build --bin p2p_chat_tui 2>&1 | tail -20
```

Expected: May still have errors; if it compiles, great!

- [ ] **Step 3: Commit**

```bash
git add src/bin/p2p_chat_tui.rs
git commit -m "feat: wire swarm handler output to command processor"
```

---

## Phase 10: Flesh Out CommandProcessor Logic

### Task 10: Implement actual business logic in CommandProcessor

**Files:**
- Modify: `bin/tui/command_processor.rs` (implement SwarmEvent handling)

Currently CommandProcessor only stubs out event handling. Now we implement the full logic from the old code.

- [ ] **Step 1: Update CommandProcessor swarm event handling to save messages**

In bin/tui/command_processor.rs, expand the match statement:

```rust
pub fn spawn_command_processor(
    state: Arc<Mutex<AppState>>,
    mut input_rx: mpsc::Receiver<InputEvent>,
    mut swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (tokio::task::JoinHandle<()>, mpsc::Sender<SwarmCommand>) {
    let (swarm_cmd_tx, _swarm_cmd_rx) = mpsc::channel(100);

    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(_input_event) = input_rx.recv() => {
                    // Input handling - TODO
                }
                Some(swarm_event) = swarm_event_rx.recv() => {
                    match swarm_event {
                        SwarmEvent::BroadcastMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = std::time::SystemTime::now();
                                let ts = p2p_app::format_system_time(now);
                                let sender_display = p2p_app::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                s.messages.push_back((msg.clone(), Some(peer_id.clone())));
                                if s.messages.len() > 1000 {
                                    s.messages.pop_front();
                                }
                                if s.active_tab != 0 {
                                    s.unread_broadcasts += 1;
                                }
                                if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, false, None) {
                                    p2p_app::log_debug(&logs, format!("Failed to save message: {}", e));
                                }
                            }
                        }
                        SwarmEvent::DirectMessage { content, peer_id, latency } => {
                            if let Ok(mut s) = state.lock() {
                                let now = std::time::SystemTime::now();
                                let ts = p2p_app::format_system_time(now);
                                let sender_display = p2p_app::peer_display_name(
                                    &peer_id,
                                    &s.local_nicknames,
                                    &s.received_nicknames,
                                );
                                let dm_msgs = s.dm_messages.entry(peer_id.clone()).or_default();
                                let msg = format!(
                                    "{} {} [{}] {}",
                                    ts,
                                    latency.unwrap_or_default(),
                                    sender_display,
                                    content
                                );
                                dm_msgs.push_back(msg);
                                if dm_msgs.len() > 1000 {
                                    dm_msgs.pop_front();
                                }
                                *s.unread_dms.entry(peer_id.clone()).or_insert(0) += 1;
                                s.dynamic_tabs.add_dm_tab(peer_id.clone());
                                if let Err(e) = p2p_app::save_message(&content, Some(&peer_id), &s.topic_str, true, Some(&peer_id)) {
                                    p2p_app::log_debug(&logs, format!("Failed to save DM: {}", e));
                                }
                            }
                        }
                        SwarmEvent::PeerConnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers += 1;
                                p2p_app::log_debug(&logs, format!("Peer connected: {} (total: {})", peer_id, s.concurrent_peers));
                                let addresses = vec![peer_id.clone()];
                                if let Ok(peer) = p2p_app::save_peer(&peer_id, &addresses) {
                                    let first_seen = p2p_app::format_peer_datetime(peer.first_seen);
                                    let last_seen = p2p_app::format_peer_datetime(peer.last_seen);
                                    if !s.peers.iter().any(|(id, _, _)| id == &peer_id) {
                                        s.peers.push_front((peer_id, first_seen, last_seen));
                                    }
                                }
                            }
                        }
                        SwarmEvent::PeerDisconnected(peer_id) => {
                            if let Ok(mut s) = state.lock() {
                                s.concurrent_peers = s.concurrent_peers.saturating_sub(1);
                                p2p_app::log_debug(&logs, format!("Peer disconnected: {} (total: {})", peer_id, s.concurrent_peers));
                            }
                        }
                        SwarmEvent::ListenAddrEstablished(addr) => {
                            p2p_app::log_debug(&logs, format!("Listening on: {}", addr));
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerDiscovered { peer_id, addresses: _ } => {
                            if let Ok(mut s) = state.lock() {
                                if !s.peers.iter().any(|(id, _, _)| id == &peer_id) {
                                    s.peers.push_front((peer_id, p2p_app::now_timestamp(), p2p_app::now_timestamp()));
                                }
                            }
                        }
                        #[cfg(feature = "mdns")]
                        SwarmEvent::PeerExpired { peer_id } => {
                            p2p_app::log_debug(&logs, format!("Peer expired: {}", peer_id));
                        }
                    }
                }
            }
        }
    });

    (handle, swarm_cmd_tx)
}
```

- [ ] **Step 2: Compile and check**

```bash
cargo build --bin p2p_chat_tui 2>&1 | grep -E "(error|warning.*unused|Finished)"
```

- [ ] **Step 3: Commit**

```bash
git add bin/tui/command_processor.rs
git commit -m "feat: implement swarm event handling in command processor"
```

---

## Phase 11: Flesh Out RenderLoop

### Task 11: Implement actual terminal rendering in RenderLoop

**Files:**
- Modify: `bin/tui/render_loop.rs` (implement full rendering logic)

Port the existing render code from the old main loop.

- [ ] **Step 1: Expand RenderLoop with full rendering logic (simplified version for MVP)**

```rust
pub fn spawn_render_loop(
    state: Arc<Mutex<crate::state::AppState>>,
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

                    // Render tabs with unread counts
                    let mut tab_titles: Vec<String> = s.dynamic_tabs.all_titles();
                    if s.unread_broadcasts > 0 {
                        tab_titles[0] = format!("{} ({})", tab_titles[0], s.unread_broadcasts);
                    }
                    for (peer_id, count) in &s.unread_dms {
                        if let Some(idx) = s.dynamic_tabs.tab_index_from_dm_id(peer_id) {
                            if idx < tab_titles.len() {
                                tab_titles[idx] = format!("{} ({})", tab_titles[idx], count);
                            }
                        }
                    }
                    let tabs = Tabs::new(tab_titles)
                        .style(Style::default().fg(Color::Cyan))
                        .select(s.active_tab);
                    f.render_widget(tabs, chunks[0]);

                    // Render peer count info
                    let peer_info = Paragraph::new(format!("Peers: {}", s.concurrent_peers));
                    f.render_widget(peer_info, chunks[1]);

                    // Render messages
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
```

This is an MVP version. Full rendering with message lists, scrolling, etc. can be added incrementally.

- [ ] **Step 2: Add required imports to render_loop.rs**

```rust
use ratatui::widgets::{Block, Borders, Paragraph};
```

- [ ] **Step 3: Compile and check**

```bash
cargo build --bin p2p_chat_tui 2>&1 | grep -E "(error|Finished)"
```

- [ ] **Step 4: Commit**

```bash
git add bin/tui/render_loop.rs
git commit -m "feat: implement basic terminal rendering in render loop"
```

---

## Phase 12: Handle Exit Signal

### Task 12: Implement graceful shutdown

**Files:**
- Modify: `bin/tui/input_handler.rs` (detect Ctrl+C)
- Modify: `bin/tui/command_processor.rs` (handle Exit event)
- Modify: `bin/p2p_chat_tui.rs` (cleanup on exit)

- [ ] **Step 1: Modify InputHandler to detect Ctrl+C**

```rust
pub fn spawn_input_handler(
    input_tx: mpsc::Sender<InputEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            if poll(Duration::from_millis(16)).ok() == Some(true) {
                if let Ok(event) = read() {
                    match event {
                        Event::Key(key) => {
                            // Detect Ctrl+C
                            if key.kind == crossterm::event::KeyEventKind::Press
                                && key.code == crossterm::event::KeyCode::Char('c')
                                && key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL)
                            {
                                // Send exit signal
                                // For now, just break the loop
                                return;
                            }
                            let _ = input_tx.send(InputEvent::Key(key)).await;
                        }
                        Event::Mouse(mouse) => {
                            let _ = input_tx.send(InputEvent::Mouse(mouse)).await;
                        }
                        _ => {}
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    })
}
```

- [ ] **Step 2: Compile and test**

```bash
cargo build --bin p2p_chat_tui 2>&1 | tail -20
```

Expected: Should compile at this point

- [ ] **Step 3: Commit**

```bash
git add bin/tui/input_handler.rs
git commit -m "feat: handle Ctrl+C for graceful shutdown"
```

---

## Summary & Verification

At this point, we have:

- [x] Defined SwarmEvent and SwarmCommand types (lib)
- [x] Created SwarmHandler task (lib)
- [x] Created CommandProcessor task (bin/tui, works with AppState)
- [x] Created InputHandler task (bin/tui)
- [x] Created RenderLoop task (bin/tui)
- [x] Created AppState (bin/tui/state.rs)
- [x] Wired tasks together in main
- [x] Implemented basic business logic
- [x] Implemented graceful shutdown

**Known Gaps (Phase 13+):**
- Full input handling (chat input, commands, tab switching)
- Complete rendering (message lists, scrolling, DM displays)
- SwarmCommand handling (send commands back to SwarmHandler)
- Error recovery for task failures
- Full copy of old logic into new tasks

These are addressed in a follow-up "Phase 13: Complete Feature Parity" task.

---

## Success Criteria

- [ ] Code compiles without errors
- [ ] All 4 tasks spawn and run
- [ ] Messages received appear in UI (via AppState)
- [ ] Ctrl+C exits cleanly
- [ ] Each task file is < 400 lines
- [ ] No `tokio::select!` in main TUI code (only in CommandProcessor where appropriate)
- [ ] SwarmHandler and types are in lib (reusable)
- [ ] Tests can be written for CommandProcessor logic separately

---

## Backlog for Future Work

- Migrate full render logic from old main into RenderLoop with proper message/DM display
- Handle all InputEvent types (character input, special keys, commands)
- Connect OutputHandler to send SwarmCommands
- Add comprehensive error handling
- Add unit tests for CommandProcessor with mock channels
- Port old command_handler.rs logic into CommandProcessor input handling
