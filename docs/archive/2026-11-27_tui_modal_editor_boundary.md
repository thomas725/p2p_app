> **ARCHIVED** — historical record, not current behaviour.

# TUI modal-editor boundary (2026-11-27)

The TUI's shared single-line input had grown seven prompt modes, but each input
gate repeated its own list of "am I editing?" checks. The lists drifted: some
prompts kept Tab/arrow navigation alive, a few tab shortcuts fired while text
was captured, and the two input renderers disagreed about which prompts were
visible.

## What shipped

- `InputMode::is_modal_editor()` (`src/types.rs`) is now the single authority for
  modal prompt ownership. It covers nickname edits (own, announced and local),
  message edits, reactions, group create/join, dialing and network-peer entry.
  `SearchingContacts` is deliberately excluded: it is an inline filter, so Tab
  still cycles tabs and arrows still navigate the roster while the query is
  edited.
- `AppState::is_modal_editor()` and `TuiRenderState::is_modal_editor()` delegate
  to that predicate. Dispatch, global navigation, tab-specific shortcuts,
  Contacts/Peer Info actions and the default text-input branch all use the
  shared guard; no caller keeps a hand-written mode list.
- The live (`src/bin/tui/render_loop/layout.rs`) and library
  (`src/tui_render/mod.rs`) input renderers now share the same visibility rule
  and both label the group-name prompt. The library renderer had silently shown
  a plain `Input` while the running TUI showed "Create or Join Group".

## Production fixes found while extending the sessions

- Enter ownership for the group-name prompt moved into `handle_enter_dispatch`.
  Previously `handle_enter_key` had a Groups-specific branch that could be
  shadowed by tab handling; now every modal commit has one owner.
- A rejected network-peer address no longer closes the prompt and discards the
  typed text. Empty submit still cancels; an invalid address keeps the prompt,
  the text and the validation popup so the next correction key edits it. This
  matches the nickname, message-edit, reaction and group-name prompts.
- Tab, `BackTab` and scroll keys are consumed (no-op) while a modal prompt owns
  the keyboard, so a prompt cannot be stranded on another tab with no visible
  input box. Ctrl+P and tab-specific action keys are inert for the same reason.

## Coverage

- `input_mode_modal_editor_boundary` locks down the included/excluded modes.
- `group_create_message_leave_session_through_real_keys` drives `g` → invalid
  name → correction → create → message → `BackTab` → `d`/`y` through
  `process_input_event`, asserting frames, DB rows, tabs and `SwarmCommand`s.
- `settings_dial_and_network_peer_session_through_real_keys` drives `d` → dial,
  then `b` → rejected address → correction → add → second add → `Up`/`x`, with
  the same cross-layer assertions.
- The library-renderer regression test asserts that the group prompt is visible
  with its text, fixing the last live/library renderer mismatch.

## Decisions and deferred work

`SearchingContacts` remains a special case by design (see
[`2026-10-21_contacts_search.md`](2026-10-21_contacts_search.md)). The existing
newest-relevant-row targeting for reply, reaction, edit, delete, copy and
attachment open is unchanged; a visible selected-message cursor is still
deferred. TUI share creation/binding/snapshots and grant editing also remain
deferred (see [`../backlog.md`](../backlog.md)).

## Related records

- [`2026-11-27_tui_key_driven_sessions.md`](2026-11-27_tui_key_driven_sessions.md)
- [`2026-11-27_group_name_validation.md`](2026-11-27_group_name_validation.md)
- [`2026-11-12_network_peer_config_surface.md`](2026-11-12_network_peer_config_surface.md)
- [`2026-10-21_contacts_search.md`](2026-10-21_contacts_search.md)
