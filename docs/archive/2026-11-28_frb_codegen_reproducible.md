> **ARCHIVED** — historical record, not current behaviour.

# FRB codegen made reproducible (2026-11-28)

`1bf19f4` introduced the canonical five-column `SortableRow` trait and
implemented it for the two FRB-exposed input structs (`PeerSortInput`,
`ContactSortInput`) in `src/mobile_api.rs` without regenerating the bridge.
Because flutter_rust_bridge 2.13 scans trait impls on exposed types, the next
no-change regeneration emitted six getter wire functions per struct — roughly
1,390 insertions and 312 deletions across `src/frb_generated.rs` and the
generated Dart files. That diff was unrelated boilerplate and made any future
FRB change hard to review.

## What shipped

- Added `#[flutter_rust_bridge::frb(ignore)]` to both internal `SortableRow`
  impls. They are Rust-only adapters for the canonical comparator, never part
  of the FRB surface.
- Regenerated with `flutter_rust_bridge_codegen generate`, restored the
  mandated clippy allows on `src/frb_generated.rs`, ran `cargo fmt --all`, and
  removed the untracked `src/frb_generated.h`.
- The resulting generated diff is two comment lines: Dart's ignored-function
  inventory now lists the twelve ignored trait getters, and `mobile_node.dart`
  picks up the `message_timestamps` helper that `7dfc3b1` had left stale. No
  Rust bridge body changed.

## Why this shape

An `#[frb(ignore)]` on the impl block suppresses FRB's trait-method getters
without touching the structs or the comparator, so the FRB surface stays
exactly what the Dart call sites use. A second regeneration produced the
identical diff before post-processing, which confirms the bridge is now
reproducible: the only recurring delta is the documented allow-list rewrite
that the regeneration protocol always re-applies.

## Coverage

- `cargo clippy --all-targets --features default,mobile,test-utils,calls -- -D
  warnings` clean.
- Full `cargo test --features default,mobile,test-utils,calls` passed (656 lib
  tests plus all integration binaries, unchanged counts).
- `flutter analyze` reported no issues; `flutter test` passed all 272 tests.
