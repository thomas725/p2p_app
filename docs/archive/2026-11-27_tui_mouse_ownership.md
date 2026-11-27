> **ARCHIVED** — historical record, not current behaviour.

# TUI mouse ownership (2026-11-27)

Wheel routing had just landed on every tab, but mouse input still reached tabs
and rows underneath the keyboard's modal surfaces. A click on a tab title could
move focus while a confirmation prompt was visible, a wheel could scroll a
conversation behind a live call, and an ordinary validation popup could be
bypassed while a modal editor was open.

## What shipped

- `AppState::has_prompt_or_call_overlay()` is the single predicate for surfaces
  that own the screen: the device-binding, group-leave and delete-message
  prompts plus an active `CallSession`. `process_mouse_event` returns immediately
  while any of them is present, so neither a click nor a wheel reaches the tab
  bar, rows or transcript beneath it.
- A plain popup remains click-to-dismiss even while a modal editor is active.
  The click consumes only the popup; after dismissal `InputMode::is_modal_editor()`
  keeps the pointer with the editor just as it keeps the keyboard. A wheel never
  dismisses the popup.
- Contacts search stays outside the modal-editor set. Its wheel still moves the
  roster selection, and a tab click still finalizes the inline box through the
  existing `switch_to_tab` path while preserving the applied filter.

## Real-dispatch coverage

Seven sessions drive `process_input_event(InputEvent::Mouse(...))` through
`KeySession::press_mouse`. Each overlay case starts from a state whose unguarded
wheel or tab click would visibly change selection/focus, then asserts all three
surfaces agree:

- the rendered frame still shows the owning prompt or call overlay;
- in-memory tab, selection, prompt/call and editor state are unchanged;
- the command channel stays empty, so no hidden navigation or action was sent.

Two more sessions cover popup dismissal during a dial editor and the
non-modal Contacts search path. No production visibility was widened for tests.

## Decisions and deferred work

The pointer order deliberately matches the keyboard order: confirmation prompts
and calls first, ordinary popups next, modal editors next, inline search last.
No FRB/web surface or wire type changed, so no Flutter regeneration was needed.
TUI share creation/binding/snapshots and grant editing remain deferred (see
[`../backlog.md`](../backlog.md)).

## Related records

- [`2026-11-27_tui_focus_and_wheel_routing.md`](2026-11-27_tui_focus_and_wheel_routing.md)
- [`2026-11-27_tui_modal_editor_boundary.md`](2026-11-27_tui_modal_editor_boundary.md)
- [`2026-11-27_tui_key_driven_sessions.md`](2026-11-27_tui_key_driven_sessions.md)
