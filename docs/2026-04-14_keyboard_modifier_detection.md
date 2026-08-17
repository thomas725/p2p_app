# Keyboard Modifier Detection Issue

## Problem

Detecting modifier keys (Shift, Ctrl, Alt) combined with Enter key in the TUI is unreliable. 

### Observed Behavior

| Key Combination | Expected | Actual |
|----------------|----------|--------|
| Enter | Send message | Works |
| Alt+Enter | Insert newline | Works |
| Ctrl+Enter | Insert newline | Sends message (not detected) |
| Shift+Enter | Insert newline | Does nothing (not detected) |

## Technical Details

The application uses `crossterm` for terminal input. The keyboard enhancement flags are set in the TUI initialization:

```rust
PushKeyboardEnhancementFlags(
    crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
        | crossterm::event::KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
)
```

### Current Code

```rust
if key.code == KeyCode::Enter && key.modifiers.contains(KeyModifiers::ALT) {
    // Insert newline
} else if key.code == KeyCode::Enter {
    // Send message
}
```

## Possible Causes

1. **Terminal handling**: Some terminals don't properly report modifier keys with Enter
2. **Crossterm version**: Different versions handle modifiers differently  
3. **Raw mode**: Terminal in raw mode may strip modifier info
4. **Keyboard enhancement**: Need additional enhancement flags

## Current Workaround

**Alt+Enter** works for inserting new lines. This is the documented approach.

## Attempts Made

1. Added `KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS` - did not help
2. Checking for `SHIFT` modifier - not detected by crossterm
3. Checking for `CONTROL` modifier - detected as plain Enter
4. Using `key.modifiers.is_empty()` - causes other issues with input handling

## Mouse Handling

From the original TUI implementation:
> Disable mouse capture to allow text selection/copying

Mouse is currently disabled in the TUI. If needed, it could be toggled with a function key (e.g., F12).

## Debug Logging

Debug logging can be enabled to see what modifiers are actually detected:

```rust
if key.code == KeyCode::Enter {
    log_debug(&logs, format!("Enter key, modifiers: {:?}", key.modifiers));
}
```

## Recommendations

1. **Stay with Alt+Enter**: This works reliably across terminals
2. **Document clearly**: Show shortcuts in the UI help text
3. **Add F12 mouse toggle**: Could re-enable mouse with a toggle key if needed
4. **Accept limitation**: Some terminals have poor modifier key support