> **ARCHIVED** — historical record, not current behaviour.

# Flutter Settings peer-row order (2026-11-27)

The TUI Settings tab sorts connected peers and bound devices by peer ID, but
Flutter rendered the event-arrival order for connected peers and Rust's
most-recent-first order for bound devices. The rows were clickable in both
frontends; only the ordering diverged. This slice aligns Flutter with the TUI
without touching the DB, wire, FRB or web surfaces.

## What shipped

- `settings.dart` copies and sorts the two lists at render time. Peer IDs are
  ASCII, so Dart's `String.compareTo` matches Rust's `str` ordering; bound
  devices use peer ID then bound-at as the tiebreak, matching the TUI renderer.
- Sorting the copies leaves the `LiveStatus` notifier and the `devices` list
  untouched. A plain string sort is intentionally local — exposing a bridge
  method just to order two already-present lists would add generated code and
  web/FRB drift for no benefit.
- Widget coverage verifies both orders with deliberately shuffled input and
  confirms a sorted connected-peer row still opens the right Peer Info page.

No Kotlin or generated bridge code changed; the change is confined to one
Flutter screen and its widget test.
