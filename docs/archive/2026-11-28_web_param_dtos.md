> **ARCHIVED** — historical record, not current behaviour.

# Web parameter DTOs moved out of the dispatcher (2026-11-28)

`src/web/methods.rs` had grown to ~1,380 lines because every JSON method's
argument struct lived inline with its dispatch arm. The 59 shapes and the
generic `parse` helper had nothing to do with routing, but they dominated the
file and buried the method arms.

## What shipped

- `src/web/params.rs` now owns all 59 camelCase parameter structs plus the
  shared `parse<T>` helper. The DTOs are plain `pub` inside the crate-private
  `params` module (`pub(crate)` on items there trips clippy's
  `redundant_pub_crate`); their wire shape is unchanged.
- `methods.rs` imports exactly the DTOs and `parse` it uses — no glob — and
  drops its now-unused `DeserializeOwned` import. It shrank from ~1,380 to
  ~900 lines, so the file reads as a router again.
- `src/web/mod.rs` registers the new `pub(crate) mod params;`.

No behaviour or wire-format change: the existing dispatch tests (including the
`invalid params:` assertions and the `dbPath` shape checks) still cover the
moved shapes. `cargo clippy --all-targets --features default,mobile,test-utils,calls -- -D warnings`
and all 27 `cargo test` suites pass.
