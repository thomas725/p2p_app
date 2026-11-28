> **ARCHIVED** — historical record, not current behaviour.

# Dead FRB/web bindings removed (2026-11-28)

The identity work introduced a set of direct FRB/web bindings that no frontend ever
called: `provisionMyUser`, `verifyDeviceBindingClaim`, `addDevice`, `getContacts`,
`addContact`, and `removeContact`. They were deferred while FRB regeneration was
unreproducible; after `69dadcf` made codegen deterministic, the whole surface
could be removed in one reviewable slice.

## What shipped

- Dropped the six wrappers from `src/api/mod.rs` and their `#[frb(ignore)]`
  helpers from `src/mobile_api.rs`, plus the now-unused `MobileContactInfo` DTO
  and `contact_info_from` converter.
- Removed the matching web dispatch arms and parameter structs from
  `src/web/methods.rs`.
- Updated the FRB/web drift guards in `tests/unit/unit_web_mod.rs` and the
  identity docs to describe the live surface only.
- Regenerated the bridge with the documented protocol; the generated Dart/Rust
  diff removes only those six wire functions and renumbers the function IDs.

## Why the core functions stay

`identity::{provision_my_user, verify_device_binding_claim, add_contact,
remove_contact}` and `identity::list_contacts` remain Rust-internal domain
functions. Startup provisions each install, the swarm verifies claims at its
receive choke point, and both frontends read a single canonical roster through
`getContactsWithStats`. Deleting the domain layer would duplicate policy in
Dart; deleting only the bindings keeps one authority per concept.

## Coverage

- `cargo clippy --all-targets --features default,mobile,test-utils,calls -- -D
  warnings` clean.
- Full `cargo test --features default,mobile,test-utils,calls` passed (656 lib
  tests plus all integration binaries).
- `flutter analyze` clean; `flutter test` passed all 272 tests.
