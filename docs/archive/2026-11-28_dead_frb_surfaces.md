> **ARCHIVED** — historical record, not current behaviour.

# Uncalled FRB surfaces removed (2026-11-28)

An audit of every FRB wire function against all three frontends found four
surfaces with no application callers — only generated bridge code and their own
Rust unit tests. All were removed in one coordinated FRB regeneration so the
generated Dart and Rust bridge files no longer carry dead codecs, wire arms or
Dart wrappers.

## What shipped

- `mobile_api::parse_last_seen_ms` — the FRB wrapper around the canonical
  parser had no Dart or web caller. Removed; Rust code calls
  `fmt::parse_last_seen_ms` directly, which remains the single parser for the
  TUI, Flutter (`sort_contacts`) and web sort paths.
- `mobile_api::sort_peers` + `PeerSortInput` — the peer-list screen they served
  was replaced by the Contacts tab, which sorts through `sort_contacts` /
  `ContactSortInput`. Removed along with their unit tests; `contact_list_test.dart`'s
  `_sortLikeRust` fake is the remaining sort-parity guard.
- `mobile_node::get_node_peer_id` — no frontend caller (the web drift guard
  listed it as a native-only exemption, and the TUI reads its ID from the core).
  Removed, with its exemption and probe test.
- `mobile_node::get_reactions` and `mobile_node::delete_group` — the frontends
  call the `api::` wrappers that delegate to these, so the functions stay but
  now carry `#[frb(ignore)]`. The generated-only duplicate bindings are gone.

The web dispatch arms and `ParseLastSeenParams` / `SortPeersParams` DTOs went
with their Rust functions; the drift guard's parser-test examples were swapped
to still-live methods. `AGENTS.md` rules that named the removed surfaces were
corrected, including the stale `peer_list_test.dart` reference.

## Verification

A no-change `flutter_rust_bridge_codegen generate` was confirmed byte-clean
first (re-appending the clippy allow-list, `cargo fmt`, deleting the stray
`.h`). Then: `cargo clippy --all-targets --features default,mobile,test-utils,calls -- -D warnings`,
all 27 `cargo test` suites, `dart format`, `flutter analyze`, all 292 Flutter
tests and `flutter build web --no-wasm-dry-run` all pass.
