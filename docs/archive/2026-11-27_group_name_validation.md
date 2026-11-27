> **ARCHIVED** — historical record, not current behaviour.

# Group-name validation at the Rust choke point (2026-11-27)

Public v1 groups identify by a hash of their canonical name, so a name with no
canonical content (`""`, `"   "`, tabs/newlines) used to hash to the same fixed
id. Both frontends guarded empty input locally, but the canonical Rust seam
accepted it: a direct FRB/web `createGroup` call (or any future caller) could
create a group whose `group_id` was the hash of an empty string. The UI guards
also disagreed on what to tell the user — the TUI cancelled silently, while
Flutter's dialog just returned.

## What shipped

- `groups::GroupName` is a validated newtype: `GroupName::new` canonicalizes
  (trim + collapse internal whitespace + lowercase) and rejects input with no
  canonical content with a typed `EmptyGroupName` error. Its canonical spelling
  is available through `as_str()`.
- `stable_group_id` now takes `&GroupName` instead of `&str`, so an empty name
  cannot produce a group identity through the canonical path. The hash itself
  and the resulting ids are unchanged for all valid names.
- `create_public_group` validates through `GroupName::new` before doing any DB
  work, so the error surfaces through the existing FRB/web `createGroup` result
  with no new binding and no wire/schema change. The stale doc claiming a
  duplicate name is rejected was removed (the seam is idempotent by design).
- The TUI no longer duplicates the emptiness check. A rejection keeps the
  create prompt open and shows a popup with the Rust error, matching the
  nickname-validation UX so the caller can correct the input instead of losing
  it.
- Flutter's dialog keeps its cheap `isEmpty` early-out; any real validation
  failure still surfaces through its existing error snackbar.

## Tests

- Rust unit tests assert `GroupName::new` rejects empty/whitespace-only input
  and canonicalizes valid names, that `create_public_group("   ")` returns the
  typed error and creates no DB row, and that valid ids/topics still round-trip
  through the new signature.

No Dart production code, Kotlin code, generated bridge file, DB migration or
wire type changed.
