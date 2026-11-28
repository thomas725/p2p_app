> **ARCHIVED** — historical record, not current behaviour.

# Unused low-level location facade methods removed (2026-11-28)

The location-sharing feature shipped with a complete Flutter UI: a share dialog,
live sessions, message map links, a device-scoped track screen with Rust time
filters and GPX export. Three low-level `NodeApi` methods were exposed as
facade primitives in the same change but never wired to a screen or test; the
UI uses higher-level methods whose Rust implementations do the same work.

## What shipped

- `locationMapUrl(latitude, longitude)` — no caller. The map link in a message
  bubble uses `locationMessageUrl`, which runs a full canonical parse rather
  than trusting hand-written coordinates.
- `isLocationMessage(content)` — no caller. It is only a prefix check; the
  strict `locationMessageUrl` parse is the one the UI relies on.
- `parseLocationTimeFilter(text, now)` — no caller. The track screen passes raw
  filter text to `loadLocationTrack`, where `messages::load_location_track`
  parses it through the same canonical Rust grammar.

All three were removed from `api/mod.rs`, the `NodeApi` interface, both Dart
transports, the test fake's override seams, the web dispatcher and its two DTOs
that lost their only consumer (`IsLocationMessageParams` stays: `locationMessageUrl`
still uses it). Generated bridge files were refreshed in one coordinated FRB
regeneration. The canonical `location.rs` functions remain — nothing about the
location feature itself changed.

Git history confirms the methods were added as facade primitives and never
called (`2c662e6`, `d046e48`); this is API cleanup, not a parity gap. The TUI
still has no location-*sending* side, which is an independent, deliberate
limitation.

## Verification

No-change FRB regeneration was confirmed byte-clean first. Then:
`cargo clippy --all-targets --features default,mobile,test-utils,calls -- -D warnings`,
all 27 `cargo test` suites, `dart format`, `flutter analyze`, all 292 Flutter
tests and `flutter build web --no-wasm-dry-run` all pass.
