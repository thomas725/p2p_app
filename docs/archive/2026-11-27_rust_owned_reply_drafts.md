> **ARCHIVED** — historical record, not current behaviour.

# Rust-owned reply drafts (2026-11-27)

Flutter used to freeze a reply snapshot in Dart: `util/reply.dart` resolved the
sender label, appended the attachment chip and bounded the preview with its own
copy of Rust's rules. That duplicated the sender, body and 80-character preview
logic in a second language, so a quote captured in the browser could drift from
one captured by the TUI or in process.

## What shipped

- `mobile_api::reply_draft` is the single pure helper that builds a `ReplyRef`.
  It reuses `ReplyRef::new` for the preview bound, `files::render_message_body`
  for attachment chips and a small sender-label rule (resolved nickname, short
  peer ID, `Me` last) shared by both frontends.
- The helper is exposed through FRB (`replyDraft`) and through the web JSON
  method dispatcher. `NodeApi.replyDraft` is implemented once per transport;
  the three chat surfaces simply ask the node when the user picks Reply.
- Dart keeps only the display-only sender label and quoted header used by live
  bubbles and the copy transcript. `replyDraftFor` delegates to `NodeApi` and
  no longer implements any part of the snapshot.
- `MobileAttachment` now derives `Deserialize` so the web boundary can accept
  the same camelCase projection Rust emits.

## Design decisions

- The helper stays in `mobile_api` because that module is already the thin
  facade shared by FRB and web; the canonical logic remains in `reply` and
  `files`, so no rule was moved out of the core.
- Flutter's chat-row fields are passed as flat named arguments, matching the
  generated FRB signature and the web JSON keys. No input struct was needed.
- Reply actions are asynchronous on every chat screen: both transports are
  already async, and the draft is captured when the user acts rather than on
  bubble build. This avoids caching a derived value on every rendered row.

## Tests

- `mobile_api::tests` covers the canonical chip (file-only and mixed), sender
  fallback (nickname / own / short peer ID), whitespace collapsing, the 80-code
  point bound and rows without a stable wire id.
- `web::tests` checks the JSON response shape and the params-less routing of
  `replyDraft`.
- `tests/unit/unit_reply.rs` guards the Flutter boundary: `reply.dart` must
  delegate to `nodeApi.replyDraft` and must not reintroduce the old duplicate
  helpers.
- Widget tests in `messages_test.dart` use the new `FakeNodeApi.replyDraftOverride`
  seam to verify draft capture, forwarding and cancel behavior.

## Related records

- [`2026-11-18_reply_quote.md`](2026-11-18_reply_quote.md)
- [`2026-11-27_reply_ref_boundary_canonicalization.md`](2026-11-27_reply_ref_boundary_canonicalization.md)
- [`../architecture.md`](../architecture.md)
