> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web private-share grant editor (2026-11-24)

The read-only share detail route already listed private-share grants. This
slice turns that section into a real permission editor using the existing FRB
and web facade — no new Rust, FRB or web binding was needed.

## What shipped

- `ShareDetailScreen` takes optional `onSetGrant(shareId, userId, mode)` and
  `onRemoveGrant(shareId, userId)` callbacks. With neither supplied the page
  stays read-only (and FFI-free in widget tests); with them the grant rows gain
  revoke buttons and an "Add grant" button.
- The add dialog offers exactly `readonly` and `readwrite`, the two modes
  `parse_share_mode` accepts. It trims the user ID, never calls the node for an
  empty one, keeps the dialog open on failure with the error inline, and
  reloads grants after a successful write. Revoke failures surface as an inline
  error beside the grant list. Both paths use `mounted` checks after awaits.
- `HomeScreen` supplies both callbacks from the active `NodeApi`, so the same
  editor works in-process (Android/FRB) and over the browser's JSON transport.
  Public shares still have no ACL section; a share with no grants shows the
  "No explicit grants." empty state.
- `test/widget/share_list_test.dart` gained six tests: the read-only fallback,
  trimmed user ID + selected mode on add, empty-ID no-op, revoke + reload, add
  failure staying in the dialog, and revoke failure showing an inline error.

Creation, binding, network refresh (re-request/re-advertise) and publishing
remain deferred UI work; the browser also still cannot bind a server-local
folder.
