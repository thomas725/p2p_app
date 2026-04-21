# TUI Task Architecture Refactor

**Date:** 2026-04-21
**Status:** Design Phase
**Goal:** Replace monolithic `tokio::select!` with 4 independent async tasks communicating via channels and shared state, enabling better code organization and reuse across frontends.

---

## Overview

Current architecture uses a single `tokio::select!` macro waiting on multiple event sources, creating deep nesting and tight coupling. New architecture splits into:

1. **SwarmHandler** (lib) - Handles libp2p events
2. **CommandProcessor** (lib) - Applies business logic to state
3. **InputHandler** (bin) - Polls terminal events (TUI-specific)
4. **RenderLoop** (bin) - Renders to terminal (TUI-specific)

Each task is independent, runs in its own `tokio::spawn`, and communicates via channels and shared `Arc<Mutex<AppState>>`.

---

## Architecture

### Task Isolation

```
┌──────────────────────────────────────────────────────────────┐
│ Shared State: Arc<Mutex<AppState>>                           │
│ - All UI state, peer info, messages, scroll state, etc.      │
└──────────────────────────────────────────────────────────────┘
                              ▲
                              │ (read-only)
                    ┌─────────┴─────────┐
                    │                   │
          ┌─────────┴────────┐   ┌─────┴──────────┐
          │  RenderLoop      │   │ CommandProcessor
          │  (bin)           │   │ (lib)
          └──────────────────┘   └─────┬──────────┘
                                       │
                ┌──────────────────────┼──────────────────────┐
                │                      │                      │
         ┌──────┴────────┐      ┌──────┴────────┐      ┌─────┴──────────┐
         │ InputHandler  │      │ SwarmHandler  │      │ SwarmCommands  │
         │ (bin)         │      │ (lib)         │      │ (outgoing)     │
         │               │      │               │      │                │
         └───────┬───────┘      └───────┬───────┘      └────────┬───────┘
                 │                      │                       │
                 │ InputEvent           │ SwarmEvent            │ SwarmCommand
                 │ channel              │ channel               │ channel
                 └──────────────────────┼───────────────────────┘
                                        │
                              CommandProcessor
```

### Task Responsibilities

#### **SwarmHandler** (lib)
- **Input:** libp2p Swarm (mutable reference, not shared)
- **Processing:** Waits on `swarm.select_next_some()`, translates raw libp2p events to app-level `SwarmEvent`
- **Output:** `mpsc::Receiver<SwarmEvent>` for CommandProcessor
- **Dependencies:** libp2p Swarm, SwarmEvent enum, logs
- **Responsibility:** Network events only; never touches AppState directly

```rust
pub async fn spawn_swarm_handler(
    swarm: Swarm<AppBehaviour>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (JoinHandle<()>, mpsc::Receiver<SwarmEvent>)
```

#### **CommandProcessor** (lib)
- **Input:** Two channels - `InputEvent` (from InputHandler) and `SwarmEvent` (from SwarmHandler)
- **Processing:**
  - Receives events from both channels via `tokio::select!`
  - Applies business logic (e.g., "on DmReceived, add to dm_messages, mark tab unread")
  - Mutates AppState via `Arc<Mutex<AppState>>`
  - Sends SwarmCommand back to SwarmHandler when needed (publish, send DM)
- **Output:** `mpsc::Receiver<SwarmCommand>` for SwarmHandler
- **Dependencies:** AppState (Arc<Mutex>), InputEvent, SwarmEvent, SwarmCommand, logs
- **Responsibility:** Single source of truth for state mutations; orchestrates the system logic

```rust
pub async fn spawn_command_processor(
    state: Arc<Mutex<AppState>>,
    input_rx: mpsc::Receiver<InputEvent>,
    swarm_event_rx: mpsc::Receiver<SwarmEvent>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> (JoinHandle<()>, mpsc::Sender<SwarmCommand>)
```

