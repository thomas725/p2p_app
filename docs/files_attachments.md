# File attachments over chat (files lane)

**Status: shipped.** Backlog §12 landed across four slices: `1a14818` (lane +
store + DB + swarm), `9ef9539` (TUI send/render), `daf4b66` (FRB + Flutter),
and the web mirror. Both frontends can send and receive file attachments on
DMs, broadcast, and groups.

This document is the lane's reference: the wire design, the store surface, and
the blob lifecycle. It also covers the versioned file-share/folder-sync
extension built on the same store; its Rust core has shipped, while the
TUI/Flutter/web UI is still open work in [`backlog.md`](backlog.md). The code is
the authority.

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
  already logged the landed path. Opening a received file is `Ctrl+O` in any chat
  tab: `AppState::newest_attachment_path` walks the aligned wire ids newest-first,
  resolves `attachment_json` through `messages::`/`groups::attachment_json_for_msg_id`
  and `files::file_path`, then launches the OS default handler — the TUI equivalent
  of Flutter's tappable `_AttachmentChip`.

## Mobile (FRB) surface

- `MobileAttachment { fileId, name, size: i64, sizeLabel, chip, mime }` lives in `mobile_node.rs`;
  `sizeLabel` is `files::format_file_size(size)` computed Rust-side, so the Flutter chip prints
  the exact same label as the TUI and Dart never does byte math. `chip` is the canonical
  `[file] name (size)` string from `files::file_chip`, reused verbatim by the Flutter reply
  preview for attachment-only rows. `ChatMessage.attachment` and
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

## Versioned file shares / folder sync (share lane) — 2026-11-22

The share lane layers a versioned tree on top of the attachment store. Single
files still move over the files lane; the share lane only negotiates *what* to
fetch and *who may change it*. Its Rust core is shipped; UI work is tracked in
[`backlog.md`](backlog.md).

- **Protocol**: `shares` `request_response` behaviour, protocol name
  `/p2p-chat/shares/1.0.0`, serde JSON `ShareRequest`/`ShareResponse`. Request
  kinds are `offer`, `list`, `manifest` and `publish`; responses carry offers, a
  manifest, or a rejection reason. The lane is blocklist-gated like every other
  `request_response` lane, and the requester is always the connection's peer id
  — a claimed user id in the payload is ignored.
- **Model**: migration `2026-11-22-000002_file_shares` adds `shares`,
  `share_grants` and `share_entries`. A `ShareManifest` names each file by a
  validated relative path plus its content address; `ShareKind` is private or
  public, and `ShareMode` is read-only or read-write. Public shares are readable
  by any connected peer and write is opt-in via `public_write`; private grants
  are keyed by verified `user_id` from `identity::find_user_id_for_peer`.
  `can_read`/`can_write` fail closed on DB errors and unbound peers.
- **Snapshots and writes**: `shares::snapshot_tree` stores every regular file
  content-addressed and skips symlinks. `accept_submitted_manifest` rejects a
  submission whose `base_revision` is no longer current, so a writer refreshes
  and retries instead of silently overwriting another device's changes.
- **Bound folders and path safety**: a share may bind a real `local_root`.
  Applying a manifest writes only through `resolve_share_path`, rejecting
  absolute paths, `.`/`..` or empty segments, backslashes, colons, control
  characters, and any symlink component. A blob is re-hashed before it is
  installed via a temporary file, and only files tracked by the previous
  manifest are ever removed; untracked local files are left alone.
- **Deferred application**: `queue_apply` records a received manifest and the
  blob ids still missing; `note_blob_available` applies it when the last blob
  lands (and emits `ShareUpdated`), dropping it if the serving peer disappears
  or a newer revision won. The event fan-out is in `file_transfer.rs` and
  `swarm_handler.rs`, so folder convergence does not depend on a frontend.
- **Coverage**: `tests/swarm_handler.rs` drives the offer/list/manifest wire
  handshake and a stale submission followed by a successful retry;
  `tests/unit/unit_shares.rs` covers revocation, stale writes with changed
  entries, superseded pending applies and disconnect cleanup. A real blob-pull
  → apply test with separate per-node databases is still open.
- **Frontend surface (deferred)**: share events are serialized in
  `mobile_node.rs` as `share_offered`, `share_updated` and `share_rejected`
  for a later UI slice; the TUI currently logs them. No FRB methods or web
  methods exist yet.

## Blob lifecycle / GC (`attachment-gc` feature) — 2026-10-04

Blob cleanup is real and behind an on/off switch: the `attachment-gc` Cargo
feature (part of `default` and `mobile`) is the enable/disable knob — disabled,
`file_transfer::gc_stale_attachments()` is a `Ok(GcStats::default())` no-op and
the store never deletes anything.

- `files::gc(active, grace)` sweeps two classes: **incomplete** `.part` partials
  of interrupted pulls (no transfer in flight for them) and **overwritten /
  orphaned** full blobs no live message references (superseded after DB reset /
  message loss), blobs detached from their `files` row, and `files` rows whose
  blob is missing on disk.
- `files::referenced_file_ids()` also unions share-entry blobs through
  `shares::referenced_file_ids()`, so a live manifest's bytes are never swept as
  orphans. A share-DB error is conservative: the GC pass aborts instead of
  deleting anything.
- Runs at startup in both frontends (TUI `main_loop.rs`, mobile
  `start_node_impl`). In-flight transfers are excluded by the active `file_id`
  set; blobs younger than `files::GC_GRACE` (1 min) are skipped.
- A transfer interrupted mid-run keeps its `.part` until the next restart clears
  the in-flight map; the restart sweep then removes it as stale. Independently
  of GC, starting a fresh receive for the same content address discards any
  leftover `.part` in `ReceivingFile::new` — so a crash plus a fast restart
  (inside `GC_GRACE`) cannot append new chunks after stale bytes and fail the
  final hash check.

## Open follow-ups

None tracked here — open work lives in [`backlog.md`](backlog.md).
