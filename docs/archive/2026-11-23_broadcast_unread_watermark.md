# ARCHIVED — DB-backed broadcast unread watermark

**Closed 2026-11-23.** This document records a completed slice; it is not
maintained and does not describe current behaviour.

## Goal

Flutter's broadcast tab stored the "last viewed" count in Dart
`SharedPreferences`. That count was a length of the loaded, capped message
window, so it drifted after history caps, local tombstones and trimming, and it
was invisible to the TUI and web client. Move the watermark into Rust
`app_settings` so all three frontends share one DB-backed source and the badge
survives a restart.

## Design

- **Marker is a row id, not a list position** — `settings::broadcast_last_viewed_id`
  stores the highest viewed broadcast id under the `broadcast_last_viewed_id`
  key. `messages::broadcast_unread_count` counts non-deleted rows in `CHAT_TOPIC`
  with `is_direct = 0` and `id > marker`. A database error degrades to zero.
- **Fresh install means zero** — a missing marker (`None`) is deliberately
  reported as zero unread so first launch does not flag all of history as new.
  Writing the marker with `0` for an empty history switches to id-based counting.
- **Idempotent marking** — `messages::mark_broadcast_viewed` computes `MAX(id)`
  over the same filtered broadcast history and skips the write when the marker
  already equals that id. DMs, other topics and locally hidden rows never move
  the marker.
- **One surface per transport** — FRB exposes `getBroadcastUnreadCount()`
  (`i32`, Dart `int`) and `markBroadcastViewed()`; `src/web/methods.rs` mirrors
  both names as no-param arms and both are covered by the web no-param drift
  list. Web keeps flat JSON; the Flutter `NodeApi` contract carries the same two
  methods across FRB, HTTP/WS and the test fake.
- **Flutter** — `HomeScreen` seeds `_unreadCount` from Rust at startup and
  computes `firstUnreadIndex = (messages.length - unread).clamp(0, length - 1)`.
  It writes the marker on at-bottom scrolling, the jump button and outgoing
  sends. `shared_preferences` is gone from `pubspec.yaml`, `pubspec.lock` and the
  generated Linux plugin list.
- **TUI** — `AppState::seed_broadcast_unread` loads the badge at startup and
  opens the pane on the first unread row (`auto_scroll.chat = false`,
  `chat_scroll_offset = len - unread`). `AppState::mark_broadcast_viewed` clears
  the badge and persists best-effort; it is called on returning to the chat tab,
  scroll/wheel-to-bottom, Esc, outgoing broadcast send, and at-bottom incoming
  broadcasts. A settings write failure never blocks input.

No schema migration was needed: the existing `app_settings` table stores the
new typed key.

## Tests

- `unit_settings`: a fresh database has no marker; `0` and later ids roundtrip.
- `unit_messages`: missing marker reports zero; only newer, visible broadcast
  rows count (DMs, another topic and a tombstone are excluded); marking advances
  to the newest id, is idempotent, and writes `0` for empty history.
- Widget test: a cold-start `HomeScreen` with a seeded fake count shows the
  `2 new messages` banner, calls `getBroadcastUnreadCount`, and does not clear
  the marker just by displaying unread rows.
- Web no-param drift test names both new dispatch arms.

## Rationale

A loaded-list length cannot identify a message across windowing: the same index
can mean different rows after a cap or a local deletion. Row ids are stable, and
centralizing the count in Rust means a badge in one frontend is immediately
meaningful in the others. `cargo ct`, `cargo test --all-features`,
`flutter analyze`, `flutter test`, `flutter build web`, regenerated metrics and
`check_docs_links.py` all passed.
