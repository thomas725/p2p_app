> **ARCHIVED** — historical record, not current behaviour.

# TUI message selection and row-targeted verbs (2026-11-27)

The TUI's reply, reaction, copy, open, edit and delete shortcuts always picked
the newest matching row. That was predictable but made an older message
unreachable without Flutter's per-message menus. This slice adds an explicit
keyboard cursor while preserving the old behaviour as a fallback, and changes
no wire, database or FRB/web shape.

## What shipped

- `Ctrl+Up` / `Ctrl+Down` move a message cursor on the Chat, DM/Direct and
  Group-chat tabs. It is clamped at both ends, and moving it scrolls the row
  into view with the same line-count window math the scroll handlers use;
  reaching the tail re-engages auto-scroll and clears that conversation's
  unread marker. Bare arrows and the wheel still only scroll — they never arm
  a selection.
- The cursor is stored by stable identity, not by index: a wire id, otherwise
  the local DB id, otherwise the rendered line plus its ordinal among equal
  lines and the loaded transcript length. A trim or local deletion that
  changes the length makes a rendered-only anchor stale, so verbs fall back to
  the newest row instead of retargeting a different equal line that inherited
  the old index. Anchors are scoped per conversation (`chat`, `dm:<peer>`,
  `group:<id>`).
- `Ctrl+R` (reply), `Ctrl+Y` (react), `Ctrl+C` (copy), `Ctrl+O` (open
  attachment), `Delete` (local delete) and `Ctrl+E` (local edit) act on the
  selected row (newest if none selected). With a cursor armed, only that row
  qualifies: reply, reaction and open never substitute a different message,
  and edit/delete never act on a different sent row. Without a cursor,
  `Ctrl+O` scans backwards for the newest row that actually carries a
  downloaded attachment rather than assuming the newest row has one.
- Chat, the DM tab's bottom pane and group transcripts highlight the selected
  row through ratatui's real `ListState`. The rendered selection is projected
  from `AppState` into `TuiRenderState` by `app_state_to_render_state`, so the
  live frame and the library renderer agree on the highlight.
- On a DM tab the cursor targets the bottom DM transcript only. Bare arrow and
  wheel scrolling remains hover-aware across the top broadcast pane and the
  bottom DM pane; the top pane cannot hold the cursor.
- `Ctrl+P` is deliberately outside this slice: it stays newest/sender-targeted
  (and partner-targeted on DMs) and never follows the cursor.
- `Esc` unwinds exactly one layer: popup/edit/reaction/Contacts search/nickname/
  group prompt/dial/network-peer prompt → the active conversation's reply draft
  → that conversation's message selection → return to Chat. Drafts and cursors
  are per conversation (`chat`, `dm:<peer>`, `group:<id>`). Device-binding,
  group-leave and delete-message prompts are handled upstream by the established
  prompt/call precedence chain.
- User-facing text uses "selected row (newest if none selected)" and states the
  DM limitation. The live references are [`../../KEYBOARD_SHORTCUTS.md`](../../KEYBOARD_SHORTCUTS.md)
  and [`../files_attachments.md`](../files_attachments.md).

## Session regressions to be added

The caller is adding key-driven sessions alongside the code; they drive
`process_input_event` and inspect rendered frames rather than only calling
state helpers:

- `Ctrl+Up` / `Ctrl+Down` on Chat, DM and group tabs move the highlighted row
  visible in the frame, clamp at both ends and coexist with bare arrows/wheel
  as scroll-only input;
- a DM session pins the cursor to the bottom transcript while the top broadcast
  pane keeps hover-based scrolling;
- reply, reaction, copy, open, edit and delete sessions assert that the verb
  targets the cursor's row when it differs from the newest row, and that
  clearing the cursor restores newest-row behaviour;
- an `Esc` session clears the active conversation's reply draft first and that
  conversation's selection on the next press, with the single-slot prompts and
  live call still owning the first press;
- stale-anchor sessions trim or delete the selected row and assert the fallback
  never retargets a different equal line or a reused index;
- frame assertions read the palette's selection background from the rendered
  buffer, so the state projection and `ListState` highlight cannot drift apart.

## Decisions and deferred work

Keeping the newest-row fallback means existing muscle memory still works: a
verb without an armed cursor behaves exactly as it did before this slice. The
id-less rendered anchor is deliberately conservative — changing the loaded
length invalidates it rather than guessing which equal line the operator meant.
No FRB binding, web method or JSON shape was added, so no Flutter regeneration
was needed and this is not a Flutter/web protocol change.

Earlier archived records such as
[`2026-11-23_tui_clipboard.md`](2026-11-23_tui_clipboard.md),
[`2026-11-20_local_message_edits.md`](2026-11-20_local_message_edits.md),
[`2026-11-17_local_message_deletion.md`](2026-11-17_local_message_deletion.md)
and [`2026-11-21_message_reactions.md`](2026-11-21_message_reactions.md)
describe these verbs as newest-only; that wording was accurate when written and
is now superseded. [`2026-11-27_tui_modal_editor_boundary.md`](2026-11-27_tui_modal_editor_boundary.md)
explicitly deferred a visible selected-message cursor — that deferral is closed
here. Dated records are not maintained later, so this new record is the place
where the supersession is written down.

Commit: to be linked by the caller when this slice lands. No hash is invented
in this historical record.

## Related records

- [`2026-11-27_tui_focus_and_wheel_routing.md`](2026-11-27_tui_focus_and_wheel_routing.md)
- [`2026-11-27_tui_modal_editor_boundary.md`](2026-11-27_tui_modal_editor_boundary.md)
- [`2026-11-27_tui_mouse_ownership.md`](2026-11-27_tui_mouse_ownership.md)
- [`2026-11-26_tui_ctrl_p_peer_info_parity.md`](2026-11-26_tui_ctrl_p_peer_info_parity.md)
- [`2026-11-23_tui_clipboard.md`](2026-11-23_tui_clipboard.md)
