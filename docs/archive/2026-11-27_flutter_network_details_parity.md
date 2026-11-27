> **ARCHIVED** — historical record, not current behaviour.

# Flutter network-details parity (2026-11-27)

The Flutter Settings card already received the full `NetworkDetails` snapshot
from Rust, but it dropped two fields the TUI renders: AutoNAT's confirmed
public address and the swarm's listen addresses.

## What shipped

- The AutoNAT row now appends the confirmed public address in parentheses,
  matching the TUI (`natStatus (addr) — probes …`).
- A `Listen addresses` section now sits between external addresses and
  interfaces. It shows each `listenAddrs` entry as selectable text, or an em
  dash when the list is empty, exactly like the TUI block.
- The card carries a `network-details-card` key so widget tests can scope
  finders when a label appears elsewhere on the Settings page.
- Tests cover both the populated and empty-list cases and the AutoNAT label
  with a public address.

No Rust, FRB or web change was needed: the fields and their labels already
exist in the shared `NetworkDetails` snapshot; only the Flutter renderer was
dropping them.
