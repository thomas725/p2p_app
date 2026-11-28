> **ARCHIVED** — historical record, not current behaviour.

# TUI share-action extraction (2026-11-28)

`src/bin/tui/input_processor.rs` had grown to 2,872 lines, roughly 775 of them
share-specific handlers sitting between the Contacts helpers and the generic
dispatcher. That mixed share policy into a file already covering prompts, calls,
groups and chat actions, and the `handle_shares_tab_shortcut` helper was stranded
hundreds of lines away from the handlers it calls.

## What shipped

- Moved the whole share cluster into `src/bin/tui/share_actions.rs`: grant
  prompt/selection keys, manifest cursor and deletion actions, discovery and
  network actions, visibility toggles, share creation/binding/deletion/snapshot
  confirmations, `handle_shares_tab_keys` and `handle_shares_tab_shortcut`.
- The six functions called from `input_processor` (`confirm_create_share`,
  `confirm_share_binding`, `confirm_share_delete`, `confirm_share_grant`,
  `handle_shares_tab_keys`, `handle_shares_tab_shortcut`) are `pub(super)`;
  everything else stays private to the new module.
- `input_processor.rs` shrank to 2,088 lines and now imports only those six
  entry points. All key precedence, modal-editor ownership and command queueing
  are unchanged.

## Why this shape

Every moved handler still uses the same `AppState` view/model and canonical
`p2p_app::shares` facade — this is a file boundary, not a behaviour change.
Grouping the share cluster by feature makes each file easier to read without
introducing a new abstraction, and keeps the strict clippy thresholds satisfied
without lint allowances. The tests drive real `InputEvent`s through the
production `process_input_event` path, so they cover the wiring as well as the
handlers.

## Coverage

- `tests/unit/unit_bin_tui_input_processor.rs` (real key/mouse sessions) and the
  full Rust suite pass unchanged.
- Verified with `cargo clippy --all-targets --features default,mobile,test-utils,calls
  -- -D warnings` and `cargo test --features default,mobile,test-utils,calls`.
