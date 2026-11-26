> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web peer-share discovery (2026-11-26)

The share lane could ask a peer for its offers (`requestPeerShares`) and accept
one locally (`acceptShareOffer`), but no frontend exposed either call. This
slice adds a discovery route on the existing `NodeApi` facade; no new Rust, FRB
or web binding was needed.

## What shipped

- The share list's app bar gained a "Discover peer shares" action. Home opens
  `ShareDiscoveryScreen`, which picks a currently connected peer and calls
  `requestPeerShares`.
- `share_offered` events are decoded once in `share_discovery.dart` through the
  same camelCase DTO used by the web transport. Offers are held in a
  `ValueNotifier` keyed by `(peerId, shareId)`: retransmits replace the old row
  rather than stacking duplicates, and offers from different peers stay
  separate. An open discovery route sees new events without being rebuilt.
- Accepting an offer calls `acceptShareOffer` and reloads the local list. This
  is explicit because an offer is untrusted metadata — Rust's `accept_offer`
  materialises a stub and can never rewrite our ACL, root or revision. The UI
  labels the advertised owner as unverified.
- Peer selection, discovery failures and accept failures stay local to the
  widget; both buttons are disabled while a request is in flight.
- Eight tests cover the no-peer state, search, exact offer acceptance, live
  notifier updates, event decoding (including malformed payloads), dedup and
  both failure paths.

Publish and TUI share UI remain deferred: a browser has no new filesystem
snapshot to publish, and the TUI still only logs share events.
