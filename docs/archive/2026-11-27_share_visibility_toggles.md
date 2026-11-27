> **ARCHIVED** — historical record, not current behaviour.

# Share visibility toggles (2026-11-27)

The share store already carried `ShareKind` and `public_write`, but neither
frontend could change them: Flutter's detail route only displayed the values,
and the TUI Shares tab showed them without an editor. This slice adds one
canonical setter and owner-only controls in both frontends.

## What shipped

- `shares::set_visibility(share_id, expected_owner_user_id, kind, public_write)`
  is the single write path. It loads the share, refuses a caller whose resolved
  user is not the stored owner, normalises `public_write` through
  `effective_public_write` (so a private share can never grant anonymous
  writes), returns the updated `Share`, and touches no revision or ACL rows.
- `api::set_share_visibility(share_id, kind, public_write)` resolves the local
  user via `local_share_owner()` instead of trusting a client-supplied owner,
  parses the canonical `private`/`public` kind, and returns the updated
  `MobileShare`. FRB exposes `setShareVisibility`; the web JSON arm forwards
  camelCase `shareId`/`kind`/`publicWrite` to the same wrapper.
- TUI: `p` flips the highlighted share between private and public (preserving
  anonymous writes only when staying public), `w` toggles anonymous writes and
  refuses private shares. Both check `AppState::own_user_id` against the stored
  owner, set a visible notice on success, and reload the canonical store. The
  detail pane shows the hints only on owned shares.
- Flutter: `ShareDetailScreen` takes an optional `ownUserId` and visibility
  callback, shows Private/Public buttons plus an "Allow anonymous writes"
  switch only on owned shares, and replaces its local row with the updated
  `MobileShare` returned by Rust. A failure stays inline. Home supplies the
  callback from the active `NodeApi`, so FRB and web use the same Rust setter.

## Design decisions

- **The caller never supplies the owner id.** Both frontends hide controls for
  peer-owned shares, but the API still resolves the local user itself and the
  domain setter rechecks ownership. A stale UI or hostile web client cannot
  reclassify an accepted peer share.
- **Normalise at the write boundary.** `effective_public_write` already kept
  private shares write-free at creation; reusing it here avoids a second rule
  for updates and keeps ACL evaluation unchanged.
- **Return the updated row.** FRB and web both answer with `MobileShare`, so
  Flutter does not re-read the list after a toggle; the TUI does reload because
  its pane also shows grants and events from one canonical snapshot.
- **Deferred.** Native create/bind/snapshot and TUI offer/manifest/publish
  actions remain open; this slice only closes the visibility half of the TUI
  share-action backlog item.

## Tests

- `tests/unit/unit_shares.rs`: owner toggles public/private, private writes are
  normalised away, foreign owners and unknown shares are refused.
- `tests/unit/unit_bin_tui_input_processor.rs`: real `p`/`w` key sessions assert
  DB effects, rendered detail text, notices, ownership/private-write refusals
  and no command traffic.
- Web: `methods::dispatch(name, Null)` routing and the no-param drift list
  include `setShareVisibility`; Dart `HttpNodeApi` asserts the camelCase frame
  and parsed row.
- Flutter widget tests cover owner-only visibility controls, the returned-row
  update and the absence of controls on a peer-owned share.

Commit: see the git history for this dated slice. No hash is invented here.

## Related records

- [`2026-11-26_tui_readonly_shares_tab.md`](2026-11-26_tui_readonly_shares_tab.md)
- [`2026-11-27_tui_share_grant_editor.md`](2026-11-27_tui_share_grant_editor.md)
- [`2026-11-24_flutter_share_grant_editor.md`](2026-11-24_flutter_share_grant_editor.md)
- [`../files_attachments.md`](../files_attachments.md)
