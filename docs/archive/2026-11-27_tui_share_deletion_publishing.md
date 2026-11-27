> **ARCHIVED** — historical record, not current behaviour.

# TUI share deletion publishing (2026-11-27)

The Shares tab could create shares, edit grants and visibility, and run network
actions, but a terminal user could not submit a manifest revision. This slice
adds the browser-safe case: delete files from a manifest we already have and
submit the next revision to a connected peer through the canonical
`PublishShareSnapshot` command. No FRB/web method was added.

## What shipped

- Manifest files have their own cursor, separate from the connected-peer and
  grant cursors: `Ctrl+Left`/`Ctrl+Right` moves it, `d` toggles a `[x]` deletion
  mark, and `Ctrl+D` publishes. Bare `Left`/`Right` still select the network
  target, and every new chord is inert while a modal editor owns the keyboard.
- `share_deleted_paths` is a `BTreeSet`, so a repeated `d` unmarks the path
  instead of submitting it twice. The renderer shows the mark and a stale-base
  hint using the same projection as the input handler.
- The base revision comes from the cached manifest and is validated with the
  single canonical `shares::manifest_matches_share` check. The candidate is
  rebuilt by `shares::manifest_with_deletions` from the store, and a candidate
  whose revision does not follow the cached base is refused before a command is
  queued — a store that moved since the view loaded cannot produce a guaranteed
  rejection. The owner-side ACL and `base_revision` check remain authoritative.
- Share changes, reloads, revision changes and vanished paths reset the draft;
  a successful submission clears the marks and leaves a "Deletion of N
  file(s) submitted to <peer>" notice. The stale hints point to `r`, the local
  reload key.

## Tests

Real-key sessions cover cursor isolation from the peer/grant cursors, toggle
and publish, the store-race guard, stale and vanished paths, draft resets, and
modal ownership. Renderer integration coverage asserts the `[x]` marker and the
stale hint. `shares::manifest_matches_share` has direct tests for missing,
current, stale and wrong-share snapshots.

## Why this design

The TUI has no direct store-write permission: peer shares and the optimistic
revision check live on the owner side of the share lane. Reusing
`PublishShareSnapshot` keeps one ACL path for every frontend. Rebuilding the
candidate in Rust avoids duplicating path validation and revision policy in the
terminal, and checking the rebuilt revision against the cached base closes the
window where the store advanced after the manifest was displayed.
