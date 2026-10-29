# File attachments over chat (files lane)

**Status: shipped.** Backlog §12 landed across four slices: `1a14818` (lane +
store + DB + swarm), `9ef9539` (TUI send/render), `daf4b66` (FRB + Flutter),
and the web mirror. Both frontends can send and receive file attachments on
DMs, broadcast, and groups.

This document is the lane's reference: the wire design, the store surface, and
the blob lifecycle. The code is the authority; the open polish item is listed
in [`backlog.md`](backlog.md).

## Wire design

- **Protocol**: `files` `request-response` behaviour, protocol name
  `/p2p-chat/files/1.0.0`, cbor `FileTransfer`/`FileTransferResponse` with a
  plain-struct `kind` discriminator `meta`/`chunk`/`error`. Receiver-driven pull:
  the sender advertises a descriptor on the message, the receiver requests it.
  `FILE_CHUNK_SIZE` = 1 MiB, `MAX_ATTACHMENT_SIZE` = 64 MiB.
- **Content addressing**: `file_id` = SHA-256 of the bytes (lowercase hex) from
  `src/files.rs`. Blobs live on disk under `<db>.files`; `files` (id, size, mime,
  path) and `files_recipients` (per-DM delivery) tables mean SQLite never holds
  blobs. First-seen-wins dedup on the content address.
- **Group/broadcast attachments are by reference only**: the message carries the
  descriptor (small), the file is fetched over the `files` request-response lane
  — so gossipsub's 256 KiB `max_transmit_size` cap is never hit.
- **Back-compat**: message `attachment_json` descriptor columns use
  serde-default; migrations use distinct `...000000`/`...000001` suffixes.
- **No feature gate** — the lane ships in the default build (mirrors the
  un-gated call-signaling precedent).

## Store + message surface

- `src/files.rs` stores, `MessageAttachment` descriptor (`files::file_chip` →
  `[file] {name} ({size})` with `{size}` from `files::format_file_size`:
  `512 B` / `1.5 KiB` / `64 MiB`), `render_message_body(content, Option<&MessageAttachment>)`
  (attachment-only messages render as just the chip) and `render_stored_body(content,
  Option<&str>)` to hydrate DB `attachment_json` on reloads.
- TUI: typed `/file <path>` → `parse_file_command` → `send_file_message` /
  `send_message(state, tx, text, attachment, tab_content)`; DB-reload chips via
  `render_stored_body` in `click_handlers.rs` and `state.rs`; `FileReceived` event
  already logged the landed path.

## Mobile (FRB) surface

- `MobileAttachment { fileId, name, size: i64, sizeLabel, mime }` lives in `mobile_node.rs`;
  `sizeLabel` is `files::format_file_size(size)` computed Rust-side, so the Flutter chip prints
  the exact same label as the TUI and Dart never does byte math. `ChatMessage.attachment` and
  `MobileGroupMessage.attachment` added (regen once).
- `store_send_attachment(Option<String>)`, `save_outgoing_broadcast|dm|group`
  take `attachment_path: Option<String>`; `attachment_file_path(file_id: &str)` →
  content-addressed blob path. `api/mod.rs` wrappers mirror them.

## Flutter frontend

- `lib/src/util/attachments.dart`: `pickFileOverride` / `openFileOverride`
  seams (no `@visibleForTesting` — used cross-library) wrapping `file_picker`
  13.x (`FilePicker.pickFile()` — top-level, the `.platform` member was removed)
  and `open_filex` (`ResultType.done` / `result.message`).
- `messages.dart`: `MessageInputBar.onPickFile` attach button,
  `MessageBubble` `_AttachmentChip` (tappable unless selecting; renders
  `attachment.sizeLabel`), `MessageChatBody` / `BroadcastChat` thread
  `onOpenAttachment`. Broadcast/DM/Group screens pass pick-and-send
  (`content: ''` + path) and open-with-snackbar handlers.
- Tests: `test/widget/attachment_chip_test.dart` (chip render, tap opens,
  selection-mode tap is inert); full Rust (clippy + `--all-features`) and
  Flutter (analyze + test) suites green.

## Blob lifecycle / GC (`attachment-gc` feature) — 2026-10-04

Blob cleanup is real and behind an on/off switch: the `attachment-gc` Cargo
feature (part of `default` and `mobile`) is the enable/disable knob — disabled,
`swarm_handler::gc_stale_attachments()` is a `Ok(GcStats::default())` no-op and
the store never deletes anything.

- `files::gc(active, grace)` sweeps two classes: **incomplete** `.part` partials
  of interrupted pulls (no transfer in flight for them) and **overwritten /
  orphaned** full blobs no live message references (superseded after DB reset /
  message loss), blobs detached from their `files` row, and `files` rows whose
  blob is missing on disk.
- Runs at startup in both frontends (TUI `main_loop.rs`, mobile
  `start_node_impl`). In-flight transfers are excluded by the active `file_id`
  set; blobs younger than `files::GC_GRACE` (1 min) are skipped.
- A transfer interrupted mid-run keeps its `.part` until the next restart clears
  the in-flight map; the restart sweep then removes it as stale.

## Open follow-ups

None tracked here — open work lives in [`backlog.md`](backlog.md).
