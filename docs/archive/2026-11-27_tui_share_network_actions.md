> **ARCHIVED** — historical record, not current behaviour.

# TUI share network actions (2026-11-27)

The Flutter/web detail route could already refresh a share manifest and
re-advertise a canonical offer from a selected connected peer. The TUI Shares
tab could list shares, grants and manifests but had no network actions. This
slice brings both actions to the terminal without adding an FRB/web method, a
wire field or a Dart/Kotlin change.

## What shipped

- `ConnectedTracker::sorted_connected_peer_ids` is the single source for the
  connected peers shown by the Shares detail pane. The same vector is projected
  into `TuiRenderState`, so a key and the highlighted row cannot disagree about
  which peer an action targets.
- `Left`/`Right` move `AppState::share_peer_selection` over that canonical list.
  Connect/disconnect handling clamps the cursor, and a disconnected peer cannot
  be silently addressed by a stale index.
- `m` queues `SwarmCommand::RequestShareManifest { peer_id, share_id }` for the
  selected share and peer. `o` calls `shares::offer_for_share` first and only
  queues `SwarmCommand::SendShareOffer` when Rust has built a canonical offer;
  a deleted or unknown share is rejected before a command is sent.
- Both actions surface their outcome in the existing share-event notice. A
  missing share or connected peer shows a popup and queues nothing.
- The detail pane lists connected peers under the manifest section with the
  selected peer marked. The shortcut bar and `?` help document the bindings
  only on the Shares tab.

## Design decisions

- **No duplicate offer builder.** The TUI calls the same `offer_for_share` used
  by `api::readvertise_share` and the Flutter/web facade. The local-only
  `unknown:share:<id>` owner marker is normalised to `owner_user_id: None`; a
  terminal user cannot invent an ownership claim on the wire.
- **One ordering path.** The renderer and input both consume
  `sorted_connected_peer_ids`. No caller sorts a hash set locally or carries a
  raw index into a different list.
- **Modal editors own the keyboard.** The new peer cursor and `m`/`o` handlers
  sit behind the shared `InputMode::is_modal_editor()` boundary, so a grant or
  nickname prompt keeps every key until it is committed or cancelled.
- **No transport churn.** The TUI already owns a `SwarmCommand` sender, so no
  FRB regeneration, web dispatch arm, JSON method or Dart change was needed.
- **Deferred.** Share discovery, deletion publishing, native creation/binding
  and folder snapshots remain outside the terminal UI.

## Tests

- `unit_connected.rs` — deterministic sorted-peer ordering across connect and
  disconnect.
- `unit_bin_tui_state.rs` — canonical selection, Left/Right movement and
  clamping when the peer list shrinks.
- `unit_bin_tui_input_processor.rs` — real key sessions through
  `process_input_event` assert the exact manifest-request and offer command
  payloads, the visible peer marker, missing-share/peer failures, a deleted
  share rejected before any command, and inert `Left`/`m`/`o` while the grant
  editor is open. DB-backed cases hold `shared_db_test_lock()`.

Commit: to be linked by the caller when this slice lands. No hash is invented
in this historical record.

## Related records

- [`2026-11-27_rust_owned_share_readvertise.md`](2026-11-27_rust_owned_share_readvertise.md)
- [`2026-11-27_tui_share_grant_editor.md`](2026-11-27_tui_share_grant_editor.md)
- [`2026-11-26_tui_readonly_shares_tab.md`](2026-11-26_tui_readonly_shares_tab.md)
- [`../files_attachments.md`](../files_attachments.md)
