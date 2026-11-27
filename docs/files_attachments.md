# File attachments over chat (files lane)

**Status: shipped.** Backlog §12 landed across four slices: `1a14818` (lane +
store + DB + swarm), `9ef9539` (TUI send/render), `daf4b66` (FRB + Flutter),
and the web mirror. Both frontends can send and receive file attachments on
DMs, broadcast, and groups.

This document is the lane's reference: the wire design, the store surface, and
the blob lifecycle. It also covers the versioned file-share/folder-sync
extension built on the same store. Its Rust core and both frontends' list,
detail, private-share grant and visibility editors have shipped, along with
Flutter/web discovery, network actions and deletion publishing; native
creation/binding/snapshot UI and TUI offer/manifest/publish actions stay open
in [`backlog.md`](backlog.md). The code is the authority.

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
  tab: `AppState::target_attachment_path` resolves `attachment_json` through
  `messages::`/`groups::attachment_json_for_msg_id` and `files::file_path`, then
  launches the OS default handler — the TUI equivalent of Flutter's tappable
  `_AttachmentChip`. It acts on the selected row (newest if none selected): with
  a message cursor armed only that row can open a file, while without one the
  aligned wire ids are scanned backwards for the newest row that actually
  carries a downloaded attachment. No wire or DB shape changed.

## Mobile (FRB) surface

- `MobileAttachment { fileId, name, size: i64, sizeLabel, chip, mime }` lives in `mobile_node.rs`;
  `sizeLabel` is `files::format_file_size(size)` computed Rust-side, so the Flutter chip prints
  the exact same label as the TUI and Dart never does byte math. `chip` is the canonical
  `[file] name (size)` string from `files::file_chip`, reused verbatim by the Flutter reply
  preview for attachment-only rows. `ChatMessage.attachment` and
  `MobileGroupMessage.attachment` added (regen once).
- Live broadcast/DM events carry the descriptor as `SwarmEventJson.attachment`.
  The frontends echo that Rust-built object into `saveIncomingMessage`; Rust converts it with
  `From<MobileAttachment> for files::MessageAttachment` before persistence. This is also the
  conversion used by `reply_draft`, so the live bubble, reply quote and reloaded row all use the
  same chip and size. `content` may be empty for an attachment-only row.
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
- Incoming events thread `event.attachment` through every `NodeApi` transport
  into `saveIncomingMessage`. `HomeScreen` handles broadcast and DM persistence;
  `DmChatScreen` handles its own DM save and the read-only broadcast context
  mirror. Web uses the same camelCase `attachment` object Rust emits on the
  event, so no Dart byte math or descriptor construction is involved.
- Tests: `test/widget/attachment_chip_test.dart` (chip render, tap opens,
  selection-mode tap is inert), live broadcast/DM attachment regressions in
  `messages_test.dart`, and `http_node_api_test.dart` event/save round-trip.
  Full Rust (clippy + available feature set) and Flutter (analyze + test)
  suites are green.

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
  content-addressed and skips symlinks. `accept_submitted_manifest` resolves the
  requester's ACL on the write transaction's connection and checks `base_revision`
  there too, so a grant revoked or a revision advanced between the check and the
  write is observed and denied; a writer whose base is stale refreshes and
  retries instead of silently overwriting another device's changes. Permission
  resolution shares the connection through `identity::find_user_id_for_peer_conn`
  and `shares::permission_for_share`, both fail-closed. A remote `ShareOffer` is
  untrusted: for a share we own, have bound, or already hold a manifest for,
  accepting a repeated offer is a no-op — it can never rewrite our ACL, root or
  revision — and a claimed owner id is stored only as a local `unknown:claim:`
  marker until Rust resolves the serving/publishing peer to that exact user.
  Both marker forms are ignored by owner ACL shortcuts, owner-only visibility
  toggles and re-advertising; `handle_share_manifest_response` and
  `accept_submitted_manifest` upgrade only an exact matching claim (the latter
  inside the submission transaction), so a spoofed claim never becomes a
  verified owner. Received manifests are monotonic: `save_manifest` rejects a lower
  revision (and an unknown share) so a replayed peer response cannot roll local
  state back; an equal revision is accepted only as an idempotent re-fetch — if
  cached JSON exists it must be byte-identical, otherwise the differing snapshot
  is rejected as a conflict rather than overwriting the cached one.
- **Bound folders and path safety**: a share may bind a real `local_root`.
  Applying a manifest writes only through `resolve_share_path`, rejecting
  absolute paths, `.`/`..` or empty segments, backslashes, colons, control
  characters, and any symlink component. A blob is re-hashed before it is
  installed via a temporary file, and only files tracked by the previous
  manifest are ever removed; untracked local files are left alone. A failed
  install removes its `.part` staging file so a synced folder never accumulates
  debris (blob GC only sweeps the blob store).
