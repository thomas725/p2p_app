> **ARCHIVED** — historical record, not current behaviour.

# Display-parity details (2026-11-27)

Three small decorations diverged between the TUI and Flutter after reloads or
live events. Each is fixed at its canonical source; no wire type, FRB surface or
web method changed.

## What shipped

- `messages.encrypted` now drives the TUI's `[e2e] ` prefix on **every** body
  render path. `e2e::mark_encrypted` was already the live formatter, but a DB
  reload and an in-place edit re-rendered from `content` and dropped the marker.
  The DM reload formatter adds it from the persisted flag; `message_display_body`
  selects the flag too, so editing a sealed row still produces the same line as
  the next reload. The marker prefixes the body, ahead of the `(edited)` suffix
  and the reply snapshot stays outside it.
- The TUI's read-only broadcast context pane on a DM tab now carries wire ids
  alongside the visible rows and appends the shared `reaction_suffix`, matching
  the Chat/Group panes and Flutter's `MessageBubble`. The ids are looked up by
  row index, so the filtered pane never misaligns them.
- Peer Info reads stats from `TuiRenderState.contacts_all`, the unfiltered
  canonical contact list. The Contacts tab keeps its filtered `contacts` view;
  hiding a row with `/` no longer blanks that peer's `Direct messages` and
  `Broadcasts sent` rows.
- Flutter's `DmChatScreen._applyReaction` refreshes both the DM thread and its
  broadcast context pane, mirroring the TUI's single `msg_id`-keyed chip map.
  A live `reaction` event can therefore update a broadcast row displayed in a
  DM conversation without a reload.

## Coverage

- Rust unit tests assert the persisted marker after a DM DB reload and after an
  in-place edit, including the marker/suffix order.
- Render integration tests assert reaction chips in the DM broadcast pane and
  Peer Info stats while the filtered Contacts view is empty.
- A Flutter widget test delivers a real `reaction` event to an open
  `DmChatScreen` and checks the broadcast pane's chip updates.

## Decisions and deferred work

Flutter Settings' "last connection lost" fallback still prints a raw peer id
when the peer has left the roster. Fixing that needs a canonical Rust-backed
display-name binding (FRB regeneration plus a matching web method and dispatch
test), so it stays deferred rather than duplicating petname logic in Dart. See
[`../backlog.md`](../backlog.md).
