> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web share publishing (2026-11-26)

The `publishShareSnapshot` facade already existed on both `NodeApi` transports,
but no frontend could construct a submission. This slice exposes the useful
browser-safe case: deleting files from a manifest we have already loaded.

## What shipped

- `ShareDetailScreen` gained an optional "Publish revision" section. It appears
  only when Home supplies an `onPublish` callback, so widget tests and read-only
  transports stay FFI-free.
- `SharePublishSection` shows the loaded manifest's entries with removal
  checkboxes and a connected-peer picker. `revisedManifestForDeletion` keeps the
  share id/name, drops selected entries and advances the revision by one; an
  empty selection, empty share id or non-positive revision is not publishable.
- The base revision is the loaded manifest's revision, and the UI refuses to
  publish when it no longer matches the share row. Stale peer selections reset
  with the roster, and a refreshed manifest clears the old selection. Failures
  stay inline; a successful submission reloads the detail.
- The callback passes `BigInt` values through Dart; Home converts the base
  revision to the platform's `PlatformInt64` at the `shares_json.dart` boundary.
  The selected peer is only a destination: Rust still checks the verified user's
  ACL and the owner's current revision atomically. An asynchronous owner
  rejection arrives as the existing `share_rejected` event.
- Ten widget/unit tests cover the revision helper, transport conversion, no
  manifest, stale state, no peers, exact submission, roster changes, refresh
  reset and publish failure. No FRB regeneration or new Rust binding was needed.

Adding new files still requires a native folder snapshot; the browser remains a
display client. The TUI Shares tab remains deferred.
