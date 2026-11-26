> **ARCHIVED** — historical record, not current behaviour.

# Read-only TUI Shares tab (2026-11-26)

The share Rust core and the Flutter/web list + detail route already existed; the
TUI only logged share events. This slice adds a **read-only** Shares tab so the
terminal frontend can see the same list and manifest detail as Flutter, without
any mutable actions. Creation, binding, snapshots, grants and publish actions
remain deliberately out of this UI slice.

## What shipped

- `TabContent::Shares` is a suffix tab. `SUFFIX_TAB_COUNT` went from 2 to 3 and
  Shares is appended **after** Settings, so every dynamic group/DM/PeerInfo
  index and the `FIXED_TAB_COUNT - 1` Groups assumption stay unchanged.
  `TuiRenderState::new()` and `get_tab_content("Shares")` know the new tab, and
  the shortcut bar / `?` help list its bindings.
- `src/tui_render/shares.rs` renders a two-pane view: a selectable left list
  (`Name [kind] rev N`) using the same windowing math as the Contacts table, and
  a wrapped detail pane with the name, id, kind/write flag, owner, local root,
  revision, the latest share-lane notice and the manifest's files. Private
  shares also show a `Grants (N)` list (`user (readonly|readwrite)`), matching
  Flutter's detail route; public shares hide it because they have no ACL rows.
  File sizes come from the canonical `files::format_file_size`, and the kind /
  mode labels are shared with the FRB/web facade via `shares::kind_label` /
  `mode_label`.
- `AppState` owns `shares`, `share_selection` and `share_event_notice`.
  `reload_shares()` re-reads `shares::list_shares()`, clamps the selection and
  keeps the last good snapshot on a DB error (fail-open, like the other
  best-effort reloads). It runs at startup, on `r`, and on share events.
- `SwarmEvent::ShareOffered` / `ShareUpdated` / `ShareRejected` now call
  `handle_share_event`, which stores a human-readable `share_event_notice`,
  re-reads the canonical store and logs the same text. The core already applied
  any downloaded manifest before emitting the event, so this is a notification
  plus a re-read — never a second apply. The notice wording is a pure function
  with focused unit tests.
- `↑`/`↓`, `PgUp`/`PgDn`, `Home`/`End` navigate the list; a click in the left
  list pane selects that share so the detail pane follows, mirroring Flutter's
  tap-through. Clicks in the read-only detail pane leave the selection alone
  (the 40/60 split is resolved with ratatui's own `Layout`, matching the
  renderer). Mouse scroll is deliberately disabled there (the list follows the
  selection). The `r` refresh works only on the Shares tab so it cannot shadow
  Settings' `r` (add relay).

## Test upkeep

Adding a suffix tab invalidated hardcoded tab counts and "last tab = Settings"
assumptions across the TUI tests. Counts moved 5→6 (and derived +1 per dynamic
tab), title vectors gained `"Shares"`, and the helpers that need Settings now
find it by `TabContent` instead of by index. The full `cargo test --all-features`
suite is green.
