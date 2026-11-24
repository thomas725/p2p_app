> **ARCHIVED** — historical record, not current behaviour.

# Read-only Flutter share list/detail (2026-11-24)

The share Rust core, FRB/web facade and two-node blob-pull coverage already
existed; the only missing frontend piece was a way to see what the node knows.
This slice adds a **read-only** list and detail route that works identically
over the in-process FRB transport and the browser's JSON transport.

## What shipped

- `lib/src/screens/share_list.dart`: `ShareList` (pure list widget with an empty
  state and per-share rows), `ShareListScreen` (full-screen route with the shared
  `MainTabNavBar`), and `ShareDetailScreen` (overview, private-share grants and
  manifest files).
- `Settings` gained an optional `onOpenShares`/`shareCount` pair. Home supplies
  it, so the "File shares" entry stays hidden in Settings widget tests that do
  not own a node. The Help dialog documents the new route.
- `HomeScreen` owns `_shares`, refreshes it at startup, and reloads it on
  `share_offered` / `share_updated` / `share_rejected` events. Detail loaders
  are injected from the active `NodeApi` (`loadShareManifest` /
  `listShareGrants`), the same seam pattern `PeerInfoScreen` uses, so the widget
  needs no Rust library in tests.
- `test/widget/share_list_test.dart` covers the empty state, row rendering and
  tap-through, detail rows (including the public-share case with no grants), and
  the refresh button re-running both loaders.

No new Rust, FRB or web binding was needed: `listShares`, `loadShareManifest`
and `listShareGrants` were already exposed on both transports. Creation,
binding, permission editing, network refresh and publishing remain deliberately
out of this UI slice.
