# Contacts search in both frontends (2026-10-21)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/README.md](../README.md).

The last open frontend-parity item: a search affordance over the Contacts
roster. Both frontends get the same semantics, and both keep the canonical Rust
sort/format paths as the single source of ordering.

## Shared matching rule

A contact matches a query when the query (lowercased) is a substring of any of:

- the contact id,
- the contact display name,
- any device's peer id,
- any device's display name.

Including the *devices* is what makes a grouped user findable by the peer id of
one of its machines, or by a device nickname; including the contact id mirrors
the TUI's pre-grouping row identity. Rust owns the rule
(`state::contact_matches`); Flutter mirrors it in `ContactListState._visibleRows`
for the same reasons the sort is mirrored only through the Rust API — there is no
FRB binding that could filter a Dart-side list, and the two implementations are
pinned by tests on both sides.

## TUI

- New `InputMode::SearchingContacts` (no new `TuiRenderState` bool —
  `struct_excessive_bools` is already at its limit).
- `AppState.contacts` is now the **filtered visible list**; a new
  `AppState.contacts_all` holds the unfiltered, sorted rows.
  `resort_peers()` rebuilds + Rust-sorts into `contacts`, then copies that to
  `contacts_all` and re-applies the query — so the table, the click→row mapping
  (`contact_table_plan`), the selection and the scroll window all agree on one
  filtered list, with no DB access on the keystroke path.
- Keys on the Contacts tab:
  - `/` opens the search box (consumed: it never sorts),
  - printable keys while open feed the shared `chat_input`, re-filtering live,
  - `Backspace` edits the query,
  - `Enter` keeps the filter and closes the box,
  - `Esc` clears the filter and closes the box (handled in `handle_esc_key`).
- Precedence matters: the search box is checked **before** the `c`/`e`/`i`
  Contacts actions, otherwise typing "carol" would place a call and expand a
  row. The Contacts action keys were extracted to
  `handle_contacts_action_key`, which is also reached from the Peer Info tab.
- The input-section title becomes `Search Contacts` while open (the box renders
  even though the Contacts tab has no chat input), and the shortcut bar
  documents `/: search`.

## Flutter

- `ContactListState` gained a `_query` + `TextEditingController` behind a
  `contacts-search` keyed `TextField` in the tab header, a `contacts-search-clear`
  button, and a live `Contacts (n/total)` count.
- Filtering happens **before** the Rust `sortContacts` call: `_computeSorted`
  sorts only the visible subset and maps results back against it, so the Rust
  comparator remains the only ordering authority and cannot return a hidden row.
  The sort cache key includes `_query`, so a query change re-sorts off-band.
- The empty state reads `No contacts match.` when a filter is active (vs.
  `No contacts yet.`).

## Tests

- Rust: `state::contact_matches` (name/id/device), filter + restore, selection
  preserved across a filter, begin/end/Esc/Enter mode transitions
  (`tests/unit/unit_bin_tui_state.rs`); `/`→typing→Backspace→Esc and
  Enter-keeps-filter over `process_key_event`
  (`tests/unit/unit_bin_tui_input_processor.rs`); the
  `Search Contacts` input-section title (`tests/tui_render_integration.rs`).
- Flutter: `test/widget/contact_list_test.dart` — name filtering + clear,
  case-insensitive id/device-name matching, "the Rust sort only ever sees the
  filtered subset", and the no-match hint/count.

## Verification

`cargo ct` clean · `cargo test --all-features` green · `flutter analyze` clean ·
`flutter test` 82/82 · `flutter build web` ✓.
