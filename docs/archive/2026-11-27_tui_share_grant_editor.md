> **ARCHIVED** — historical record, not current behaviour.

# TUI private-share grant editor (2026-11-27)

The TUI Shares tab could list shares, grants and manifest files, but grants were
read-only: the Flutter/web detail route already had an editor calling the same
Rust store. This slice brings that editor to the terminal without adding an FRB
binding, a web method or a JSON field.

## What shipped

- `a` arms an add-grant prompt on the selected private share. The shared input
  takes a verified `user_id`; `Tab`/`BackTab` toggles between the canonical
  `readonly` and `readwrite` modes (both renderers show the current mode), and
  `Enter` calls `shares::set_grant`.
- `x` arms a revoke prompt for the highlighted grant. `Ctrl+Up`/`Ctrl+Down`
  moves the grant cursor independently of the share row. The prompt carries
  `ShareGrantAction::Revoke { user_id }`, so the highlighted grant is the
  authority for the DB write; the prefilled text is only a confirmation and
  stray keystrokes cannot retarget the revoke. `Enter` calls
  `shares::remove_grant`.
- Public shares and empty selections are rejected with a hint, as is an empty
  user id in the add prompt. A store failure keeps the editor open and surfaces
  the error; a successful commit cancels the prompt, stores a visible notice,
  and reloads shares and grants from Rust.
- `InputMode::EditingShareGrant` participates in
  `InputMode::is_modal_editor()`, so dispatch, navigation, both input renderers
  and pointer ownership agree on one boundary. `Tab` is the deliberate
  exception: it changes the add mode instead of cycling tabs. `Esc` cancels in
  the established one-level chain, after reaction and before Contacts search;
  switching tabs cancels an open editor through `finalize_tab_input()`.
- `AppState::select_share` is the single selection path for wheel, arrow and
  click input. It clamps the share row and resets the grant cursor when the
  share changes, so an index from one share can never revoke a user on another.
  `reload_shares()` clamps the grant cursor after a store change.

## Design decisions

- **No new canonical core API.** `shares::set_grant`/`remove_grant` were already
  the authority for Flutter/web; the TUI calls them directly instead of adding
  a duplicate binding. No `flutter_rust_bridge_codegen generate`, web dispatch
  arm or wire type changed.
- **The stored action owns the revoke target.** A revoke prompt's text starts
  as the highlighted user id but is intentionally not editable, because a
  confirmation cannot safely double as an input field. This also keeps the
  test contract explicit: typing cannot redirect `remove_grant`.
- **Share-scoped cursor.** The grant index is only meaningful for the selected
  share. Resetting it on selection change is safer than clamping a stale index
  into a different ACL, and it matches Flutter's per-detail-route state.
- **Deferred.** Public/write toggles still have no canonical setter, and
  create/bind/snapshot/offer/manifest/publish remain native or Flutter/web
  actions.

## Tests

Real `process_input_event` sessions drive the production frame path:

- `test_share_grant_add_through_real_keys_persists_and_reloads` — `a`, typed
  user id, `Tab`, `Enter`; asserts DB mode, visible frame and clean editor.
- `test_share_grant_revoke_uses_highlighted_grant_not_typed_text` — `x` on one
  of two grants, stray keys, `Enter`; asserts only the stored target is removed.
- `test_share_grant_add_rejects_empty_user_id_without_writing` — validation
  keeps the prompt open and the store untouched.
- `test_share_grant_actions_refuse_public_shares_and_missing_grants` — wrong
  share kind and empty selection show hints without writing.
- `test_share_grant_revoke_without_selected_grant_shows_hint` — an empty grant
  pane cannot arm a revoke.
- `test_share_grant_editor_owns_tab_and_mouse_until_cancelled` — `Tab` changes
  mode, navigation and pointer input are inert, and `Esc` leaves persisted
  grants intact.
- `test_share_selection_resets_grant_cursor_through_real_mouse_input` — wheel
  and click both route through `select_share` and reset the grant cursor.

Commit: to be linked by the caller when this slice lands. No hash is invented
in this historical record.

## Related records

- [`2026-11-26_tui_readonly_shares_tab.md`](2026-11-26_tui_readonly_shares_tab.md)
- [`2026-11-27_tui_modal_editor_boundary.md`](2026-11-27_tui_modal_editor_boundary.md)
- [`2026-11-27_tui_mouse_ownership.md`](2026-11-27_tui_mouse_ownership.md)
- [`2026-11-24_flutter_share_grant_editor.md`](2026-11-24_flutter_share_grant_editor.md)
- [`../files_attachments.md`](../files_attachments.md)
