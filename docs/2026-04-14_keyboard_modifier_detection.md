# Keyboard and Input Handling

## Current Key Bindings

| Key Combination | Action |
|----------------|--------|
| Enter | Send message |
| Shift+Enter | Insert newline (multi-line message) |
| Ctrl+Q | Exit application |
| F12 | Toggle mouse capture on/off |
| Arrow keys / Page Up/Down | Navigate messages/peers |

## Implementation Details

The TUI uses `crossterm` for terminal input with keyboard enhancement flags:

```rust
PushKeyboardEnhancementFlags(
    crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | crossterm::event::KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
)
```

### Enter Key Handling

The current implementation in `src/bin/tui/input_processor.rs`:

```rust
/// Handles Enter key (send message or multi-line input)
async fn handle_enter_key(
    state: &mut AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
    shift_held: bool,  // Shift+Enter detected
    tab_content: TabContent,
) {
    if shift_held {
        if tab_content.is_input_enabled() {
            state.chat_input.insert_str("\n");  // Insert newline
        }
    } else if /* ... */ {
        // Send message on Enter (without Shift)
    }
}
```

**Key points:**
- Plain `Enter` sends the message
- `Shift+Enter` inserts a newline for multi-line messages
- This works reliably across terminal emulators

### Mouse Handling

Mouse capture is **enabled by default** and can be toggled with **F12**:

```rust
/// Toggles mouse capture mode (F12)
fn toggle_mouse_capture(state: &mut AppState) {
    // Enable/disable mouse capture
    state.mouse_capture = !state.mouse_capture;
}
```

Mouse enables:
- Click-to-select peers from the peer list
- Click-to-navigate between tabs
- Scroll wheel for message history navigation
- Drag to select text for copying

## Related Files

- `src/bin/tui/input_processor.rs` — keyboard event processing
- `src/bin/tui/click_handlers.rs` — mouse click handling (100% coverage)
- `src/bin/tui/scroll_handlers.rs` — scroll wheel handling (98.2% coverage)
- `src/bin/tui/main_loop.rs` — terminal setup (`EnableMouseCapture` on line 126)