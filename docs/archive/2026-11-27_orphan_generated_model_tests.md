> **ARCHIVED** — historical record, not current behaviour.

# Orphaned generated-model tests (2026-11-27)

`tests/models/insertable_tests.rs` and `tests/models/queryable_tests.rs` were
not Cargo integration-test targets: files nested below `tests/` are only
compiled when another top-level target includes them as modules, and no
`mod models` reference existed anywhere in the codebase. They also imported
`crate::generated::*`, which would not resolve from a standalone integration
test crate. The only remaining references were a historical prompt log and
the description table in `scripts/generate_metrics.py`.

## What shipped

- Removed both dead files (and the now-empty `tests/models/` directory).
- Removed their stale entries from the metrics generator so regenerated docs
  no longer report tests that never ran.
- Regenerated `docs/codebase_metrics.md`.

No runtime or test behavior changed: these files had never contributed to a
`cargo test` run. The live generated-model assertions remain in the compiled
unit/integration suites.
