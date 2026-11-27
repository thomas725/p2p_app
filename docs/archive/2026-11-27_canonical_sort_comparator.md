> **ARCHIVED** — historical record, not current behaviour.

# Canonical five-column sort comparator (2026-11-27)

The Contacts/peers tables in the TUI and both Flutter transports were ordering
the same five columns with three separate copies of the same comparator:
`contacts::sort_canonical_rows` (live TUI path), `mobile_api::sort_contacts`
(live Flutter Contacts path) and `mobile_api::sort_peers` (FRB/web surface).
Copying invited drift; the legacy TUI copy already used a different fallback
column for an unknown sort key.

## What shipped

- `src/sort.rs` owns the rule. A `SortableRow` trait exposes only the values the
  ordering needs (name, stable id, two seen strings, two counts), and
  `sort_rows_by_column` implements it once: lowercase name for column 0,
  DM count, broadcasts sent, last seen, first seen, with the stable id as the
  tie-break. Any unknown column falls back to the name. Seen columns go through
  the canonical `fmt::parse_last_seen_ms` (0 on unparseable input), and a
  descending sort reverses the **whole** ordering, tie-break included.
- The three row types implement the trait as thin adapters, and their public
  functions now delegate to the shared comparator. No FRB/web/Dart signature,
  field, or wire shape changed; `sort_peers` keeps its place in the dispatch
  table and generated bindings.
- Five unit tests pin the fallback column, case-insensitive name ties, count
  ordering, canonical timestamp parsing, and whole-ordering reversal. The
  existing contacts/mobile/TUI sort tests continue to pass unchanged.

## Why this design

This was a behaviour-preserving deduplication, not a new abstraction for its own
sake: the three copies had identical semantics on every reachable input and
differed only on an unreachable fallback branch. Keeping the ordering rule in
one Rust module means a future column or parser change cannot land in one
frontend only. The remaining `tui_helpers::sort_peers_table` and
`sort_peers_by_column` are test-only legacy helpers for the old peer-table path;
they are deliberately left in place because the `VecDeque` wrap regression test
still guards the "never sort a sliced ring" invariant.

Commit: to be linked by git history; no hash is invented in this record.
