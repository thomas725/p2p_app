# ARCHIVED — TUI DM/group unread banners and scroll fidelity

**Closed 2026-11-23.** This document records a completed slice; it is not
maintained and does not describe current behaviour.

## Goal

Flutter's DM and group chat screens track `MessageChatMixin.atBottom` /
`unreadCount` and render a "N new" jump button while the reader is scrolled up
(`apps/flutter_app/lib/src/screens/messages.dart:26-39,503-518`). The ratatui
TUI tracked unread only for the broadcast chat, and incoming group messages
unconditionally reset that group's scroll state to `(len, true)`, yanking a
reader who was reading history back to the tail. Close both gaps in one batch.

## Design

- **Per-conversation unread maps** — `AppState::dm_unread_count` and
  `group_unread_count` (`HashMap<String, usize>`) join the existing per-pane
  `dm_scroll_state` / `group_scroll_state`. `TuiRenderState` mirrors them as
  `BTreeMap`s via the existing `cloned_log` adapter in `app_state_to_render_state`.
  Missing entries mean "nothing unread", so no extra `Option` state is needed.
- **One "scrolled up" rule** — `command_processor::is_scrolled_up` treats a
  missing scroll entry as pinned to the tail and otherwise reads the stored
  `auto_scroll` flag. Both `apply_dm_to_state` and
  `apply_group_message_to_state` increment unread through it, exactly as the
  broadcast path does with `auto_scroll.chat`.
- **No forced group scroll** — the incoming-group handler used to insert
  `(len, true)` on every message. Rendering already follows the tail whenever
  `auto_scroll` is true (`calc_visible_impl` ignores the stored offset), so the
  write was redundant for followers and actively harmful for readers. It is
  gone; outgoing group sends still jump to the tail and clear that group's
  unread, since our own message has obviously been seen.
- **One clear rule** — `scroll_handlers::clear_unread_on_bottom` resets the map
  entry when scrolling flips a pane from manual back to auto-scroll. It is
  wired into the DM and group keyboard scroll functions and both mouse-wheel
  paths, mirroring the broadcast `chat_unread_count` clearing. Leaving a group
  drops its unread entry alongside its history and scroll state.
- **One banner renderer** — `render_unread_banner` in `src/tui_render/mod.rs`
  owns the label ("▼ N new message(s) — End: jump to latest") and placement,
  and is called by the broadcast, DM and group panes. The broadcast-specific
  copy in `chat.rs` was removed; the same wording now applies everywhere.

No protocol, database, FRB or web surface changed.

## Tests

- `unit_bin_tui_command_processor`: DM unread increments only while that peer
  is scrolled up; group history keeps the reader's exact `(offset, false)`
  while counting one unread; a follower accumulates neither a scroll entry nor
  unread; a closed group ignores the event entirely.
- `unit_bin_tui_scroll_handlers`: `End` and wheel-down clear DM/group unread
  once auto-scroll re-engages, while `Up` preserves it. The wheel-down group
  test uses a one-line viewport so a real scroll occurs — with a tall viewport
  all messages fit and scrolling is correctly a no-op.
- `tui_render_integration`: the DM pane renders only its own peer's count, the
  group pane renders its count, and neither shows a banner at zero.

## Rationale

The TUI keeps scroll state per conversation across tab switches, while Flutter
keeps it per open screen. Counting unread against the conversation's own scroll
state (rather than the active tab) is the closest faithful translation: a
message that arrives for a scrolled-up DM is unread even if the user is on the
Chat tab, and the banner is waiting when they return. Metrics regenerated;
docs links verified.
