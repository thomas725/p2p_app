> **ARCHIVED** — historical record, not current behaviour.

# Share manifest hardening (2026-11-27)

Two contained correctness fixes landed in the share store after the share UI
work. Neither changes the FRB/web surface, wire shape or any frontend: both are
inside `src/shares.rs` (plus a small shared-connection helper in
`src/identity.rs`).

## Equal-revision conflicts

`save_manifest` previously treated every equal revision as an idempotent
re-fetch and rewrote the cached manifest. A peer could therefore replace a
manifest at revision *N* with different content at the same *N*, silently
changing which files a share advertises without advancing the revision that
frontends and the deferred-apply logic key on.

Now the read inside the write transaction selects `(revision, manifest_json)`.
A lower revision is still rejected as a rollback; an equal revision is accepted
only when there is no cached JSON or the cached JSON is byte-identical to the
canonical re-serialization of the submitted manifest. Different content at the
same revision is rejected with a `conflicts with cached content` error, leaving
the cached manifest, revision, entry rows and blob GC references untouched.
`manifest_to_json` is the single canonical encoder, so an identical manifest
always compares equal.

## ACL checked inside the submission transaction

`accept_submitted_manifest` used to call `can_write(share_id, peer_id)` before
opening its write transaction. That read used a separate connection, so a grant
revoked between the check and the write could still let a formerly authorised
peer publish one last manifest.

The ACL resolution now runs on the transaction's connection:

- `identity::find_user_id_for_peer_conn(conn, peer_id)` resolves the verified
  device binding on the caller's connection; `find_user_id_for_peer` delegates
  to it.
- `shares::get_share_conn(conn, share_id)` and
  `shares::permission_for_share(conn, share, peer_id)` load the share and resolve
  the requester's effective mode on that same connection. The public/owner
  branches are unchanged; private grants are read from the transaction view.

`accept_submitted_manifest` checks `permission_for_share(...) ==
Some(ReadWrite)` alongside the base-revision check and the write, so a revoke
or revision bump committed before the transaction begins is observed and
denied. Any DB error still fails closed.

## Coverage and docs

`tests/unit/unit_shares.rs` adds:

- `save_manifest_rejects_same_revision_with_different_content`: an identical
  re-fetch still succeeds, a conflicting same-revision manifest is refused,
  and the cached manifest, revision and GC references stay on the first blob.
- `accept_submitted_manifest_reads_acl_on_the_write_connection`: a revoke staged
  in a transaction is visible to `permission_for_share` on that connection
  (`None` where it was `ReadWrite` before), and a committed revoke makes the
  submission fail with `may not write` while revision and manifest stay put.

`docs/files_attachments.md` and `docs/backlog.md` record the new semantics;
`docs/codebase_metrics.md` was regenerated.
