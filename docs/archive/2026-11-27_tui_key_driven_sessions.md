> **ARCHIVED** — historical record, not current behaviour.

# Key-driven TUI usage sessions (2026-11-27)

Most TUI tests called individual helpers directly. That covered units but missed
the properties that matter to a terminal user: the dispatch path, the state
lock, the command channel and the frame the renderer actually draws. This slice
adds realistic sessions that press keys through the same public entry point as
the event loop and checks that the frames, in-memory state, DB rows and queued
commands agree.

## What shipped

- `KeySession` (`tests/unit/unit_bin_tui_test_helpers.rs`) sends
  `InputEvent::Key(KeyEvent)` to `process_input_event`, drains render
  notifications after every key, exposes typed text/control helpers, collects
  queued `SwarmCommand`s, and returns newline-joined production frame rows.
  Test visibility was not widened: `process_key_event` stays private.
- `render_frame_for_test` (`src/bin/tui/render_loop/mod.rs`) is a `#[cfg(test)]`
  wrapper over the real binary renderer. Sessions no longer observe the legacy
  library renderer, which derives tabs from tab titles; production uses typed
  `DynamicTabs` lookup.
- A DM session drives reply arming (`Ctrl+R`), sending, a quick reaction,
  editing and deletion through real keys. It asserts the captured quote,
  outbound commands, persisted rows, reaction chips, tombstone behaviour and
  rendered transcript at each step.
- A Contacts session drives `/` search, opening Peer Info with `i`, and editing
  a local nickname with `n`. Assertions use the canonical
  `get_peer_display_name_or_short` text, the filtered view and the rendered
  Peer Info rows.
- A Shares session tabs to the suffix Shares tab, refreshes with `r`, then
  injects a `ShareUpdated` swarm event. It checks the share, grant and manifest
  rows, the refreshed list, the event notice, the rendered detail pane and that
  the read-only tab queues no swarm commands.

## Production fixes found by the sessions

- A validation popup used to swallow the first correction key. Now
  `popup_dismissal_feeds_editor` lets character, Backspace and Delete keys
  dismiss the hint and continue into the active editor; Enter remains
  popup-owned so an unchanged invalid submit acknowledges the hint once.
  Successful nickname, message-edit, reaction and group-create commits clear a
  stale popup.
- The live input section showed a generic `Input` title while a reply was
  armed. It now renders the canonical quoted header, matching the library
  renderer and the transcript's `Reply: ↳ sender: preview` affordance.

## Decisions and deferred work

Sessions exercise the real key path rather than calling private handlers, and
they observe production frames rather than title-parsed library frames. The
existing newest-relevant-row targeting for reply, reaction, copy, edit, delete
and attachment open remains in place; a visible selected-message cursor is
still deferred. TUI share creation/binding/snapshots and grant editing also
remain deferred (see [`../backlog.md`](../backlog.md)).

## Related records

- [`2026-11-18_reply_quote.md`](2026-11-18_reply_quote.md)
- [`2026-11-20_local_message_edits.md`](2026-11-20_local_message_edits.md)
- [`2026-11-21_message_reactions.md`](2026-11-21_message_reactions.md)
- [`2026-11-27_quick_reaction_palette_parity.md`](2026-11-27_quick_reaction_palette_parity.md)
- [`2026-11-26_tui_readonly_shares_tab.md`](2026-11-26_tui_readonly_shares_tab.md)
- [`2026-10-21_contacts_search.md`](2026-10-21_contacts_search.md)
