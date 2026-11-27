> **ARCHIVED** — historical record, not current behaviour.

# TUI local share deletion (2026-11-27)

Flutter/web's detail route had just gained a confirmed local `deleteShare`
action; the TUI could create shares and publish manifest revisions but had no
way to remove a share locally. This slice closes that parity gap without
changing the Rust deletion policy or any FRB/web method.

## What shipped

- `D` on the Shares tab arms a new single-slot `share_delete_prompt`. The prompt
  captures the highlighted share's id and display name at arm time; while it is
  open every key belongs to it (`y`/Enter confirms, any other key or `Esc`
  cancels), and `has_prompt_or_call_overlay` makes wheel and tab clicks inert.
- Confirmation calls the existing `p2p_app::api::delete_share` facade, which
  delegates to `shares::delete_share`: entries, grants and the share row are
  removed in one transaction and pending applies are dropped. The canonical
  view is reloaded and a `Share deleted: <name>` notice appears in the detail
  pane. No swarm command is queued — peers that already accepted the share keep
  their copy, and the bound folder plus content-addressed blobs stay on disk.
- The confirmation popup spells out exactly what is kept, distinguishing `D`
  from `d`/`Ctrl+D` (mark and publish manifest deletions to a peer). The Shares
  shortcut bar, `?` help table and `KEYBOARD_SHORTCUTS.md` document `D`.
- Prompt handling was extracted into `handle_prompt_action` so adding a prompt
  did not need a `too_many_lines` allow; terminal defaults moved into a module
  constant for the same reason. The strict no-new-allows policy is preserved.
- Real-key tests in `tests/unit/unit_bin_tui_input_processor.rs` cover arming
  from `D` (including the rendered warning), DB removal of the share and its
  grants on confirmation with the reloaded notice, cancel/Esc leaving the store
  untouched, and mouse/keyboard ownership while the prompt is up.

## Why no Rust or bridge change

`api::delete_share` already mirrored `shares::delete_share` on FRB and web, so
both frontends now call the same authoritative path. Adding a TUI binding does
not change generated Dart, the web method list or the wire protocol.

Commit: to be linked by git history; no hash is invented in this record.
