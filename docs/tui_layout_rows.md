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
let tabs_rows = 3;
let notification_rows = if unread_broadcasts > 0 || !unread_dms.is_empty() { 1 } else { 0 };
let list_header_rows = 1;  // Just the "Peers" header line
let peers_start_row = tabs_rows + notification_rows + list_header_rows;
```

**Without notification:** `3 + 0 + 1 = 4` (first peer at row 3)
**With notification:** `3 + 1 + 1 = 5` (first peer at row 4)

### Chat Tab

```rust
let tabs_rows = 3;
let notification_rows = if unread_broadcasts > 0 || !unread_dms.is_empty() { 1 } else { 0 };
let list_header_rows = 2;  // Title line + column headers
let content_start_row = tabs_rows + notification_rows + list_header_rows;
```

**Without notification:** `3 + 0 + 2 = 5` (first message at row 4)
**With notification:** `3 + 1 + 2 = 6` (first message at row 5)

## Common Mistakes

1. **Hardcoding notification_rows**: Always check if there are unread broadcasts or DMs before adding 1 to the notification row count.

2. **Wrong list_header_rows**: Peers has 1 header row, Chat has 2 header rows. Don't assume they're the same.

3. **Confusing row index vs 1-indexed**: All calculations use 0-indexed rows. The first row displayed on screen is row 0, not row 1.

4. **Not accounting for multi-line messages**: When clicking on messages, the handler must account for messages that wrap to multiple lines based on terminal width.

## Click Handler Pattern

```rust
if row as usize >= content_start_row {
    let clicked_row_in_content = row as usize - content_start_row;
    // For peers: index = clicked_row_in_content
    // For messages: iterate through messages, accumulating line counts
}
```

## Testing with TuiTestState

The `TuiTestState` struct in `src/lib.rs` provides testable methods for verifying row calculations:

```rust
pub fn list_header_start_row(&self) -> u16 {
    let tabs_rows = 3;
    let notification_rows = if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() { 1 } else { 0 };
    tabs_rows + notification_rows
}

pub fn first_message_row(&self) -> u16 {
    self.list_header_start_row() + 2  // +2 for Chat's 2-row list header
}
```
