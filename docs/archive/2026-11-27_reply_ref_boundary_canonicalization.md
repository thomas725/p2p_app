> **ARCHIVED** — historical record, not current behaviour.

# Reply snapshots are canonicalized at every trust boundary (2026-11-27)

`ReplyRef { msg_id, sender, preview }` is an immutable quote snapshot carried on
the chat wire, persisted in `reply_to_json` and rendered above a message. Rust
owned the single-line, 80-character `preview_line` rule, and `ReplyRef::new`
applied it to the preview. That was not enough: `ReplyRef` derives serde and its
fields are public because Flutter Rust Bridge's generated SSE codec constructs
the struct directly. A crafted peer payload or a direct FRB call could therefore
put newlines or arbitrarily long text into `sender`/`preview` without ever
calling the constructor — the TUI's `↳ sender: preview` header then became an
untrusted multi-line injection into the transcript.

## What shipped

- `serde(deserialize_with = "deserialize_preview_line")` on both display fields
  canonicalizes every JSON ingress: gossipsub/request-response wire payloads,
  web JSON method params and persisted `reply_to_json` rows.
- `ReplyRef::canonicalized()` and `canonicalized_option()` re-establish the
  invariant for non-serde ingress. `reply::encode` canonicalizes before writing
  the DB column, so a direct FRB snapshot cannot smuggle a raw quote into long-
  term storage.
- `swarm_handler` canonicalizes outbound broadcast/group snapshots in the wire
  builders and the DM snapshot in `send_dm` before it is sealed, outboxed or
  sent. The TUI's quoted header canonicalizes both fields at render time as a
  final defense for any in-memory snapshot.
- `ReplyRef::new` still only canonicalizes its `preview` argument: canonicalizing
  the owned `sender` there trips strict clippy's `needless_pass_by_value`. Full
  canonicalization lives in `canonicalized()` and the boundary helpers instead.

No FRB signature, web JSON shape, migration, DB column or wire type changed;
only the values admitted at those boundaries are canonical.

## Tests

- serde ingress and a nested wire payload flatten newlines in both fields;
- storage encoding canonicalizes a directly constructed snapshot, and the TUI
  header does the same;
- `canonicalized_option` handles both `None` and `Some`;
- the web `saveIncomingMessage` frame test sends newline-bearing JSON and
  asserts the canonical returned snapshot and persisted row;
- the broadcast/group wire builders canonicalize a directly constructed
  snapshot before serialization.

See the live invariant in [`../architecture.md`](../architecture.md).
