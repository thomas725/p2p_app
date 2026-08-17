# TUI Layout and Mouse Click Handling

## Row Structure (0-indexed)

The terminal is organized into rows starting from 0:

| Row | Content |
|-----|---------|
| 0 | Tab controls (tabs header with borders) |
| 1 | Notification bar OR empty row |
| 2 | Box border + header (varies by tab) |
| 3 | First peer / first message content |

### Row Breakdown by Tab

**Peers Tab:**
- Row 0: Tab controls
- Row 1: Notification (if present) or empty
- Row 2: Box border + "Peers" header
- Row 3+: First peer, second peer, etc.

**Chat Tab:**
- Row 0: Tab controls
- Row 1: Notification (if present) or empty
- Row 2: Box border + list header row 1 (e.g., "Messages [1/100]")
- Row 3: List header row 2 (column titles like "From", "Message")
- Row 4+: First message, second message, etc.

## Formula for Content Start Row

### Peers Tab

```rust
let peers_start_row = 3;
if row >= peers_start_row {
    let p = row - peers_start_row;
    // p = 0 is first peer, p = 1 is second peer, etc.
}
```

**Click row 3** → first peer (p = 0)
**Click row 4** → second peer (p = 1)

### Chat Tab

```rust
let content_start_row = 4;
if row >= content_start_row {
    let clicked_row = row - content_start_row;
    // Iterate through messages accumulating line counts
}
```

**Click row 4** → first message (relative row 0)
**Click row 5** → second message (relative row 1), assuming single-line messages

### Notification Handling

The notification bar occupies row 1 but does NOT shift the peers list down.
Both Peers and Chat use fixed base offsets regardless of notification state.

## Common Mistakes

1. **Off-by-one errors**: Always test with the actual TUI. Unit tests may not catch row calculation bugs.

2. **Confusing row index vs 1-indexed**: Crossterm uses 0-indexed rows. Row 0 is the top row.

3. **Not accounting for multi-line messages**: When clicking on messages, the handler must account for messages that wrap to multiple lines based on terminal width.

4. **Wrong peer index**: Clicking row X should give peer index (X - start_row), not just X.

## Click Handler Pattern (Peers)

```rust
let peers_start_row = 3;  // Hardcoded after empirical testing
if row >= peers_start_row {
    let p = row - peers_start_row;
    if p < peers.len() {
        // Open DM with peers[p]
    }
}
```

## Click Handler Pattern (Chat)

```rust
let content_start_row = 4;  // First message row
let content_width = terminal_width - 4;  // Account for borders

if row >= content_start_row {
    let clicked_row = row - content_start_row;
    
    let mut current_row = 0;
    for msg_idx in chat_scroll_offset..messages.len() {
        let msg = &messages[msg_idx];
        let manual_breaks = msg.matches('\n').count();
        let wrapped_lines = (msg.len() / content_width).max(1);
        let msg_lines = manual_breaks + wrapped_lines;
        
        if clicked_row >= current_row && clicked_row < current_row + msg_lines {
            // Clicked on this message
            break;
        }
        current_row += msg_lines;
    }
}
```

## Testing with TuiTestState

The `TuiTestState` struct in `src/lib.rs` provides testable methods:

```rust
pub fn list_header_start_row(&self) -> u16 {
    let tabs_rows = 3;
    let notification_rows = if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() { 1 } else { 0 };
    tabs_rows + notification_rows
}

pub fn first_message_row(&self) -> u16 {
    self.list_header_start_row() + 2  // +2 for Chat's 2-row list header
}

pub fn handle_mouse_click(&self, row: u16, _col: u16) -> Option<String> {
    // Returns peer ID for clicked message, or None if outside content area
}
```

Run tests:
```bash
cargo test --test tui_chat
```
