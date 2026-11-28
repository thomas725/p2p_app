> **ARCHIVED** — historical record, not current behaviour.

# TUI in-app help matches the real bindings (2026-11-28)

The `?` help overlay and the per-tab shortcut bars had fallen behind the code:
`Ctrl+G` (open a location's canonical map) worked in all three chat tabs but
was nowhere advertised, the Shares line omitted the create/bind/snapshot/
publish and visibility keys, and the Esc sentence described a shorter unwind
chain than the dispatcher actually implements. This slice made the in-app
documentation match the bindings in `KEYBOARD_SHORTCUTS.md`.

## What shipped

- The Chat, Direct (kitty and non-kitty) and Group shortcut bars now include
  `Ctrl+G: open map`; the other tabs deliberately do not.
- The help overlay's chat line documents `Ctrl+G`, and the Shares line lists
  `c` create, `b` bind/clear folder, `s` snapshot, `u` publish and `p`/`w`
  visibility alongside the existing discovery/grant/deletion keys.
- The Esc sentence now says "active prompt/editor" instead of a single named
  prompt, reflecting that any modal editor owns the first Esc.
- `tui_render_integration.rs` asserts the `Ctrl+G` hint appears only on chat
  bars and that the new help lines are present, so the bars cannot drift from
  the dispatcher again.

No runtime binding changed; `KEYBOARD_SHORTCUTS.md` was already current.
