> **ARCHIVED** — historical record, not current behaviour.

# TUI share creation (2026-11-27)

The Shares tab could list shares, edit grants and visibility, and run network
actions, but a terminal user could not create a share. This slice adds a modal
name prompt that creates a private, read-only share through the canonical Rust
store — no new FRB/web method, JSON arm or Dart/Kotlin change.

## What shipped

- `c` on the Shares tab opens a modal name prompt backed by the shared
  `chat_input`. `InputMode::CreatingShare` joins
  `InputMode::is_modal_editor()`, so Tab, arrows, the wheel and tab clicks keep
  their ordinary "prompt owns the keyboard" behaviour and a tab switch cancels
  the draft through `finalize_tab_input()`.
- `Enter` commits through `AppState::confirm_create_share`, which calls
  `shares::create_share(name, owner, ShareKind::Private, false, None)`. The
  local root is deliberately absent: binding a real folder stays a native-only
  action. Name validation and the DB write remain in the canonical store.
- A successful create closes the prompt, reloads the share list, selects the
  new row (resetting the grant cursor through `select_share`) and surfaces
  `Share created: <name>` in the existing share-event notice.
- An invalid or empty name, or a missing local owner, keeps the editor open with
  its text and a popup describing the failure. Esc cancels without a write.
- The detail pane and `?` help document `c` on the Shares tab only.

## Design decisions

- **One creation path.** The TUI calls the same `shares::create_share` used by
  the FRB/web facade, so validation, owner checks and the DB schema cannot
  drift between frontends. There is no TUI-only share writer.
- **Private/read-only defaults.** A newly created share grants nobody anything
  until a verified user id is added with `a`; `public_write` stays false, so a
  private share can never grant anonymous writes.
- **No local root.** The prompt captures a display name only. A path supplied
  from the terminal would bypass the share lane's bound-folder contract, so
  binding and folder snapshots remain deferred.
- **Shared input, no extra bool.** `InputMode::CreatingShare` extends the
  existing single-slot editor state rather than adding a field to
  `TuiRenderState` (already at clippy's `struct_excessive_bools` limit).
- **Deferred.** Binding, snapshots, discovery and publish actions remain
  outside the terminal UI.

## Tests

- `unit_types.rs` — `CreatingShare` is a modal editor; Contacts search and
  ordinary chat are not.
- `unit_bin_tui_state.rs` — prompt lifecycle: begin, modal membership,
  `finalize_tab_input` cancellation and idempotent explicit cancel.
- `unit_bin_tui_render_loop_layout.rs` — the live input section renders
  `Create share (name)` and the typed name on the Shares tab, which has no
  ordinary input box.
- `unit_bin_tui_input_processor.rs` — real key sessions through
  `process_input_event` assert persistence of one private/read-only share with
  no local root, selection and event notice, inert Tab/Down/wheel while the
  prompt owns input, rejection of empty/whitespace names with text retained,
  Esc leaving the store untouched, and the missing-owner error path. DB-backed
  cases hold `shared_db_test_lock()`.

Commit: to be linked by the caller when this slice lands. No hash is invented
in this historical record.

## Related records

- [`2026-11-27_tui_share_grant_editor.md`](2026-11-27_tui_share_grant_editor.md)
- [`2026-11-27_tui_share_network_actions.md`](2026-11-27_tui_share_network_actions.md)
- [`2026-11-27_share_visibility_toggles.md`](2026-11-27_share_visibility_toggles.md)
- [`../files_attachments.md`](../files_attachments.md)
