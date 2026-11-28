> **ARCHIVED** — historical record, not current behaviour.

# Shared message DTO timestamps (2026-11-28)

`message_to_chat` (DMs/broadcasts) and `group_message_to_mobile` built their two
display strings independently: the `sent_at` Unix-seconds value to a local
`HH:MM`, and the DB `created_at` to a local `%Y-%m-%d %H:%M:%S`. Both blocks were
byte-for-byte copies, so a future change to one format (or a `ToPrimitive`
edge case) could silently diverge the two chat surfaces.

## What shipped

- Added `mobile_node::message_timestamps`, one helper returning
  `(Option<String>, String)` for `sent_at`/`created_at`. Both mappers now call
  it, so Flutter and web get identical strings for DMs, broadcasts and group
  messages.
- Added a parity regression that feeds identical timestamps through both
  mappers and asserts the resulting `sent_at` and `created_at` are equal.

## Why this shape

The duplication was pure formatting policy, not data: both DTOs read the same
generated Diesel model fields and both frontends render the strings verbatim.
Keeping one Rust helper means the display authority stays Rust-side (matching
the seen-timestamp convention) with no new FRB binding, web method, wire field
or database column. The `to_i64().unwrap_or_default()` truncation semantics are
preserved exactly.

## Coverage

- `mobile_node::tests::test_group_message_to_mobile_uses_message_timestamps`
  covers the group mapper's use of the shared helper and the cross-DTO equality.
- Existing `test_message_to_chat_*` cases still cover send-time presence,
  attachment-only bodies and `None` send times.
- Verified with `cargo clippy --all-targets --features default,mobile,test-utils,calls
  -- -D warnings` and the full `cargo test --features default,mobile,test-utils,calls`
  suite (ALSA-dependent `tui-audio` is unavailable locally; CI installs
  `libasound2-dev`).
