> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web share deletion (2026-11-27)

`api::delete_share` and both transports' `deleteShare` method already existed,
but no Flutter/web screen exposed them: a share could be created, inspected and
published from the browser but never removed locally. TUI deletion publishing
is a different action (it sends the next revision to a peer); this slice adds
the local, confirmed removal in the shared detail route.

## What shipped

- `ShareDetailScreen` has an optional `onDeleteShare` seam. When Home supplies
  it, the app bar shows a delete action. The confirmation dialog states that the
  share row, entries and grants are removed from this device while the bound
  folder and stored blobs stay, and that peers that already accepted the share
  keep their copy. Cancel changes nothing; a failed call leaves the dialog open
  with the Rust error inline; success pops the detail route.
- Home passes `nodeApi.deleteShare` through that seam on both FRB and web. The
  detail route's completion refreshes the canonical store, so Home's badge
  count and the share list agree.
- `ShareListScreen` accepts an optional `ValueListenable<List<MobileShare>>`.
  Home now updates it from `_refreshShares`, so an already-open list route sees
  creations and deletions instead of showing the list captured when the route
  was pushed. Plain-list callers (widget tests and embeds) remain unchanged.
- `FakeNodeApi.deleteShare` records calls and supports an override instead of
  throwing; list/manifest/grant loaders gained overrides so the real Home path
  can be driven without FFI. Widget tests cover the hidden action, cancel,
  confirmed delete with route pop, inline failure, notifier refresh, and the
  full Home → Settings → File shares → detail → delete wiring.

## Why no Rust or bridge change

`shares::delete_share` already performs the whole local removal in one DB
transaction and drops pending applies; it deliberately leaves the user's files
and content-addressed blobs untouched. The FRB binding and web `deleteShare`
arm already mirror it, so this was purely a frontend exposure. No generated
Dart, `NodeApi`, or wire shape changed.

Commit: to be linked by git history; no hash is invented in this record.
