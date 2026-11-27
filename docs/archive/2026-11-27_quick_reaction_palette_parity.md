> **ARCHIVED** — historical record, not current behaviour.

# Quick-reaction palette parity (2026-11-27)

Flutter's reaction menu offered six quick emoji from a Dart `const`; the TUI
only had `Ctrl+Y` plus a free-form token. This slice makes the palette a
Rust-owned affordance and gives the TUI the same one-key choices, without
changing reaction storage, wire signals, or the existing token rules.

## What shipped

- `src/reactions.rs` owns `QUICK_REACTIONS` (`👍`, `❤️`, `😂`, `🎉`, `😮`,
  `😢`) plus small helpers for indexed access, the FRB/web boundary and the TUI
  title hint. Every palette token still passes through `validate_emoji`.
- The FRB/web facade gains the no-parameter `quickReactions()` method. `NodeApi`
  mirrors it in both Dart transports: FRB calls the generated binding, while
  the web client calls the matching JSON method. Flutter's menu now loads its
  palette through that abstraction instead of a local constant.
- In the TUI, `Ctrl+Y` still opens the reaction editor for the newest stable
  message. Plain `1`–`6` while that editor is open commit the matching palette
  token through the same `commit_reaction` path used by custom tokens, so
  validation, toggling, fan-out and chip refresh stay identical. Enter still
  accepts a custom token; Esc still cancels.
- The editor title and both shortcut references document the palette. The TUI's
  newest-message-only target remains unchanged; arbitrary/multi-row selection
  is a separate, larger parity item.

## Tests

- Rust unit tests check that the palette is indexable, complete, valid under the
  canonical validator, and renders the expected TUI hint.
- Web frame tests check `quickReactions` over JSON and register it in the
  no-parameter dispatch drift list.
- TUI key tests drive `Ctrl+Y` followed by `1`/`6`, assert the canonical tokens
  fan out and persist, and confirm plain digits remain composer text outside
  the editor.
- Flutter widget tests replace `nodeApi` with a fake and assert the reaction
  menu loads its palette from the API, including custom palette entries; the
  existing custom-token validation tests still pass.

No DB migration, wire type, reaction signal or persisted field changed.
