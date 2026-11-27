> **ARCHIVED** — historical record, not current behaviour.

# Untrusted share-owner claim hardening (2026-11-27)

`shares::accept_offer` used to copy `ShareOffer.owner_user_id` straight into a
new local `shares` row from wire data. That id later unlocked the owner ACL
shortcut in `permission_for_share` and the owner-only visibility toggles in both
frontends, so a peer could advertise `owner_user_id: <our user id>` and gain
write access to a mirror it did not own. The advertised value is a claim, not a
verification: the share lane's authoritative identity is the device binding
resolved from the connection peer id.

This slice keeps the FRB/web surface, wire JSON and Dart/Kotlin code unchanged.
The fix is Rust-only, inside `src/shares.rs` and the share response arm of
`src/swarm_handler.rs`.

## What changed

- `ShareOffer.owner_user_id` is documented as a claim. New remote stubs store
  either `unknown:claim:<claim>` (bounded to `MAX_OWNER_CLAIM_LEN`, rejecting
  empty/marker claims) or the existing no-claim `unknown:share:<share-id>`
  sentinel. A repeated bare-stub offer demotes any legacy raw id to the claim
  marker instead of preserving it as verified.
- `is_unknown_owner_id` covers both marker prefixes. `permission_for_share`
  refuses the owner shortcut for either one, `offer_for_share` never advertises
  them, and `set_visibility`/the frontends' owner-only controls keep comparing
  against the local user id, so markers cannot pass those checks.
- `record_verified_owner` upgrades a marker only when a conditional update
  matches the exact `unknown:claim:<verified-user>` row. A wrong user, a
  no-claim sentinel, an unbound peer, a DB error and a locally owned row are all
  no-ops. `handle_share_manifest_response` resolves the serving peer once and
  calls it after a successful `save_manifest`, so a real owner serving a manifest
  to a mirror that accepted their offer first is still recognised.
- `accept_submitted_manifest` resolves the submitting peer on the transaction
  connection and calls the same upgrade there, before the ACL/revision checks.
  This lets a legitimate owner publish to a mirror even when no manifest
  response upgraded it earlier, without introducing a check/write race.

## Coverage and docs

`tests/unit/unit_shares.rs` now pins:

- a claimed offer stays a local marker and is idempotent, while marker-shaped
  and oversized claims fall back to the no-claim sentinel;
- a spoofed claim does not grant `can_write`, `set_visibility` or a verified
  re-advertisement, and a wrong verified user cannot upgrade it;
- a matching Rust-resolved user does upgrade the marker, after which the owner
  shortcut and re-advertisement work;
- a no-claim marker and a locally owned row are never upgraded;
- a bound stranger cannot publish through the claim, while the verified owner
  can, with the upgrade and revision write committed together.

`docs/files_attachments.md` and `docs/backlog.md` describe the new semantics;
`docs/codebase_metrics.md` was regenerated.
