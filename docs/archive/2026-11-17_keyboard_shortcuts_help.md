# Keyboard shortcut help overlay (2026-11-17)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

US-029 closed the gap between the tab-aware shortcut bar and
[`KEYBOARD_SHORTCUTS.md`](../../KEYBOARD_SHORTCUTS.md): the TUI can now show a
compact modal reference with `?`, and the Flutter/Web app gained an honest
**Help / controls** dialog in Settings.

## TUI

- `src/tui_render::shortcuts_help_text()` is the single, pure payload. It
  compresses navigation, scrolling, editing, per-tab keys and the file-link to
  a handful of lines that fit a 90% × 80% popup on an 80×24 terminal.
- `?` is handled in `process_key_event` after the call/device-binding/group
  prompts and after the generic "any key dismisses a popup" branch, but before
  tab dispatch. It is accepted with or without Shift (most layouts type `?` as
  `Shift+/`) and rejected with Ctrl or Alt.
- The binding only fires in `InputMode::Chat`, so an editor, the Contacts
  search, a dial prompt and the Settings network-peer prompt keep their literal
  `?`. Because the help payload is an ordinary `state.popup`, the existing
  any-key/Esc/left-click dismissal still works and no new
  `TuiRenderState` bool was added (`struct_excessive_bools` is at its limit).
- `render_frame` recognizes the exact help payload and calls
  `render_shortcuts_help_popup` for a larger, titled popup; ordinary notices
  keep the original 70% × 40% "Details" popup. The geometry is shared through
  one private helper.
- `shortcuts_text` advertises `?: help` on every tab.

## Flutter / Web

- Settings gained a keyed **Help / controls** card and a scrollable dialog. The
  content lives entirely in Dart (`_helpSections` in `settings.dart`): bottom
  navigation, chat gestures, contacts, groups/calls and a keyboard note.
- The wording is deliberately honest: this app installs no custom key
  handlers. It documents the on-screen buttons, double-tap selection, sender
  name/attachment taps, column-header sorting and standard text-field Enter
  behavior, while pointing at the TUI's own `?` table.
- Flutter Web uses the same screen over `HttpNodeApi`, so the web client gets
  the dialog without a second implementation. No FRB signature, web JSON
  method, wire field or DB migration was needed.

## Tests

- `tests/unit/unit_bin_tui_input_processor.rs`: `?` opens help without typing;
  any key dismisses it; `?` stays literal while editing a nickname; Ctrl+`?`
  does not open it.
- `tests/tui_render_integration.rs`: every tab's shortcut bar advertises
  `?: help`; the help text covers the core bindings; the dedicated popup
  renders its heading and sections at 80×24.
- `apps/flutter_app/test/widget/settings_test.dart`: the keyed card opens the
  dialog, the gesture/keyboard sections render, and Close dismisses it.

## Verification

Green: `cargo fmt --all -- --check` · `cargo ct` · stable-clippy gate ·
`cargo doc --no-deps --all-features --lib` with `RUSTDOCFLAGS='-D warnings -W
missing_docs'` · `cargo build --no-default-features --features
basic,mdns,sqlite_bundled` · `flutter analyze` / `flutter test` (98 tests) ·
`cargo audit` · `python3 scripts/check_docs_links.py`. The new Rust tests were
also run directly (`--bin p2p_chat_tui shortcut`, 5 passed;
`--test tui_render_integration shortcut`, 9 passed).

`cargo test --all-features` passes every suite except two pre-existing,
network-dependent cases in `tests/p2p_integration.rs`:
`test_auto_discovery_via_mdns` and
`test_connection_with_stale_db_address_and_mdns_recovery`. Both time out with
`mdns_discovered=false` in this container, including when run individually;
mDNS multicast is unavailable here. This slice touches only TUI input/rendering,
Dart settings UI and docs, so neither test exercises changed code.
