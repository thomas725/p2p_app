> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web share creation (2026-11-27)

The TUI's Shares tab could create an empty private share with `c`, but the
shared Flutter/web UI could only list shares and accept peer offers. Creation
needed no local path — `api::create_share`, the web `createShare` arm and both
`NodeApi` transports already existed and were tested — so this slice closes the
parity gap without touching Rust or regenerating the bridge.

## What shipped

- `ShareList` is now stateful and renders a keyed "Create private share"
  action when a `onCreateShare` callback is supplied. Read-only embeds omit the
  button, mirroring the other optional seams in this widget.
- A small name dialog mirrors the existing grant dialog: it trims the input,
  refuses an empty name, disables the submit button while the call is in flight
  and keeps the dialog open with an inline error when Rust rejects the name.
- Home wires the callback to `NodeApi.createShare` with the same parameters the
  TUI commits: `kind = private`, `publicWrite = false` and no local root. It
  reloads the canonical share list after success, so both transports land back
  on the list with the new row. Name validation, owner resolution and the share
  row itself stay in Rust; Dart only moves the trimmed string across the
  existing facade.
- The empty-list copy, Settings help and share-list doc comment now describe
  creation as available in both frontends. Binding a folder and taking a native
  folder snapshot remain native-only actions.

## Tests

Four widget tests cover the button's presence, submission of the trimmed name,
the empty-name no-op and inline failure handling; the existing empty-state
assertion was updated for the new copy. `flutter analyze` and the full widget
suite pass. No new Rust or web test was needed: the rootless create contract is
already covered by the facade unit test and the web JSON transport test.

## Why this design

The action is a thin UI affordance over an existing canonical binding. Adding a
local-root field here would have created a second path that the browser must
reject anyway, so creation deliberately stays rootless and folder binding stays
on the native install. Keeping the callback optional preserves the pure-widget
tests and the read-only embeds.

Commit: to be linked by git history; no hash is invented in this record.