#### **InputHandler** (bin, TUI-specific)
- **Input:** Terminal (crossterm poll/read)
- **Processing:** Polls for keyboard/mouse events, translates to `InputEvent`
- **Output:** Sends `InputEvent` to CommandProcessor channel
- **Dependencies:** crossterm, InputEvent enum
- **Responsibility:** Terminal input only

```rust
async fn spawn_input_handler(
    input_tx: mpsc::Sender<InputEvent>,
) -> JoinHandle<()>
```

#### **RenderLoop** (bin, TUI-specific)
- **Input:** AppState (read-only via Arc)
- **Processing:** On timer (every 16ms), reads AppState and renders to terminal
- **Output:** Terminal graphics
- **Dependencies:** ratatui, AppState (Arc<Mutex>), Terminal backend
- **Responsibility:** Rendering only; no business logic

```rust
async fn render_loop(
    state: Arc<Mutex<AppState>>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
) -> JoinHandle<()>
```

---

## Shared State

### AppState Structure

Located in `bin/tui/state.rs`:

```rust
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
```

**Initialization:** Before spawning tasks, populate AppState with:
- Load messages, peers, DMs from database
- Set own_nickname, topic_str
- Initialize empty UI state (tabs, inputs, etc.)

---

## Channel Types & Events

### InputEvent (bin)
```rust
pub enum InputEvent {
    Key(crossterm::event::KeyEvent),
    Mouse(crossterm::event::MouseEvent),
}
```

### SwarmEvent (lib)
High-level app events, not raw libp2p events:

```rust
pub enum SwarmEvent {
    // Incoming messages
    BroadcastMessage { content: String, peer_id: String, latency: Option<String> },
    DirectMessage { content: String, peer_id: String, latency: Option<String> },

    // Peer lifecycle
    PeerConnected(String),
    PeerDisconnected(String),

    // Network setup
    ListenAddrEstablished(String),

    // Peer discovery (if mDNS enabled)
    PeerDiscovered { peer_id: String, addresses: Vec<Multiaddr> },
    PeerExpired { peer_id: String },
}
```

### SwarmCommand (lib)
```rust
pub enum SwarmCommand {
    Publish(String),          // Broadcast a message
    SendDm(String, String),   // Send DM: (peer_id, content)
}
```

---

## Data Flow Examples

### Example 1: User Types & Sends a Broadcast Message

```
1. InputHandler detects KeyCode::Enter
2. InputHandler sends InputEvent::Key(...) → CommandProcessor
3. CommandProcessor:
   - Reads chat_input from AppState
   - If message valid, extracts content
   - Sends SwarmCommand::Publish(content) → SwarmHandler
   - Updates AppState: messages.push_back(...), chat_input.clear()
4. SwarmHandler receives SwarmCommand::Publish
   - Calls swarm.behaviour_mut().gossipsub.publish(...)
   - Message propagates to network
```

### Example 2: Network Receives a Broadcast Message

```
1. SwarmHandler receives Libp2pSwarmEvent::Behaviour(AppEv::Gossipsub(...))
2. SwarmHandler:
   - Extracts message content, peer_id, calculates latency
   - Sends SwarmEvent::BroadcastMessage(...) → CommandProcessor
3. CommandProcessor:
   - Locks AppState
   - Appends to messages
   - Increments unread_broadcasts if not on broadcast tab
   - Saves to database
   - Releases lock
4. RenderLoop (on next 16ms tick):
   - Locks AppState (read-only)
   - Renders updated messages
   - Releases lock
```

### Example 3: User Switches Tab

```
1. InputHandler detects KeyCode::Right
2. InputHandler sends InputEvent::Key(...) → CommandProcessor
3. CommandProcessor:
   - Increments active_tab
   - Clears unread_broadcasts/unread_dms for that tab
   - Updates AppState
4. RenderLoop reflects the change on next render
```

---

## Error Handling & Lifecycle

