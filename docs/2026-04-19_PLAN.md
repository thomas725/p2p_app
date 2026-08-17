# Refactoring Plan: Decoupled Tokio Tasks for `run_tui`

## Problem
The original `run_tui` function contained a large `tokio::select!` loop that mixed UI rendering, network event handling, and input processing in a single coroutine. This made the code difficult to maintain and prevented proper separation of concerns.

## Solution
Refactor the code into separate, concurrent Tokio tasks with proper channel-based communication:

### 1. RunningState (Shared State)
Create a `RunningState` struct to hold all application state, protected by a `tokio::sync::Mutex` for safe concurrent access.

### 2. Communication Channels
Define channels for inter-task communication:
- UI Task ↔ Swarm Task: For sending commands and receiving events
- UI Task ← Input Task: For receiving keyboard/mouse events
- All Tasks → Logging: For sending log messages

### 3. Separate Tasks
- **UI Task**: Handles terminal setup, rendering, and UI event processing
- **Swarm Task**: Listens for and processes network events
- **Input Task**: Polls for and processes keyboard/mouse input

## Implementation Plan

### Phase 1: Foundation
1. [ ] Define RunningState struct with all necessary state variables
2. [ ] Create channel types for inter-task communication
3. [ ] Extract existing helper functions to work with RunningState references
4. [ ] Test compilation

### Phase 2: Task Extraction
1. [ ] Extract UI task with terminal handling and rendering loop
2. [ ] Extract swarm event handling task
3. [ ] Extract input polling task
4. [ ] Implement basic task coordination
5. [ ] Test compilation

### Phase 3: Integration
1. [ ] Connect tasks via channels
2. [ ] Implement proper shutdown handling
3. [ ] Test compilation and fix errors
4. [ ] Run existing tests to ensure no regressions

### Phase 4: Verification
1. [ ] Manual testing of TUI functionality
2. [ ] Verify all original features work correctly
3. [ ] Performance testing if needed

## Benefits
- Proper concurrency with each task running independently
- Clean separation of concerns: UI, input, swarm, and rendering are separate
- Better maintainability: Each function has a single responsibility
- Scalability: Easy to add new event types or tasks
- No busy-waiting: Proper async/await patterns with channels

## Files to Modify
- `src/bin/p2p_chat_tui.rs`: Main refactoring of run_tui function
- Potential new modules in `src/tui/` for state and task definitions