> **ARCHIVED** — historical record, not current behaviour.

# Settings peer rows open Peer Info (2026-11-26)

Flutter's Settings tab already let a user tap a connected-peer or bound-device
row to open that peer's PeerInfo page. The TUI printed the same rows but treated
them as plain text, and `Ctrl+P` was a no-op on Settings. This slice closes that
frontend gap without changing the DB, wire or FRB surfaces.

## What shipped

- `tui_render::settings::SettingsLine` is now the one visual-row model used by
  both `render_settings_content` and the TUI mouse handler. A row carries the
  exact rendered text plus an optional peer target, so informational rows stay
  inert and peer rows open the same deduplicated PeerInfo tab.
- Connected peers and bound devices are sorted by peer ID before rendering. The
  `› ` affordance survives wrapping onto the first visual line, every wrapped
  continuation line keeps the peer target, and hit-testing uses the same
  `settings_visual_lines(state, width)` output as the renderer (width =
  terminal width minus the block border).
- `Ctrl+P` on Settings resolves through `AppState::peer_info_target`: first
  connected peer by ID, otherwise first bound device by ID. With neither, the
  chord is consumed as a no-op and never types a literal `p`.
- The "Last connected …" row shown when nobody is connected stays informational,
  matching Flutter's last-connection label rather than presenting a stale peer
  as clickable.
- The shortcut bar and `?` help now advertise Settings `Ctrl+P` and the mouse
  action; `KEYBOARD_SHORTCUTS.md` documents the ordering and the informational
  row.

## Tests

- `tests/tui_render_integration.rs` covers peer-ID ordering, wrapped peer rows
  keeping their target, and informational rows having none. Settings tests use
  an isolated temporary database and `shared_db_test_lock`.
- `tests/unit/unit_bin_tui_click_handlers.rs` clicks connected, bound-device and
  wrapped continuation rows and verifies an informational row click is inert.
- `tests/unit/unit_bin_tui_input_processor.rs` verifies `Ctrl+P` prefers the
  connected peer, falls back to a bound device, and is a no-op with no targets.
- Shortcut-hint assertions were updated so Settings is no longer treated as a
  tab without a PeerInfo target.

No Dart/Kotlin or generated bridge code changed.
