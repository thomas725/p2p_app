> **ARCHIVED** — historical record, not current behaviour.

# Shared integration-test DB helper (2026-11-27)

`tests/db.rs`, `tests/messages.rs` and `tests/nickname.rs` each carried an
identical `test_db_lock` + `with_test_db` pair. `tests/test_utils.rs` already
provided the same helper (temp dir → thread-local DB URL → migrations →
closure → release/reset), and `tests/behavior.rs` / `tests/peers.rs` already
used it.

## What shipped

- Replaced the three local copies with `mod test_utils;` and imports of
  `test_utils::{test_db_lock, with_test_db}` (or just `with_test_db`).
- Documented the invariant in `AGENTS.md`: integration tests must not copy the
  helper, and these tests need the `cargo t` alias (`test --all-features`).
- Regenerated `docs/codebase_metrics.md`.

## Why this shape

Each top-level file under `tests/` is its own integration-test crate. Including
`test_utils.rs` as a module gives each crate its own static lock, exactly as the
deleted local definitions did; the thread-local DB URL still isolates tests on
separate threads. Tests that deliberately manage `set_db_url` / `reset_db_url`
themselves keep their direct lock guards because the URL reset is the behavior
under test.

Plain `cargo test` compiles the library without the `test-utils` feature, so
`reset_db_url` takes the production branch and the DB-reset assertions fail by
design. `cargo t` is the project test command; this is now recorded so a future
targeted run is not mistaken for a regression.
