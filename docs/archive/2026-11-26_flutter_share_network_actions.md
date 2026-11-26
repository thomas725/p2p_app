> **ARCHIVED** — historical record, not current behaviour.

# Flutter/web share network actions (2026-11-26)

The share detail route could inspect grants and manifests but could not ask the
network for a fresh copy or re-advertise its own copy. This slice adds those two
actions over the existing `NodeApi` facade — no new Rust, FRB or web binding.

## What shipped

- New `lib/src/screens/share_network.dart`: `ShareNetworkSection` owns a
  connected-peer picker plus "Refresh manifest" and "Re-advertise" buttons.
  It lives in its own file so the detail route does not grow deeper.
- Home supplies the current peer IDs and wires the buttons to
  `requestShareManifest` and `sendShareOffer`. The same widget therefore works
  on FRB and over the browser's JSON transport.
- Re-advertising rebuilds the `ShareOffer` from the share row with the two
  canonical modes/kinds and `ownerUserId: null`; the local
  `unknown:share:<id>` sentinel is never advertised as an ownership claim.
- A stale selected peer is reset when the peer list changes, failures appear
  inline, and both buttons are disabled while a request is in flight. On
  success the detail page reloads its manifest and grants from Rust, so it
  never keeps showing pre-refresh data.
- `test/widget/share_network_test.dart` covers the no-peer state, manifest
  refresh, canonical offer construction, stale-selection reset and failure
  path; `share_list_test.dart` checks that the section only appears when wired.

Peer discovery (`requestPeerShares`) and publish are still deferred: discovery
needs its own entry point, and a browser has no new filesystem snapshot to
publish.
