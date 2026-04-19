# Refactoring Summary: Decoupled Tokio Tasks for `run_tui`

## Problem
The original `run_tui` function contained a bare `tokio::select!` loop that didn't process any events. All actual event handling (Swarm events, input processing, UI rendering) was done outside the select loop, causing a mismatch between the async runtime and the event processing logic.

## Solution
Refactored the code into separate, concurrent tokio tasks with proper channel-based communication:

### 1. **RunningState** (Shared State)
- Created a `RunningState` struct to hold all application state
- Protected by `tokio::sync::Mutex` for safe concurrent access
- Contains: messages, dm_messages, peers, dynamic_tabs, UI state, nicknames, counters

### 2. **Channels for Communication**
- `ui_tx` / `ui_rx`: UI events (tick, mouse clicks, key presses)
- `input_tx` / `input_rx`: Input events from polling thread
- `swarm_tx` / `swarm_rx`: Swarm commands/events
- `render_tx` / render receiver: UI render triggers

### 3. **Separate Tasks**

#### UI Task (`run_ui_task`)
- Handles terminal setup and event loop
- Processes mouse clicks and keyboard input
- Renders UI at 16ms intervals
- Delegates event handling to helper functions

#### Swarm Task (`run_swarm_task`)
- Listens for swarm commands/events
- Processes network events asynchronously
- Decoupled from UI rendering

#### Event Handlers (moved from main loop)
- `handle_mouse_click_impl`: Mouse interaction logic
- `handle_key_press_impl`: Keyboard input processing  
- `handle_swarm_command_impl`: Swarm event processing

### 4. **Benefits**
- **Proper concurrency**: Each task runs independently
- **Clean separation of concerns**: UI, input, swarm, and rendering are separate
- **Better maintainability**: Each function has a single responsibility
- **Scalability**: Easy to add new event types or tasks
- **No busy-waiting**: Proper async/await patterns with channels

### 5. **Key Changes Made**
- Replaced bare `tokio::select!` with proper task architecture
- Created `RunningState` for shared, mutex-protected state
- Implemented channel-based communication between tasks
- Separated UI rendering from event processing
- Maintained all original functionality while improving structure

### 6. **File Modified**
`src/bin/p2p_chat_tui.rs` - Refactored `run_tui` function and added supporting structures