- **Deferred application**: `queue_apply` records a received manifest and the
  blob ids still missing; `note_blob_available(file_id, peer_id)` applies it when
  the last blob lands (attributed to that blob's delivering peer and emitting
  `ShareUpdated`), dropping it if the serving peer disappears or a newer
  revision won. A manifest whose blobs are all already local (a rename/delete or
  a repeated fetch) is applied immediately — no blob arrival will ever trigger
  it. The event fan-out is in `file_transfer.rs` and `swarm_handler.rs`, so
  folder convergence does not depend on a frontend.
- **Coverage**: `tests/swarm_handler.rs` drives the offer/list/manifest wire
  handshake and a stale submission followed by a successful retry;
  `tests/unit/unit_shares.rs` covers revocation (including ACL reads on the
  write connection and a denied submission after a committed revoke), same-
  revision conflicts with different content, stale writes with changed entries,
  superseded pending applies and disconnect cleanup. Unit coverage also pins
  spoofed owner claims: a claim cannot unlock the owner ACL, visibility toggles
  or publishing; only a matching Rust-resolved user upgrades the marker, and
  no-claim/locally owned rows are never rewritten. A real blob-pull → apply
  test drives two nodes on separate OS threads with per-thread DB and blob
  stores, covering blob delivery, manifest apply and folder contents.
- **Facade**: `src/api/mod.rs` exposes the share store as `createShare`,
  `listShares`, `getShare`, `bindShare`, `deleteShare`, `listShareGrants`,
  `setShareGrant`, `removeShareGrant`, `loadShareManifest`, `saveShareManifest`,
  `acceptShareOffer`, `snapshotShare`, `requestPeerShares`,
  `requestShareManifest`, `readvertiseShare` and `publishShareSnapshot`.
  Re-advertising is Rust-owned: the binding takes only share and peer IDs and
  builds the canonical offer through `shares::offer_for_share`; the raw
  offer-send binding is gone.
  `src/web/methods.rs` mirrors all of them: `createShare`/`snapshotShare` reject
  client-supplied server paths, `bindShare` is rejected outright (a browser has
  no folder to bind, and a null root could clear a native binding), and
  `saveShareManifest` is native-only because it skips ACLs and `base_revision`
  checks — web clients use `publishShareSnapshot` instead.
  The domain types keep snake_case for the wire protocol and stored JSON; the
  browser and mobile event boundaries convert explicitly through
  `src/shares_json.rs` (camelCase) so a field-name mismatch cannot silently
  default every value. The generated Dart bindings live in
  `lib/src/rust/shares.dart`; the browser's matching DTO conversion lives in
  `lib/src/data/shares_json.dart`, and both transports expose the same methods
  through `NodeApi`.
- **Frontend surface**: share events are serialized in `mobile_node.rs` as
  `share_offered`, `share_updated` and `share_rejected`. Flutter handles them by
  reloading the share list (`lib/src/screens/share_list.dart`, opened from
  Settings → File shares); the browser gets the same route through the JSON
  transport because both use the existing `NodeApi` facade. The list's app bar
  opens `lib/src/screens/share_discovery.dart`, which asks a connected peer for
  its offers and explicitly accepts one through `acceptShareOffer` — an offer
  is untrusted until that step, and the advertised owner is shown as
  unverified. The detail page can add/revoke private-share grants through
  `setShareGrant`/`removeShareGrant` (optional callbacks keep it read-only in
  tests); public shares have no ACL rows. `lib/src/screens/share_network.dart`
  adds manifest refresh and offer re-advertise from a chosen connected peer;
  re-advertise passes only share and peer IDs and Rust rebuilds the offer so the
  local unknown-owner sentinel is never advertised.
  `lib/src/screens/share_publish.dart` submits a deletion
  revision through `publishShareSnapshot` with the loaded manifest revision as
  `baseRevision`; stale manifests/selections are blocked before the call, and a
  caller still cannot bypass the owner's ACL check. The TUI mirrors the
  list/detail flow and adds a private-share grant editor: `a` adds or upgrades
  a grant, `x` revokes the highlighted grant, and `Ctrl+Up`/`Ctrl+Down` moves
  the grant cursor. `Enter` commits through `shares::set_grant` /
  `shares::remove_grant` directly, so no FRB/web method changed; `Esc` leaves
  the persisted grants untouched. `p` toggles private/public and `w` toggles
  anonymous writes through `shares::set_visibility`, while Flutter's owner-only
  detail controls use the same setter via FRB/web `setShareVisibility`. Share
  events and editor commits reload the canonical store and leave a visible
  notice. The TUI detail pane also lists connected peers in canonical peer-ID
  order: `Left`/`Right` selects one, `m` queues `RequestShareManifest`, and `o`
  rebuilds the offer through `shares::offer_for_share` before queueing
  `SendShareOffer`. Selection is clamped on connect/disconnect, and modal
  editors keep ownership of all keys. The TUI detail pane also marks manifest
  files with `d` (`Ctrl+Left`/`Ctrl+Right` moves the file cursor) and submits the
  marked deletions with `Ctrl+D`: Rust rebuilds the next revision from the
  canonical store via `shares::manifest_with_deletions`, using the displayed
  manifest revision as the base and rejecting stale or moved snapshots before a
  command is queued. Native creation/binding/folder snapshots and share
  discovery remain deferred in the TUI.

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
