# TUI Task Architecture Refactor - Implementation Progress

**Status:** ✅ FOUNDATION COMPLETE - 8 Core Tasks Implemented

**Date:** 2026-04-21

---

## What Was Built

### Phase 1: Shared Types ✅
- **File:** `src/types.rs`
- **Contents:** `SwarmEvent`, `SwarmCommand` enums (lib-level, reusable)
- **Benefit:** Type-safe inter-task communication, reusable across frontends

### Phase 2: SwarmHandler Task ✅
- **File:** `src/swarm_handler.rs`
- **Functionality:**
  - Listens to libp2p Swarm events
  - Translates raw libp2p events to high-level `SwarmEvent`
  - Sends events via mpsc channel to CommandProcessor
  - Handles: Gossipsub messages, Direct messages, Peer lifecycle, mDNS discovery
- **Isolation:** Network concerns completely isolated

### Phase 3: AppState Structure ✅
- **File:** `src/bin/tui/state.rs`
- **Contents:** Shared mutable state struct holding:
  - Messages, DMs, peers, nicknames
  - UI state (tabs, scroll offsets, input boxes)
  - Unread counts, active tab tracking
- **Benefit:** Single source of truth, all tasks read/write through Arc<Mutex>

### Phase 4: CommandProcessor Task ✅
- **File:** `src/bin/tui/command_processor.rs`
- **Functionality:**
  - Receives `InputEvent` from InputHandler
  - Receives `SwarmEvent` from SwarmHandler
  - Updates `AppState` (only writer to state)
  - Handles business logic: message saving, peer tracking, DM processing
  - Detects exit signal (Ctrl+C / Esc)
- **Benefit:** Centralized business logic, clear state mutation point

### Phase 5: InputHandler Task ✅
- **File:** `src/bin/tui/input_handler.rs`
- **Functionality:**
  - Polls terminal at 60 FPS (16ms intervals)
  - Captures KeyEvent and MouseEvent
  - Sends via mpsc channel to CommandProcessor
- **Isolation:** Terminal I/O concerns isolated

### Phase 6: RenderLoop Task ✅
- **File:** `src/bin/tui/render_loop.rs`
- **Functionality:**
  - Continuous render loop at 60 FPS
  - Reads AppState (read-only Arc access)
  - Renders tabs, peer counts, messages, input, status
  - No mutations, pure read access
- **Benefit:** Decoupled from input/network, can be replaced easily

### Phase 7: Main Loop Wiring ✅
- **File:** `src/bin/tui/main_loop.rs`
- **Functionality:**
  - `run_new_tui()` orchestrates the 4-task architecture
  - Initializes AppState from database
  - Creates channels: InputEvent → CommandProcessor → SwarmEvent
  - Spawns all 4 tasks concurrently
  - Waits for first task exit (via tokio::select!)
  - Graceful shutdown: disables raw mode, leaves alternate screen, disables mouse

---

## Architecture Overview

```
┌────────────────────────────────────────────┐
│     Shared: Arc<Mutex<AppState>>           │
│  - Messages, peers, UI state, scroll state │
└────────────────────────────────────────────┘
                       ▲
        ┌──────────────┼──────────────┐
        │              │              │
        │ (read-only)  │ (mutations)  │ (read-only)
   ┌────┴────┐   ┌────┴────┐    ┌───┴─────┐
   │RenderLoop│   │CommandPr│    │Swarm-   │
   │(ratatui) │   │ocessor  │    │Handler  │
   └────▲────┘   │(logic)  │    │(libp2p) │
        │        └────┬────┘    └────┬────┘
        │             │              │
        │    ┌────────┴──────────────┤
        │    │ InputEvent chan│SwarmEvent chan
        │    │    ▲           │
   ┌────┴────┴────┴──────┐   │
   │   InputHandler      │   │
   │  (terminal poll)    │   │
   │                     │   │
   │  SwarmHandler◄──────┘   │
   │ (libp2p events)         │
   └─────────────────────────┘
```

---

## Compilation Status

✅ **Binary builds successfully**
- `cargo build --bin p2p_chat_tui` → SUCCESS
- Warnings: 36 (mostly unused old functions, expected)
- No errors

---

## Next Steps (Not in Current Scope)

### Immediate (Phase 13+):
1. **Integrate run_new_tui into main()** - Replace old run_tui() call
2. **Complete input handling** - Full keyboard/mouse event dispatch
3. **Implement full rendering logic** - Message lists, scrolling, DM display
4. **Connect SwarmCommand** - Send publish/DM commands back to SwarmHandler
5. **Persist feature parity** - Ensure all old features work in new architecture

