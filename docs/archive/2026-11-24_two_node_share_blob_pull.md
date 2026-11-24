> **ARCHIVED** — historical record, not current behaviour.

# Two-node share blob pull with separate databases (2026-11-24)

The share wire test in `tests/swarm_handler.rs` already covered offer → list →
manifest → stale-write retry, but both nodes shared one process-wide database in
that test: the manifest's blobs were effectively pre-seeded, so the files lane
was never exercised. `test_share_blob_pull_applies_with_separate_node_databases`
closes that gap.

## Harness shape

- Test builds keep the DB URL thread-local, so each node runs on its own OS
  thread with a **current-thread Tokio runtime** and its own SQLite file under a
  `TempDir`. There is no `set_db_url` race and no cross-node DB access; the
  thread-local blob store (`<db>.files`) is separate too.
- The swarm is constructed **inside** the runtime: mDNS opens a netlink socket
  at build time and panics without an active reactor. B publishes its listen
  address over a `std::sync::mpsc` channel; A dials it and waits for the
  handler's `PeerConnected` event before sending the offer.
- A creates a real blob with `files::store_outgoing_bytes`, publishes a one-file
  manifest and offers a **public, read-only** share. B accepts the offer with a
  bound local folder, requests the manifest, and waits for `ShareUpdated`.
- A's handler stays alive on a `tokio::sync::oneshot` until B has asserted, so
  aborting the runtime cannot starve the meta/chunk server mid-pull.

## What it proves

B has no blob before the request. The missing entry makes the swarm handler
call `maybe_start_fetch`, the meta/chunk responses assemble the file into B's
own store, and `finish_received_file` → `shares::note_blob_available` applies
the queued manifest. The test asserts revision `1`, one written file, the
applied bytes in B's bound folder, and the pulled blob's presence in B's store.

No production code changed — this is integration coverage for the existing
share/files lanes. The remaining share work is UI only (TUI Shares tab and
Flutter/web share screens).
