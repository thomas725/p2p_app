# Human-readable attachment sizes (2026-10-26)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

The last open P2 polish item of the files lane: the chat chip printed the raw
byte count (`[file] report.pdf (1048576)`). It now prints a binary-unit label
(`[file] report.pdf (1 MiB)`) in both frontends, from one Rust formatter.

## The rule

`files::format_file_size(bytes: u64) -> String`:

- binary units (`1 KiB = 1024 B`), `B` / `KiB` / `MiB` / `GiB` / `TiB`;
- at most one decimal place, truncated to tenths first then rounded to nearest;
- a trailing `.0` is dropped (`1 KiB`, not `1.0 KiB`);
- values that round up to `1024 <unit>` step up a unit instead
  (`1024 * 1024 - 1` → `1 MiB`), and `u64::MAX` caps at `16777216 TiB`.

Arithmetic is `saturating_*` / `checked_*` only — the crate denies
`arithmetic_side_effects` and `indexing_slicing`, so the scaling runs in `u128`
with `checked_div(...).unwrap_or(0)` and the unit name comes from
`UNITS.get(unit)`.

## One formatter, two frontends

- **TUI**: `files::file_chip` composes the label, so chat rows, the sent-message
  echo, and DB-reloaded rows (`render_stored_body`) all agree.
- **Flutter**: `MobileAttachment` gained a `sizeLabel: String` field filled
  Rust-side by `mobile_node::mobile_attachment` from the same function;
  `_AttachmentChip` prints `attachment.sizeLabel`. No new FRB function and no
  transport plumbing — the label rides along with the descriptor the message
  already carries, so the web client gets it for free.
- Dart does no byte math, which is the repo rule for every shared formatter
  (timestamps, sort keys, sizes).

## Verification

- `tests/unit/unit_files.rs`: `format_file_size_uses_binary_units_with_one_decimal`
  pins the boundaries (`0 B`, `1023 B`, `1 KiB`, `1.5 KiB`, `64 MiB`, the
  unit-step case, and `u64::MAX`); the chip/body tests now expect the labels.
- `tests/unit/unit_bin_tui_click_handlers.rs`: the DB-reload chip test asserts
  `(2 KiB)`.
- Flutter: `attachment_chip_test.dart` + `attachments_test.dart` +
  `http_node_api_test.dart` carry `sizeLabel: '2 KiB'`; `flutter analyze` and
  `flutter test` green, `cargo ct` and `cargo test --all-features` green.
- FRB regenerated (2.13.0) for the new model field; clippy allow block
  re-appended, `cargo fmt --all`, `src/frb_generated.h` removed.

Docs updated in the same commit: `AGENTS.md` canonical-chip rule,
[`../files_attachments.md`](../files_attachments.md), and
[`../../KEYBOARD_SHORTCUTS.md`](../../KEYBOARD_SHORTCUTS.md).
