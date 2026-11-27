> **ARCHIVED** — historical record, not current behaviour.

# Incoming attachment descriptors on Flutter/web (2026-11-27)

The files lane already put a `MessageAttachment` descriptor on every live
broadcast/DM `MessageEvent`, but the Flutter/web event envelope dropped it. A
received file therefore appeared in the live bubble only when the blob had
already been fetched; persisting the event wrote an empty body and reloading the
chat lost the chip entirely. TUI did not have this gap because it reads the
descriptor from the same core event.

## What shipped

- `SwarmEventJson` gained `attachment: Option<MobileAttachment>`. Only
  `message_event_json` (broadcast/DM) fills it; every other event keeps `None`
  from `default_event()`.
- The Rust-built `MobileAttachment` is echoed verbatim by both frontends into
  `saveIncomingMessage`. FRB's `SaveIncomingMessageParams` and the web JSON
  params carry it, so no new method names or no-param drift entries were added.
- `impl From<MobileAttachment> for files::MessageAttachment` is now the one
  canonicalizer used both by `mobile_api::reply_draft` and by the incoming-save
  path. `sizeLabel`/`chip` stay display-only; Rust rebuilds the store descriptor
  and clamps an out-of-range `i64` size.
- `HomeScreen` passes the event descriptor into both broadcast persistence and
  the DM-only persistence path. `DmChatScreen` passes it into its own DM save
  and into the read-only live-broadcast mirror.
- Web parses the camelCase descriptor from an event frame with the existing
  `attachmentFromJson` helper and sends it back with `attachmentToJson`.

## Design decisions

- Frontends never construct or reinterpret a descriptor: they receive Rust's
  chip/label and hand the same object back at the persistence boundary. This
  keeps the reply preview, live bubble and reloaded history visually identical.
- An attachment-only event uses `content: ''`. That is still a real message, so
  the frontend save paths persist it rather than treating an empty body as
  "nothing happened".
- The conversion is centralized in Rust so the FRB reply-draft path and the
  reloaded-message persistence path cannot drift on size clamping or fields.

## Tests

- `mobile_node::tests::test_event_to_json_broadcast` checks the canonical chip,
  size label and MIME on the event envelope.
- `tests/messages.rs::test_incoming_message_persists_attachment_descriptor`
  checks the live save result and a DB reload.
- The web dispatch test posts a full event-shaped attachment descriptor and
  asserts both the response and the stored `attachment_json`.
- `http_node_api_test.dart` parses a live event descriptor and asserts the
  exact JSON params sent by `saveIncomingMessage`.
- Widget tests cover a live attachment-only broadcast and DM in `HomeScreen`
  and `DmChatScreen`, including the DM context mirror.

## Related records

- [`../files_attachments.md`](../files_attachments.md)
- [`2026-11-27_reply_ref_boundary_canonicalization.md`](2026-11-27_reply_ref_boundary_canonicalization.md)
- [`2026-11-27_rust_owned_reply_drafts.md`](2026-11-27_rust_owned_reply_drafts.md)
