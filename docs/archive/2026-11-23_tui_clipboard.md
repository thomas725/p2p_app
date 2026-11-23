# ARCHIVED — TUI clipboard parity (OSC 52)

**Closed 2026-11-23.** This document records a completed slice; it is not
maintained and does not describe current behaviour.

## Goal

Flutter can copy selected messages to the clipboard (`copySelectedMessages` in
`lib/src/screens/messages.dart`), but the ratatui TUI had no clipboard action at
all. Close that gap with the smallest self-contained change: no protocol,
database or FRB/codegen surface touched.

## Design

- **Pure sequence builder** — `src/clipboard.rs` (gated behind the `tui`
  feature) exposes `copy_sequence(&str) -> String`. It delegates base64 encoding
  and the OSC 52 framing to crossterm's `CopyToClipboard::to_clipboard_from`,
  targeting the `c` (clipboard) selection. Because the payload is base64-encoded
  by crossterm, message text containing newlines, tabs or ANSI bytes cannot
  inject a second OSC command. crossterm emits the canonical `ESC \` string
  terminator.
- **Single-threaded stdout** — the render loop already owns every TUI stdout
  write (that is how the OSC 777 desktop notifications in
  `src/notifications.rs` work). `AppState.pending_clipboard: Vec<String>` queues
  payloads; after `terminal.draw`, the render loop drains both that queue and
  `pending_notifications` through one locked stdout handle. A copy sequence can
  never interleave with a ratatui frame.
- **`Ctrl+C` binding** — added to `handle_chat_action_shortcut` in
  `src/bin/tui/input_processor.rs`, which only runs on Chat / DM / Group chat
  tabs while the composer is in `InputMode::Chat`. Raw mode delivers `Ctrl+C` as
  a key event, not SIGINT. It copies `AppState::newest_message_line(tab)` — the
  exact rendered transcript line (time, sender tag, body, including continuation
  lines) that `Ctrl+R`/`Ctrl+Y` already target. On an empty conversation the key
  is a no-op and never types `c` into the composer.

## Why "newest message"

The TUI's other message actions (`Ctrl+R` reply, `Ctrl+Y` react, `Ctrl+E` edit,
`Delete`, `Ctrl+O` open) all operate on the newest row rather than an arbitrary
selection; closing that wider gap needs a selection model and is deliberately
out of scope. `Ctrl+C` follows the established model. Flutter's per-message
chips can still target any message — that parity difference remains documented
in `KEYBOARD_SHORTCUTS.md`.

## Dependency

`Cargo.toml` enables crossterm's `osc52` feature. It only pulls base64 0.22,
already present in the lockfile via the `web` feature, so there was no lockfile
churn.

## Tests

- `src/clipboard.rs` unit tests: clipboard destination + known base64, no raw
  control-character / OSC injection, empty payload, distinct payloads.
- `tests/unit/unit_bin_tui_state.rs`: `newest_message_line` returns the last row
  for Chat/DM/Group, keeps multiline bodies, and yields nothing for non-chat
  tabs or missing conversations.
- `tests/unit/unit_bin_tui_input_processor.rs`: `Ctrl+C` queues the newest line
  on a chat tab without typing into the composer, is a no-op when empty, and is
  inert on non-chat tabs.
- `tests/tui_render_integration.rs`: the copy hint appears on chat shortcut bars
  and in the `?` help, and stays off other tabs.

## Terminal caveat

OSC 52 support is terminal-dependent (kitty, iTerm2, WezTerm, Alacritty, foot
and modern xterm work; tmux needs clipboard passthrough / `set-clipboard`
enabled). The same caveat already applies to the OSC 777 notifications, so no
new limitation is introduced.
