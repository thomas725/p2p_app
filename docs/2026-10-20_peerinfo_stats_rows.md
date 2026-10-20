# Peer Info stats rows in both frontends (2026-10-20)

## Gap

The Peer Info tab was the one roster surface with no message activity: the TUI's
`render_peer_info_content` and Flutter's `PeerInfoScreen` both showed only the name origin +
first/last seen, even though each tab's Contacts table already displays DB-backed DM / broadcast
counts. Opening a peer dropped information the row you clicked from was showing.

## Fix

Both pages now render two extra rows — `Direct messages` and `Broadcasts sent` — sourced from the
**contact row that owns the peer's device** (TUI: `TuiRenderState.contacts`; Flutter: the
`MobileContactRow` passed down from `HomeScreen`), i.e. the exact numbers the Contacts table shows
for that row. Single source, no second query, and the two views can never disagree.

- TUI: `render_peer_info_content` looks the device up in `state.contacts` and appends the two lines
  (omitted when the peer has no contact row).
- Flutter: `PeerInfoScreen` gained an optional `contact: MobileContactRow?`; `HomeScreen` passes the
  clicked row (`_openPeerInfo`) and, for the message-sender path, resolves it with the new
  `_contactForPeer(peerId)` helper. No FFI/web-method change: the data already rides
  `getContactsWithStats`, so the web client (which reuses the Flutter screens) gets it for free.

## Tests

- `tests/tui_render_integration.rs::render_tests::test_render_peer_info_shows_contact_message_stats`
  sets a contact's `dm_count`/`broadcast_sent_to_peer` and asserts both lines render;
  `…_omits_stats_for_peer_without_a_contact_row` asserts they are absent for an unknown peer id.
- `apps/flutter_app/test/widget/peer_info_test.dart` covers both directions over a
  `MobileContactRow` fixture (`renders the contact stats rows when a contact is supplied`,
  `renders no stats rows without a contact`).
