# ARCHIVED — TUI Log "Copy All" (OSC 52)

**Closed 2026-11-23.** This document records a completed slice; it is not
maintained and does not describe current behaviour.

## Goal

Flutter's Log tab exposes an `Icons.copy` "Copy All" app-bar action that writes
`getLogs().join('\n')` to the clipboard (`apps/flutter_app/lib/src/screens/log_tab.dart`).
The ratatui TUI Log tab only had `c` to clear the buffer. Close the parity gap
with no protocol, database or FRB surface touched.

## Design

- **Pure payload helper** — `src/clipboard.rs` gains `copy_all_logs(&[String]) ->
  Option<String>`, joining with `\n` exactly as Flutter does and returning `None`
  for an empty buffer (no pointless OSC sequence). Keeping the join rule pure and
  separate from the OSC 52 builder means its edge cases are directly testable.
- **One clipboard owner** — the `Ctrl+C` arm moved out of
  `handle_chat_action_shortcut` (which is gated to chat tabs with
  `InputMode::Chat`) into a new `handle_clipboard_shortcut` that runs immediately
  after global navigation in `handle_key_dispatch`. It handles both cases:
  - Chat / DM / Group chat → `newest_message_line(tab)` (the same row `Ctrl+R`
    and `Ctrl+Y` target), only while composing normally.
  - Log → `copy_all_logs(&get_tui_logs())`, the whole in-memory buffer.
  
  Returning `true` consumes the key in both cases, so `c` never reaches the
  composer or the Log tab's bare-`c` clear arm. Every other tab leaves `Ctrl+C`
  alone, which keeps `c` available to its own shortcuts.
- **No conflict with log clearing** — the dispatch arms for Settings, Groups and
  Log are all gated on `!is_ctrl`, so `Ctrl+C` never reached the bare-`c` "clear
  logs" action even before this change. The two actions are now deliberately
  distinct: `Ctrl+C` copies, `c` clears.

## Tests

- `src/clipboard.rs`: empty buffer → `None`, single line verbatim, multiple
  lines (including blank lines) joined with newlines.
- `tests/unit/unit_bin_tui_input_processor.rs`: `Ctrl+C` on Log captures the
  whole buffer (serialised around the global log buffer), an empty buffer queues
  nothing, and `Ctrl+C` on Settings remains inert. The previous "inert on
  non-chat tabs" test was corrected — it had pointed at index 4 (Settings) while
  claiming to be the Log tab; Log is index 3 with no dynamic tabs.
- `tests/tui_render_integration.rs`: the Log shortcut bar advertises
  `Ctrl+C: copy all`, the `?` help names the action, and other tabs do not claim
  the hint.

## Documentation

`KEYBOARD_SHORTCUTS.md` now lists the Log binding in the global action table,
the Clipboard section and the Log-tab table. Metrics regenerated; docs links
verified.
