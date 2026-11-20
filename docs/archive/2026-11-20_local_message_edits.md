# Local-only sent-message edits (2026-11-20)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

This closes the edit half of **US-005**. A user can replace the body of a
message they sent from broadcast chat, a DM, or a group chat; the edit is
visible only on this device. Nothing is sent to peers — they keep their
original copy. Local deletion shipped separately on 2026-11-17
([deletion archive](2026-11-17_local_message_deletion.md)).

## Shape of the slice

- `migrations/2026-11-20-000000_local_message_edits` adds a nullable
  `edited_at DOUBLE` to `messages` and `group_messages`. `NULL` means unchanged;
  a Unix timestamp records the last local edit. Repeat edits refresh the marker.
  The down migration is a no-op because the columns are additive and nullable.
- `messages::edit_message(id, body)` and
  `groups::edit_group_message(id, body)` replace the body only when
  `id = ? AND sent = 1 AND deleted_at IS NULL`, so incoming rows and local
  tombstones are never editable. They return `true` iff a row changed.
- `api::edit_message(message_id, is_group, content)` is the single shared
  surface. It rejects blank bodies and returns an error when no editable sent
  row exists. The web JSON mirror adds the `editMessage` arm with
  `{messageId, isGroup, content}` params; no swarm command or wire type changes.
- `ChatMessage.edited` and `MobileGroupMessage.edited` are filled from
  `edited_at` on history loads (like `confirmed` and `latency`, they are display
  decorations, not wire fields). Every frontend renders the canonical
  `(edited)` marker from `fmt::EDITED_MARKER` next to the new body; a reloaded
  row keeps the marker because the body and timestamp are both persisted.

## Frontend bindings

- **TUI**: `e` in Chat/DM/Group opens the single-slot editor pre-filled with
  the newest sent message's full body; Enter saves, Esc cancels. State holds
  `EditMessagePrompt { message_id, is_group, tab_content }` and
  `InputMode::EditingMessage`. `reply::with_body` preserves the timestamp,
  sender tag and quoted header when the in-memory line is replaced.
- **Flutter/web**: selecting one sent bubble shows a local Edit action in the
  selection toolbar. The dialog explains that only this device changes, calls
  `NodeApi.editMessage`, and updates the row through `ChatMessageExt.withEditedBody`.
  DM, group and broadcast screens share the same mixin/body path, so all three
  keep the marker and no screen persists locally.

No edit is ever retried or queued: unlike the sealed DM outbox
([outbox archive](2026-11-19_store_and_forward_dm_outbox.md)), this is a
presentation-layer correction on the sender's own history.
