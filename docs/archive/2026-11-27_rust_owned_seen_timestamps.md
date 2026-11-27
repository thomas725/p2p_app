> **ARCHIVED** — historical record, not current behaviour.

# Rust-owned seen timestamps (2026-11-27)

`firstSeen` and `lastSeen` arrive in Flutter as display-ready strings from Rust.
The Contacts and Peer Info screens used to run them through a second Dart
formatter that sliced to 19 characters, replaced `T` with a space, dropped a
trailing `Z`, and turned an empty string into `unknown`. That made Dart a second
display authority for a value Rust already formats with
`fmt::format_peer_datetime`, and the old widget fixtures asserted the Dart
reformat using wire shapes the Rust producers never emit.

## What shipped

- Removed `formatSeenTimestamp` from `apps/flutter_app/lib/src/util/ui_utils.dart`.
  `formatTimestamp(DateTime?)` remains for Settings' live connection status,
  which still starts from a live `DateTime` rather than a Rust string.
- `ContactList` contact rows and expandable device rows now render
  `c.lastSeen`/`c.firstSeen` and `d.lastSeen`/`d.firstSeen` verbatim.
  `PeerInfoScreen` renders `peer.firstSeen`/`peer.lastSeen` verbatim.
- Widget fixtures now use canonical `YYYY-MM-DD HH:MM:SS` strings. Each screen
  also has an explicit pass-through case with an unusual string, proving Dart
  no longer parses or truncates what it is given.
- Rust coverage tightened: `test_format_peer_datetime` asserts the exact
  canonical output, and a source guard asserts the three affected Dart files
  never reintroduce `formatSeenTimestamp`.

## Why this shape

`contacts::load_contacts_with_stats`, `mobile_node::get_known_peers` and the web
JSON mapping all deliver the same string the TUI renders. Parsing that string
again in Dart could silently truncate a future Rust display format, so the
frontend stays a pure renderer. No FRB binding, web method, wire field or
database column changed.

## Coverage

- `apps/flutter_app/test/widget/contact_list_test.dart` asserts verbatim values
  for both contact and device rows, including an unrecognized string.
- `apps/flutter_app/test/widget/peer_info_test.dart` asserts both seen rows
  render verbatim, including an unrecognized string.
- `tests/unit/unit_lib.rs` pins the exact Rust formatter output and guards the
  Dart sources against a second formatter.

An empty string now renders empty rather than `unknown`. The Rust producers
always write a formatted timestamp, so an empty value is not part of the normal
wire shape; if one ever appears, it is passed through like any other untrusted
string instead of being reinterpreted by Dart.
