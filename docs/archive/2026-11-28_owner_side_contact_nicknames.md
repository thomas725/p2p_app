> **ARCHIVED** — historical record, not current behaviour.

# Owner-side contact nicknames (2026-11-28)

The contacts table already had `nickname_override`, but nothing could write it. This slice made
that column the Rust-owned, owner-side nickname for a bound user contact and wired the one writer
through the shared API, TUI and Flutter/web frontends.

## What shipped

- `identity::set_contact_nickname(contact_user_id: String, nickname: Option<String>)` is the single
  canonical writer. It derives the owner from the local device binding, refuses self and unknown
  users, validates with `nickname::validate_nickname`, and treats `None` as a clear. The existing
  roster loader resolves the override before the user's announced name and before device names.
- The FRB/web method `setContactNickname` exposes the writer to both frontends with owned strings;
  borrowed `Option<&str>` generated an opaque `Str?` bridge that did not compile. The web dispatch
  arm is covered by the FRB↔web drift guard.
- TUI: `Ctrl+N` on the Contacts tab opens a modal editor for the selected `is_user` row; empty input
  clears, invalid input keeps the prompt open with the Rust error, and `Esc` cancels. Bare `n` stays
  the name-sort shortcut. Unbound/device rows show an explanatory popup because their nickname is a
  separate device-level value on Peer Info.
- Flutter/web: `ContactList` takes an optional `onEditContactNickname` callback and renders the
  editor only on `isUser` rows. Home saves through `NodeApi.setContactNickname` and refreshes the
  roster so the Rust-resolved display name is shown.
- Widget/unit tests cover the dialog write/clear/error paths, the Home write-then-refresh wiring,
  and the HTTP camelCase wire shape; focused TUI key and render tests cover the new prompt.

## Why there is no cache invalidation

`DISPLAY_NAME_CACHE` memoizes the device-level name computed by `get_peer_display_name`. The contact
override is re-read from SQLite whenever the contact roster is rebuilt, which every frontend does
after a successful write, so adding a second invalidation point would only duplicate that reload
path.

## Deferred

Editing the contact nickname from Peer Info is not a separate surface: Peer Info remains the
device-level nickname editor. There is still no external sync of owner-side overrides between a
user's devices; each install keeps its own view, as with the existing device-local nicknames.