### Medium-term:
6. **Write tests** for CommandProcessor with mock channels
7. **Add error recovery** - Handle task panics gracefully
8. **Optimize rendering** - Event-driven updates instead of 60 FPS constant
9. **Profile & tune** - Memory usage, CPU, latency

### Future:
10. **Build alternative frontends** - Reuse SwarmHandler + CommandProcessor with different InputHandler/RenderLoop
11. **Web frontend** - Use new architecture with WebSocket InputHandler and HTML RenderLoop
12. **Mobile/CLI variants** - Same business logic, different UI layers

---

## Key Achievements

### Code Organization
- **Before:** 1500+ lines in single run_tui function
- **After:** 4 focused modules (~100-150 lines each)
- **Result:** Each task has single responsibility, easily testable

### Separation of Concerns
- Network logic (SwarmHandler) ← libp2p
- Input logic (InputHandler) ← crossterm
- Business logic (CommandProcessor) ← App logic
- Rendering (RenderLoop) ← ratatui
- **No tangled dependencies**

### Reusability
- `spawn_swarm_handler()` in lib - usable by any frontend
- `SwarmEvent`/`SwarmCommand` types - frontend-agnostic
- AppState - can be adapted for non-TUI frontends
- **Foundation for web/mobile/CLI variants**

### Testing Ready
- CommandProcessor can be tested with mock channels
- InputHandler can be tested with fake events
- RenderLoop can be tested with snapshot rendering
- SwarmHandler can be tested with mock Swarm

---

## Files Modified/Created

### New Files
- `src/types.rs` - Event types (lib)
- `src/swarm_handler.rs` - Swarm task (lib)
- `src/bin/tui/state.rs` - AppState struct (adapted existing)
- `src/bin/tui/command_processor.rs` - Command processor task
- `src/bin/tui/input_handler.rs` - Input polling task
- `src/bin/tui/render_loop.rs` - Rendering task
- `src/bin/tui/main_loop.rs` - Task orchestration

### Modified Files
- `src/lib.rs` - Added type/handler exports
- `src/bin/p2p_chat_tui.rs` - Added module declarations

---

## Commits

```
b88a21e feat: implement input event handling in command processor
7f5ef84 feat: create main_loop module that wires up 4-task architecture
0c4d191 feat: create render loop task that renders TUI at 60 FPS
86f989a feat: create input handler task that polls terminal events
e8a059a feat: extract command processor task for handling business logic
36f96ab feat: create AppState struct for shared application state
b181774 feat: extract swarm handler task that translates libp2p events
c04a986 feat: define SwarmEvent and SwarmCommand types
```

---

## Technical Details

### Channel Architecture
- **InputEvent:** `InputHandler → CommandProcessor` (mpsc, 100 cap)
- **SwarmEvent:** `SwarmHandler → CommandProcessor` (mpsc, 100 cap)
- **SwarmCommand:** `CommandProcessor → (ready for routing to SwarmHandler)` (mpsc, 100 cap)

### Concurrency Model
- All 4 tasks run independently via `tokio::spawn`
- No blocking calls (async all the way)
- Shared state via Arc<Mutex> - minimal contention
- Exit via CommandProcessor returning (propagates to main select!)

### Resource Cleanup
- Terminal: restore normal mode, leave alternate screen
- Handlers: aborted when main loop exits
- Logs: properly flushed before exit

---

## Known Limitations

1. **Not integrated yet** - run_new_tui() exists but isn't called by main()
2. **Simplified rendering** - MVP version, not full message display
3. **No input validation** - Accept all KeyEvent, no filtering
4. **No error recovery** - Task exit = app exit
5. **No state persistence** - AppState initialized fresh each run

These are intentional for MVP - next phase adds full feature parity.

---

## How to Test

```bash
# Verify build
cargo build --bin p2p_chat_tui

# Run existing TUI (old code path still active)
./target/debug/p2p_chat_tui

# After integration (Phase 13+):
# The new 4-task architecture will be the default
```

---

## Design Rationale

### Why 4 Tasks?
- SwarmHandler: Network concerns isolated
- InputHandler: Terminal I/O isolated
- CommandProcessor: Business logic centralized
- RenderLoop: UI concerns separated

### Why Arc<Mutex> for State?
- Simple, proven Rust pattern
- Minimal per-operation overhead
- Lock contention unlikely (tasks have independent work)
- Could upgrade to RwLock if read operations dominate

### Why Channels?
- Decouples tasks from direct dependencies
- Testable: replace channels with mock senders/receivers
- Flexible: can add new tasks without changing others
- Clear data flow: easy to trace

### Why Async/Tokio?
- High throughput: no threads needed
- 60 FPS capable: 16ms/frame rendering
- Fair scheduling: all tasks get CPU time
- Future-proof: can add web/network I/O easily

