> **ARCHIVED** — historical record, not current behaviour.

# Share offers no longer advertise the local unknown-owner marker (2026-11-24)

`shares::accept_offer` materialises an offer that carries no verifiable owner as
`unknown:share:<share-id>`. That sentinel is deliberately local bookkeeping:
real account ids are `PeerId` strings, so the namespaced value cannot collide
with one, and the ACL is always enforced by the real owner.

`shares::offer_for_share` used to copy that sentinel straight into a new
`ShareOffer`. Re-advertising a remote stub to another peer therefore presented
a local bookkeeping value as a real ownership claim. The offer now carries
`owner_user_id: None` whenever the stored owner is the sentinel; the receiver
materialises its own local marker as before.

`shares::offer_for` was written for this mapping but had no callers (it also
took a `ShareManifest`, while a remote stub may have no manifest). It now takes
the `Share` domain view and is the single construction path, so id, name and
revision cannot drift between the two offer builders. `is_unknown_owner_id`
keeps the sentinel check in one place.

## Coverage

- `offer_for_maps_share_and_owner_without_inventing_ownership` verifies
  all fields and that a `None` owner stays `None`.
- `offer_for_share_never_advertises_the_unknown_owner_marker` checks a locally
  owned share keeps its real owner while an anonymous remote stub is
  re-advertised without one. It also pins the deliberate revision `0` of a
  stub: a first real manifest must remain acceptable.

No wire, DB, FRB, web or frontend surface changed — only the contents of an
advertisement for an already-ownerless share.
