> **ARCHIVED** — historical record, not current behaviour.

# Native Flutter share folder UI (2026-11-28)

The Rust core and both frontend facades already exposed `bindShare` and
`snapshotShare`, and the TUI had a folder workflow. Flutter/web could create,
delete, discover, publish and inspect shares but could not bind a local folder
or take a native snapshot from the detail route. This slice closes that adapter
gap without changing any Rust policy, FRB signature or web method.

## What shipped

- `ShareDetailScreen` has an optional **Local folder** section with Bind folder,
  Clear binding and Snapshot folder actions. Home supplies the callbacks only
  when `nodeApi.isRemoteTransport` is false; the browser keeps its display-client
  contract and never sees the actions.
- Directory selection goes through `pickDirectoryOverride` in the new
  `src/util/share_folders.dart`. The real implementation calls the top-level
  `FilePicker.getDirectoryPath()`; tests replace the seam, so no widget test
  touches a platform channel.
- Binding and snapshotting call the existing `NodeApi.bindShare` and
  `NodeApi.snapshotShare`. Rust canonicalises the path, walks the tree into
  content-addressed blobs and bumps the stored manifest revision; Dart only
  forwards the chosen path and displays the returned row. Clearing sends a null
  root, matching the TUI's empty-line binding.
- Home refreshes the canonical share notifier after each action and returns the
  fresh `MobileShare` so the detail route immediately shows the new root or
  revision. Cancelling the picker is a no-op; failures render inline beside the
  buttons.
- Widget tests in `test/widget/share_list_test.dart` cover bind, picker cancel,
  clear, snapshot refresh, missing-root guard and inline failure with injected
  seams. `test/widget/home_share_folder_test.dart` drives real Home → Settings →
  File shares → detail navigation with `FakeNodeApi` on the native transport and
  asserts the `bindShare`/`snapshotShare` calls and arguments.
- Settings help text now describes the native actions instead of calling them a
  deferred concept.

## Why no Rust or bridge change

`api::bind_share` and `api::snapshot_share` already existed on FRB and web, so no
generated Dart, web dispatch arm or protocol field changed. The only new logic
outside the GUI is the platform directory picker, which cannot live in Rust.

Commit: to be linked by git history; no hash is invented in this record.
