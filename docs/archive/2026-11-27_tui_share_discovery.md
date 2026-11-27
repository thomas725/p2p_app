> **ARCHIVED** — historical record, not current behaviour.

# TUI share discovery (2026-11-27)

The Shares tab could list local shares, edit grants/visibility, run network
actions and publish deletions, but a terminal user could not act on a peer's
`share_offered` announcement. Flutter already kept those offers in a discovery
route. This slice brings the same flow to the TUI without adding a new FRB/web
binding: Rust owns the merge/accept policy and the terminal only drives it.

## What shipped

- `shares::DiscoveredShare` plus `shares::merge_discovered_share()` are the one
  dedup rule, keyed by `(peer_id, share_id)`. A retransmit replaces the row (so
  newer metadata wins) without duplicating it; an empty share id is ignored.
  Flutter mirrors the same key in Dart because the offer list lives in widget
  state and cannot cross FRB.
- `handle_share_event` records every `ShareOffered` event into transient
  `AppState::discovered_shares` before the notice/reload. An offer is untrusted
  metadata: it is never persisted until the user accepts it.
- `f` queues `SwarmCommand::ListShares` for the canonical sorted connected peer
  (the same `selected_share_peer_id()` the other network actions use), and shows
  a popup when no peer is connected. `j`/`k` move an independent discovered-offer
  cursor; `Enter` accepts the highlighted offer through `shares::accept_offer`.
  The cursor is preserved by `(peer, share id)` across merges and removals so a
  retransmit or a concurrent event cannot retarget `Enter`.
- Acceptance reuses the canonical store: the name and id are validated, an
  untrusted owner claim becomes a local-only marker, the stub starts at
  revision 0 (the advertised revision is not believed), and locally owned or
  bound shares are never overwritten. The offer is consumed, the share list is
  reloaded and the new row is selected.
- The detail pane renders discovered offers next to the connected peers, even
  when no local share exists. Every new key is inert while a modal editor owns
  the keyboard, matching the rest of the Shares tab.

## Tests

Direct tests cover the Rust merge rule (dedup, newest-wins, empty id) and cursor
stability across merge/removal. A command-processor test asserts an offer event
is recorded without persisting a share. Real-key sessions cover `f` (with and
without a peer), cursor isolation and clamping, modal inertness, and `Enter`
accepting through the canonical store (including the untrusted owner marker).
Render projection and renderer integration tests assert the offer pane and its
unverified-owner hint.

## Why this design

Discovery is protocol metadata, not an account-layer truth, so no new FRB/web
surface is justified: both frontends already receive `share_offered` on their
event bus. Keeping merge and accept in Rust means the terminal cannot invent an
owner claim or a revision, and the existing owner-side ACL remains the single
authority when a stub is later filled by a manifest.

Commit: to be linked by git history; no hash is invented in this record.
