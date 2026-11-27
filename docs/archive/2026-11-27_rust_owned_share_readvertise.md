> **ARCHIVED** — historical record, not current behaviour.

# Rust-owned share re-advertising (2026-11-27)

Share network actions previously asked Dart to rebuild a `ShareOffer` from the
display row and call a raw `sendShareOffer` binding. That duplicated Rust's
canonical offer rules and left a frontend path able to supply wire-level offer
fields. This slice replaces it with an id-only binding that builds the offer in
Rust.

## What shipped

- `api::readvertise_share(share_id, peer_id)` is the only UI-facing offer-send
  path. It calls `shares::offer_for_share`, rejects an unknown share before
  queueing anything, and sends the canonical offer to exactly one peer. The
  public `api::send_share_offer` binding is gone.
- Web exposes `readvertiseShare` with camelCase `shareId`/`peerId`, reusing the
  manifest-request parameter struct; the raw offer parameter struct and dispatch
  arm are gone. Both web dispatch/no-param drift tests were updated.
- Every Dart transport implements `NodeApi.readvertiseShare`. `ShareNetworkSection`,
  `ShareDetailScreen` and `HomeScreen` now pass only share and peer IDs; no Dart
  code constructs a `ShareOffer` for re-advertising.
- Rust API coverage asserts that an unknown share fails before a swarm command
  is queued; the existing `offer_for_share` unit test still pins the owner-marker
  invariant. Dart widget and HTTP transport tests assert the ID-only callback
  and JSON frame.

## Design decisions

- **Rust owns the wire object.** The local `unknown:share:<id>` owner marker is
  bookkeeping; only `offer_for_share` may turn a stored share into an advertised
  offer. A frontend cannot accidentally or deliberately advertise a false owner.
- **No raw escape hatch.** Leaving `sendShareOffer` exposed would preserve the
  bypass this slice removes, so the raw offer-send binding was deleted rather
  than kept as a compatibility alias.
- **IDs are the frontend contract.** Peer IDs are already protocol identity, and
  share IDs are stable store keys. Passing only those keeps FRB and web surfaces
  identical and avoids DTO duplication for a wire type the frontend should not
  author.

## Tests

- `src/api/mod.rs`: unknown-share re-advertise fails with `unknown share`.
- `tests/unit/unit_shares.rs`: `offer_for_share` never advertises the local
  unknown-owner marker.
- Web: `methods::dispatch(name, Null)` routing and both no-param drift lists
  include `readvertiseShare`.
- Flutter: `share_network_test.dart` asserts the `(shareId, peerId)` callback;
  `http_node_api_test.dart` asserts the `readvertiseShare` JSON frame.

Commit: see the git history for this dated slice. No hash is invented here.

## Related records

- [`2026-11-26_flutter_share_network_actions.md`](2026-11-26_flutter_share_network_actions.md)
- [`2026-11-24_share_offer_owner_marker.md`](2026-11-24_share_offer_owner_marker.md)
- [`../files_attachments.md`](../files_attachments.md)
