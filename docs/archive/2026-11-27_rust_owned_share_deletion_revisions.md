> **ARCHIVED** — historical record, not current behaviour.

# Rust-owned share deletion revisions (2026-11-27)

The share publish editor previously built the next `ShareManifest` in Dart: it
filtered entries, copied the id/name and incremented the revision itself. That
duplicated Rust's canonical path rules and let a client author a wire-level
manifest before the owner-side ACL check. This slice moves the whole revision
build behind Rust and leaves Dart only the share id and the user's path
selection.

## What shipped

- `shares::manifest_with_deletions(share_id, removed_paths)` reads the canonical
  stored snapshot, validates every selected path against that snapshot, rejects
  an empty or unmatched selection, preserves the id/name/remaining entries and
  bumps the revision with `saturating_add(1)`. The returned manifest is only a
  candidate; it is not persisted and still goes through the owner-side ACL and
  `base_revision` check on publish.
- `api::revise_share_manifest_for_deletion` exposes that helper over FRB with an
  input struct, mirroring the existing grouped-parameter pattern. Web exposes
  `reviseShareManifestForDeletion` with camelCase `shareId`/`removedPaths` and
  converts the result through `shares_json::ShareManifestJson`, so the browser
  never sees Rust's snake_case field names.
- `NodeApi`, `FrbNodeApi`, `HttpNodeApi` and the shared fake implement the new
  binding. `SharePublishSection` takes an injected revision callback and no
  longer contains any revision arithmetic; a revision failure surfaces inline
  and publish is never called. Stable widget keys are unchanged.
- Web dispatch has a parameter-shape test and is covered by the generated-source
  drift guard. Rust unit tests cover happy path, duplicate paths, empty
  selection, unsafe/absent paths, unknown share and a share without a manifest.

## Design decisions

- **Rust owns the manifest.** Canonical path validation, the revision counter
  and the preserved fields live beside the share store, so the editor cannot
  diverge from the owner-side acceptance rules.
- **Selection is the only Dart policy.** The checkbox set is user intent; every
  structural decision about the next manifest is Rust's.
- **Publish stays a separate step.** The helper does not write to the DB. The
  owner still verifies the sender's user id, applies the manifest to its bound
  folder and rejects a stale `base_revision` atomically.
- **Web keeps camelCase.** The browser speaks the same flat JSON shape as the
  other share methods; only the FRB adapter uses the generated params struct.

## Tests

- `tests/unit/unit_shares.rs` — revision helper success and rejection cases.
- `src/web/methods.rs` — dispatch routing and camelCase parameter parse.
- Flutter `share_publish_test.dart` — Rust callback arguments, returned
  manifest passed to publish, revision failure and stable keys.
- Flutter `share_list_test.dart` — detail route supplies the revision callback.
- Flutter `http_node_api_test.dart` — outbound JSON frame and result parsing.

Commit: to be linked by git history; no hash is invented in this record.

## Related records

- [`2026-11-26_flutter_share_publish.md`](2026-11-26_flutter_share_publish.md)
- [`2026-11-27_share_manifest_hardening.md`](2026-11-27_share_manifest_hardening.md)
- [`../files_attachments.md`](../files_attachments.md)