### Task Spawning
```rust
async fn main() {
    // Setup: swarm, terminal, state
    let state = Arc::new(Mutex::new(AppState { /* ... */ }));

    // Spawn tasks
    let (swarm_handler, swarm_event_rx) = spawn_swarm_handler(swarm, logs.clone());
    let (command_processor, swarm_cmd_tx) = spawn_command_processor(
        state.clone(),
        input_rx,
        swarm_event_rx,
        logs.clone()
    );
    let input_handler = spawn_input_handler(input_tx);
    let render_loop = spawn_render_loop(state.clone(), terminal);

    // Wait for any task to panic or Exit signal
    tokio::select! {
        _ = swarm_handler => { log("SwarmHandler exited"); },
        _ = command_processor => { log("CommandProcessor exited"); },
        _ = input_handler => { log("InputHandler exited"); },
        _ = render_loop => { log("RenderLoop exited"); },
    }

    // Cleanup: abort remaining tasks, restore terminal
    cleanup_tui();
}
```

### Exit Signal
- InputHandler detects Ctrl+C or user clicks Exit
- Sends InputEvent to CommandProcessor
- CommandProcessor sends Exit to RenderLoop (or returns error to main)
- All tasks abort or finish gracefully
- Main restores terminal state and exits

---

## Code Organization

### lib.rs changes
- `pub mod swarm_handler` - Exports `spawn_swarm_handler()`
- `pub mod command_processor` - Exports `spawn_command_processor()`, defines `SwarmEvent`, `SwarmCommand`
- Keep all domain logic (message saving, peer tracking, etc.) as lib functions

### bin/p2p_chat_tui.rs
- Import from lib: `swarm_handler`, `command_processor`
- Define: `InputEvent`, task functions for InputHandler and RenderLoop
- Main orchestration loop

### bin/tui/ directory structure
```
bin/tui/
  ├── state.rs           # AppState struct
  ├── input_handler.rs   # spawn_input_handler()
  ├── render_loop.rs     # spawn_render_loop()
  ├── tracing_writer.rs  # Existing
  └── command_handler.rs # (deprecated after refactor)
```

---

## Benefits

1. **Testability:** Each task logic can be tested with mock channels
2. **Reusability:** Lib provides SwarmHandler + CommandProcessor; web frontend could use same logic with different InputHandler/RenderLoop
3. **Maintainability:** ~150-200 lines per file instead of 1500+ in one file
4. **Debuggability:** Clear data flow via channels; state mutations centralized in CommandProcessor
5. **Error isolation:** Network task failure doesn't crash rendering
6. **Performance:** No deep nesting; cleaner task-local reasoning

---

## Success Criteria

- [ ] Code compiles without errors
- [ ] All 4 tasks spawn and communicate correctly
- [ ] Messages sent/received work as before
- [ ] UI renders at 60 FPS without visual artifacts
- [ ] Each .rs file is <300 lines
- [ ] AppState in lib.rs is accessed only by CommandProcessor
- [ ] Lib exports reusable `spawn_swarm_handler()` and `spawn_command_processor()` for other frontends
- [ ] Tests can verify CommandProcessor logic with mock channels

---

## Non-Goals (Out of Scope)

- State mutation logging/replay system
- Actor model with exclusive state ownership
- WebSocket/HTTP integration (future frontends can add InputHandler variants)
- Persistence layer changes

---

## Implementation Phases

1. **Phase 1:** Define channel types and event enums (lib)
2. **Phase 2:** Extract SwarmHandler from current code (lib)
3. **Phase 3:** Extract CommandProcessor from current code (lib)
4. **Phase 4:** Create AppState and split TUI tasks (bin)
5. **Phase 5:** Connect tasks via channels in main
6. **Phase 6:** Test and cleanup

---

## Known Constraints

- Terminal backend (ratatui) requires mutable state, so RenderLoop needs Arc<Mutex<>>
- Swarm requires mutable reference (no Arc possible), so it lives in SwarmHandler only
- TextArea from ratatui_textarea is not Clone, so stored directly in AppState
- Arc<Mutex<>> is sufficient; no Actor model indirection needed since CommandProcessor is single writer
