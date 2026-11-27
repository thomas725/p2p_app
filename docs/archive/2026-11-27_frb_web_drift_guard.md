> **ARCHIVED** — historical record, not current behaviour.

# Exhaustive FRB↔web dispatch drift guard (2026-11-27)

The web server mirrors the FRB surface one-for-one as JSON methods. That rule
used to be enforced by hand-written lists: a no-param source-string list in
`src/web/mod.rs`, plus per-domain `dispatch(name, Null)` tests for methods whose
parameters can be parsed before any side effect. A newly FRB-exposed function
could therefore be added without a web arm and still pass CI until a browser
called the missing name.

`src/web/mod.rs` now derives the expected method set from the generated bridge
source. The test parses every `wire__crate__…_impl` function in
`src/frb_generated.rs`, converts its `snake_case` name to the Dart `camelCase`
method name and asserts that the production part of `src/web/methods.rs`
contains the quoted arm. Only four wires are exempt, each with a documented
reason in the test:

- `initMobileDatabase`, `pollEvent` and `getNodePeerId` are Flutter bootstrap
  and transport seams; the in-process server owns node startup and pushes
  events over WebSocket instead of polling;
- `attachmentFilePath` is a native filesystem path for the Dart file opener;
  web clients fetch the same bytes from `GET /api/files/{file_id}`.

A synthetic parser test covers the module-segment and `snake_case` conversion,
and the live guard asserts that a sentinel method was parsed so a codegen
format change cannot silently turn the check into a no-op. The existing
no-param list stays because it documents which arms cannot be probed safely,
and the parameterised arms keep their `invalid params:` dispatch tests.

No runtime behaviour, wire shape or frontend code changed.

## Validation

`cargo fmt --all`, strict clippy with `default,test-utils,calls`, the full test
suite, `python3 scripts/check_docs_links.py`, `git diff --check` and regenerated
`docs/codebase_metrics.md` all pass.
