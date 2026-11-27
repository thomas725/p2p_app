> **ARCHIVED** — historical record, not current behaviour.

# TUI focus finalization and wheel routing (2026-11-27)

The TUI shares one text editor between the chat composer, prompt modes and the
Contacts filter. Because Contacts search is inline rather than modal, keyboard
and mouse focus changes could leave the query in the shared box after the user
had moved to another tab. Separately, the generic `Esc` handler duplicated the
single-slot prompt branches, and list tabs ignored the mouse wheel even though
their arrow keys already selected rows.

## What shipped

- `AppState::finalize_tab_input` / `AppState::switch_to_tab` are the single
  focus-change seam. Finalization closes the Contacts search box while keeping
  the applied query, and cancels a nickname edit. Keyboard tab cycling, tab
  clicks, close buttons, DM/Peer Info entry, message-click navigation and group
  leave all use `switch_to_tab`; production no longer assigns `active_tab`
  directly.
- `Esc` now unwinds exactly one layer. Device-binding, group-leave and delete
  prompts own it in `prompt_key_action`, a live call overlay owns it next, and
  the generic handler dismisses only the innermost popup/edit/search/prompt
  before returning. Dismissing a network-peer validation popup therefore leaves
  the rejected address open for correction.
- Mouse wheel events reach `handle_mouse_scroll` on every tab. Contacts,
  Groups, Settings and Shares move the selection with the same logic as
  `Up`/`Down`; Chat, DM, Group chat and Log still scroll their transcripts; Peer
  Info is inert and emits no redraw.

## Real-dispatch coverage

- `KeySession::press_mouse` sends `InputEvent::Mouse` through
  `process_input_event`, with no new production visibility.
- Sessions press `Tab`/`BackTab` out of a live Contacts search and assert the
  filter survives while the rendered input title and shared buffer are cleared.
  Tab clicks and Contacts row clicks get equivalent coverage.
- Both wheel directions are driven through real mouse events on all four list
  tabs, checking the in-memory selection and the rendered tab. Peer Info wheel
  coverage asserts neither state nor redraw changes.
- The existing settings session now presses `Esc` over a rejected network-peer
  address and verifies that only the validation popup closes.
- Focused `handle_mouse_scroll` tests cover list selection, Peer Info and an
  unknown wheel direction.

## Decisions and deferred work

Contacts search stays inline: Tab, arrows and the wheel keep their roster
meanings while the box is open. Keeping the filter on tab exit matches the
existing Enter behavior; clearing only the shared buffer prevents a Composer
draft from inheriting the query. No FRB/web surface or wire type changed, so no
Flutter regeneration was needed. TUI share creation/binding/snapshots and
grant editing remain deferred (see [`../backlog.md`](../backlog.md)).

## Related records

- [`2026-11-27_tui_modal_editor_boundary.md`](2026-11-27_tui_modal_editor_boundary.md)
- [`2026-11-27_tui_key_driven_sessions.md`](2026-11-27_tui_key_driven_sessions.md)
- [`2026-11-26_tui_readonly_shares_tab.md`](2026-11-26_tui_readonly_shares_tab.md)
- [`2026-10-21_contacts_search.md`](2026-10-21_contacts_search.md)
