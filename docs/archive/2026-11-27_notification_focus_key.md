> **ARCHIVED** — historical record, not current behaviour.

# Canonical notification focus keys (2026-11-27)

Notification delivery and TUI focus suppression used to encode the same
`broadcast` / `dm:<peer>` / `group:<id>` rule twice: once in the payload builder
and once in `AppState::active_conversation_key`. A change to either side could
silence a notification for a conversation that was not on screen, or show one
for the chat the user was already reading. This slice removes the second copy.

## What shipped

- `notifications::conversation_key_for_tab` maps a visible `TabContent` through
  the canonical message-event key function. Chat, Direct and GroupChat tabs
  produce the same strings as a broadcast, DM or group notification; Contacts,
  Groups, Log, Settings, Shares and Peer Info produce no key. Inspecting a peer
  is deliberately not the same as reading that peer's DM.
- `AppState::active_conversation_key` now delegates to that helper instead of
  formatting keys itself. The distinct `AppState::conversation_key` still uses
  `chat` for the broadcast conversation because it indexes reply drafts and
  message cursors, not notification focus.
- Malformed ids no longer create suppressible keys: a DM with an empty peer id
  and a group message with an empty group id never notify and cannot be matched
  by a tab.

## Design decisions

- The canonical function lives in `src/notifications.rs`, beside the payload
  builder, and depends only on the shared `tui_tabs::TabContent` type. The
  binary-only app state stays a thin adapter.
- Existing wire/Dart JSON is unchanged: the public helper is not in FRB's
  `rust_input`, so no generated code, web dispatch method or transport change
  was needed.
- The rule remains string-keyed at the FRB boundary because that is the
  generated payload contract. Correctness comes from having exactly one Rust
  function that produces those strings for both sides.

## Tests

- `src/notifications.rs::conversation_key_maps_message_kinds` now rejects empty
  DM and group ids, as well as control events.
- `src/notifications.rs::conversation_key_for_tab_uses_the_same_rule_as_message_events`
  compares every chat tab key with the corresponding event key and asserts no
  key for non-chat tabs, Peer Info, and malformed Direct/GroupChat tabs.
- `tests/unit/unit_bin_tui_state.rs::test_active_conversation_key_tracks_the_visible_chat`
  drives the state through Chat, Contacts, Groups, GroupChat, DM, Peer Info and
  the Shares suffix tab, and checks that notification focus (`broadcast`) stays
  distinct from the draft/cursor key (`chat`).

## Related records

- [`2026-11-21_notifications.md`](2026-11-21_notifications.md)
- [`../architecture.md`](../architecture.md)
