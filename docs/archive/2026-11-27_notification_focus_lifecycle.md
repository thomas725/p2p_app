> **ARCHIVED** — historical record, not current behaviour.

# Notification focus lifecycle and share-peer pickers (2026-11-27)

The Rust notification key rule was already canonical, but two Flutter wiring
details could still make the visible screen disagree with it:

- a full-screen chat restored its previous focus key in `dispose` by comparing
  strings. When a bottom-nav tap changed focus before the popped screen was
  disposed, that stale restore overwrote the destination tab's focus;
- Peer Info pushed no focus at all, so a message in the chat underneath was
  silently suppressed even though the TUI explicitly treats Peer Info as a
  non-chat tab;
- share network/publish pickers were given every known peer, while their labels
  and the TUI both promise *connected* peers.

## What shipped

- `ConversationFocusGuard` replaces raw previous-key strings. Focus is a base
  value plus a stack of screen-local entries, each keyed by a unique token.
  Releasing a guard only pops its own entry and anything above it; after the
  shell publishes a new focus, the stale guard is a no-op. A pushed focus may
  be `null`, so Peer Info explicitly suppresses nothing.
- DM and group chat screens hold a guard; Peer Info now pushes and releases a
  null guard. The single suppression rule remains in
  `deliverNotification`: foreground + exact conversation key.
- `connectedShareActionPeerIds` turns the connected-peer set into a sorted,
  deduplicated list. Home uses it for both share discovery and the share
  detail network/publish pickers, matching the TUI's
  `connected::sorted_connected_peer_ids` and excluding offline roster rows.

## Tests

- `notifications_test.dart` covers nested guards, a `null` pushed focus and a
  stale guard released after navigation has changed focus.
- `share_list_test.dart` covers deduplication and deterministic peer-ID order.

No Rust, FRB or web surface changed: this was a frontend lifecycle and wiring
repair, not a policy change.
