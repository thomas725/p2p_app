# ARCHIVED — TUI `Ctrl+P` Peer Info parity (2026-11-26)

**Closed 2026-11-26.** This document records a completed slice; it is not
maintained and does not describe current behaviour.

## Goal

The TUI mouse handler already opens a sender's Peer Info when a message in the
Chat or Group chat tab is clicked, and `Ctrl+P` opened the DM partner's Peer
Info on a Direct tab only. Keyboard users had no way to reach a sender's Peer
Info without the mouse. Close that gap without changing the wire, DB or FRB
surfaces, and keep `Ctrl+P` from ever typing a literal `p` into a composer.

## Design

- **One tab-aware resolver** — `AppState::peer_info_target(tab)` answers "which
  peer does Peer Info show here?": the selected device on Contacts, the partner
  on Direct, the newest row with a known sender on Chat/GroupChat (our own rows
  carry no sender), and `None` everywhere else — including Log, whose lines
  have no sender. Both the `i` action on Contacts and the control chords now
  use it, so the keyboard and the existing click handler agree on the target.
- **`Ctrl+P` is universal** — `handle_global_navigation` consumes
  `Char('p')`+CONTROL on *every* tab, calling the resolver and no-oping safely
  when there is no peer. `Ctrl+I` stays Direct-only and kitty-keyboard-only;
  on other tabs it still falls through to the chat composer, so bare letters
  and bare `Tab` are unaffected.
- **Honest hints** — the shortcut bar advertises `Ctrl+P` only where a target
  exists (Contacts, Direct, Chat, Group chat) and never on Log/Settings/Groups/
  Shares/PeerInfo. The `?` help and `KEYBOARD_SHORTCUTS.md` name the
  Chat/GroupChat sender rule and the Log no-op.

## Tests

- `tests/unit/unit_bin_tui_input_processor.rs`: `Ctrl+P` in Chat opens the
  newest sender, in GroupChat it skips the local row to the newest known
  sender, and on Log it is a consumed no-op that types no `p`.
- `tests/tui_render_integration.rs`: the hint appears only on tabs with a peer
  target; the old assertion that Chat has no Peer Info hint was corrected.

## Documentation

`KEYBOARD_SHORTCUTS.md` lists the Chat/GroupChat binding, the Log no-op and the
Contacts `i`/`Ctrl+P` pair. Codebase metrics regenerated; all doc links resolve.
