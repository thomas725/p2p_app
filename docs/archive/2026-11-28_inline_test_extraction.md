> **ARCHIVED** — historical record, not current behaviour.

# Inline test module extraction (2026-11-28)

Two of the largest library source files still inlined their whole test suite:
`src/mobile_node.rs` (2,678 lines, 908 of them tests) and `src/web/mod.rs`
(1,824 lines, 1,304 of them tests). The repo already uses the
`#[cfg(test)] #[path = "../tests/unit/unit_*.rs"] mod tests;` pattern everywhere
else (db, contacts, relay, sort, …), so these two were the outliers.

## What shipped

- Moved the mobile-node test bodies verbatim to `tests/unit/unit_mobile_node.rs`
  and the web-server test bodies to `tests/unit/unit_web_mod.rs`. Both keep the
  standard test scaffolding header (the restriction set plus
  `used_underscore_binding`/`significant_drop_tightening`).
- Each source file now ends with the `#[path]` declaration. Production files
  shrank to 1,770 and 520 lines.
- No test was rewritten: the self-referential source guards read absolute paths
  via `env!("CARGO_MANIFEST_DIR")`, so relocation is transparent.

## Why this shape

The extraction is a pure file boundary — same `use super::*` access to private
items, same test names and counts (656 lib tests before and after). Keeping test
code next to its production code in one giant file made both harder to navigate
and pushed them to the top of the codebase-metrics table. Following the existing
`#[path]` convention needs no new abstraction and keeps clippy clean without
new lint allowances.

## Coverage

- Full `cargo test --features default,mobile,test-utils,calls` suite passes with
  identical per-binary test counts.
- Verified with `cargo clippy --all-targets --features default,mobile,test-utils,calls
  -- -D warnings`.
