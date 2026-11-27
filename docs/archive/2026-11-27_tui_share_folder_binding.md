> **ARCHIVED** — historical record, not current behaviour.

# TUI share folder binding and snapshots (2026-11-27)

The Rust core already had canonical helpers to validate a share root, store the
binding and snapshot a folder, but the TUI could only create empty shares. This
slice wires those helpers into the Shares tab so a terminal user can bind a
folder, turn it into the next manifest revision and submit that revision to a
connected peer without duplicating path or manifest policy in the frontend.

## What shipped

- `b` arms a modal folder editor for the highlighted share. The share id is
  captured when the prompt opens, so selection changes, reloads and concurrent
  share events cannot redirect the write. The current root is prefilled and the
  cursor starts at its end, making an existing path easy to edit or clear.
  - `Enter` commits through `shares::set_local_root`. An empty line clears the
    binding; a rejected path keeps the editor text open with an error popup.
  - `Esc` and tab finalization cancel without touching the store.
  - Peer-owned shares may be bound as local mirrors, so the editor does not
    impose an owner check that the canonical store does not have.
  - `InputMode::BindingShare` is part of `is_modal_editor()`, so navigation,
    other Shares shortcuts and pointer input cannot act behind the prompt.
- `s` snapshots the highlighted share's bound folder through
  `shares::snapshot_share`. Rust canonicalizes paths, walks the tree, stores
  blobs and writes the next revision; a missing binding produces a hint rather
  than a guessed path. After a successful snapshot the canonical store is
  reloaded and the same share is reselected by id.
- `u` submits the cached manifest to the selected connected peer through the
  canonical `SwarmCommand::PublishShareSnapshot` command. It requires a fresh,
  non-initial snapshot and uses `revision - 1` as `base_revision`; ACL and
  stale-revision enforcement stay in receiving Rust. A stale cached manifest,
  a vanished share or no connected peer blocks the command before it is queued.
- The input section, detail pane and shortcut bar name all three actions, and
  the oversized tab dispatcher was split into a Shares-only helper instead of
  adding a clippy allowance.
- The FRB `snapshotShare` facade now borrows `&str` on both parameters. The
  generated bridge was regenerated with pinned FRB 2.13.0 and its lint
  allowances restored; the Dart-facing `String` API and web JSON are unchanged.

## Tests

A state test covers prompt target capture, prefilling and modal lifecycle.
Real-key sessions cover bind → snapshot → publish end to end, clearing a
binding, target stability after reload, invalid-root correction and modal
inertness. Strict clippy, the all-available-features Rust suite, `flutter
analyze` and `flutter test` all pass.

## Why this design

Path validation, canonicalization, blob storage and revision assignment already
belonged to `shares.rs`; letting the TUI choose a root string or build a manifest
would have created a second policy path. Capturing the share id in the prompt
keeps the modal write deterministic, and reusing the existing publish command
keeps ACLs and optimistic concurrency at the receiving swarm boundary. Adding
files from an explicit picker remains a separate UI slice.

Commit: to be linked by git history; no hash is invented in this record.
