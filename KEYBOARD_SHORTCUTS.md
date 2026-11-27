# Keyboard & Mouse Shortcuts

Every binding in the TUI. Where a binding only exists on one tab, the table
says so.

The in-app shortcut bar at the bottom of the screen shows the active
bindings for the current tab, and `?` opens a compact modal table anywhere
outside a prompt. This file is the complete reference. The Flutter/Web app
is touch-first and defines no custom keyboard bindings; its Settings tab has
a **Help / controls** dialog describing the on-screen actions instead.

## Tab navigation

| Key | Action |
|-----|--------|
| `Tab` | Next tab |
| `Shift+Tab` / `BackTab` | Previous tab |
| `Click` on a tab title | Jump to that tab |
| `X` on a DM / Peer Info / Group tab title | Close that tab |
| `Ctrl+W` | Close the current DM, Peer Info, or Group chat tab |

Tabs are ordered: **Chat**, **Contacts**, **Groups**, group chats, DMs,
Peer Info tabs, **Log**, **Settings**, **Shares**. Peer Info and DM tabs are
transient — they are inserted, deduplicated by peer, and closed with `Ctrl+W`.
Shares is a permanent suffix tab after Settings. While a modal prompt owns the
input (nickname, message edit, reaction, group name, dial, network peer or
share grant), `Tab` and `Shift+Tab` are no-ops so the prompt cannot be stranded
on another tab — except in the add-grant prompt, where `Tab` toggles the mode.
Contacts search is inline, not modal: Tab and arrows keep working there.
Moving focus away from Contacts — by Tab, a tab click, a row click, or opening
Peer Info — closes the search box but keeps the applied filter; the shared input
is cleared so a Composer draft does not inherit the query.

## Scrolling

Message tabs scroll **hover-based**: the section under the mouse pointer is the
one that scrolls. If the pointer has never been over the content, the focused
section is used. On list tabs (Contacts, Groups, Settings and Shares), the arrow
keys and the mouse wheel move the highlighted row instead; the detail pane
follows the selection.

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Scroll the hovered message section one line; move the list selection one row |
| `Ctrl+Up` / `Ctrl+Down` | Move the message cursor on Chat / DM / Group chat, or the grant cursor on Shares; clamped at both ends |
| `PgUp` / `PgDn` | Page-scroll the hovered message section; page the list selection |
| `Home` | Jump to the first message / list row in the focused section |
| `End` | Jump to the newest message / last list row (re-enters auto-scroll in a conversation) |
| `Mouse wheel` | Scroll the hovered message section; move the list selection one row |

Which message section that is, per tab:

| Tab | Sections |
|-----|----------|
| Chat | broadcast messages |
| DM / Direct | broadcast messages (top half) and the conversation (bottom half) — the pointer picks |
| Group chat | the group's messages |
| Log | the log lines |

Bare arrows and the wheel only scroll — they never arm a message selection.
`Ctrl+Up`/`Ctrl+Down` arm and move an explicit message cursor on the Chat,
DM/Direct and Group-chat tabs; it is clamped at both ends, and moving it brings
the row into view. On a DM tab the cursor targets the bottom DM transcript only:
bare arrow/wheel scrolling stays hover-aware across the top broadcast pane and
the bottom DM pane. `Esc` clears the active conversation's cursor before
returning to Chat; each conversation keeps its own cursor.

Peer Info has no scrollable content and ignores the wheel. On Contacts, Groups,
Settings and Shares, `↑`/`↓`, `PgUp`/`PgDn`, `Home`/`End` and the wheel move the
highlighted row; Settings moves the persisted network-peer row. On Shares the
detail pane wraps to show the selected share's grants and manifest, and
`Ctrl+Up`/`Ctrl+Down` moves the grant cursor independently of the share row.
Changing shares resets the grant cursor, so `x` always targets that share.

Mouse input follows the same ownership order as the keyboard. A device-binding,
group-leave or delete-message prompt and a live call swallow every click and
wheel, so nothing behind the overlay changes. A plain popup is dismissed by one
click even while a prompt box is open; with no popup left, the prompt box owns
the pointer. Contacts search is inline: the wheel still moves the highlighted
row and a click still leaves the search box while keeping the applied filter.

### Unread banners

When messages arrive while you are scrolled up, a cyan
`▼ N new message(s) — End: jump to latest` banner appears at the bottom of
that conversation's pane — the broadcast pane, a DM pane, or a group chat.
Messages for a conversation you are actively reading follow the tail as usual;
only conversations left scrolled up accumulate a count. Press `End` (or scroll
to the bottom) to jump to the newest message; the counter clears. This mirrors
Flutter's "N new" jump button in DM and group chats.

## Message interaction

| Action | Result |
|--------|--------|
| `Click` a message in the **Chat** tab | Open the sender's Peer Info |
| `Click` a message in the **DM** tab's broadcast pane | Open the sender's Peer Info |
| `Click` a message in the **DM** tab's conversation pane | Open the partner's Peer Info |
| `Click` a message in a **Group chat** tab | Open the sender's Peer Info |
| `Click` a line in the **Log** tab | Nothing — log lines have no sender |
| `Ctrl+P` in **Chat** / **Group chat** | Open the newest known sender's Peer Info |
| `Ctrl+P` in **Settings** | Open the first connected peer's Peer Info, or the first bound device when nobody is connected |
| `Ctrl+P` in **Log** | Nothing — log lines have no sender |
| `Ctrl+Up` / `Ctrl+Down` in **Chat** / **DM** / **Group chat** | Move the message cursor (clamped at both ends) |
| `Ctrl+R` in **Chat** / **DM** / **Group chat** | Quote the selected row (newest if none selected) |
| `Ctrl+Y` in **Chat** / **DM** / **Group chat** | Add or remove an emoji reaction on the selected row (newest if none selected) |
| `1`–`6` in the reaction prompt | Apply the matching quick-reaction emoji (`👍`, `❤️`, `😂`, `🎉`, `😮`, `😢`) |
| `Ctrl+C` in **Chat** / **DM** / **Group chat** | Copy the selected row's rendered line (newest if none selected) to the clipboard |
| `Ctrl+C` in **Log** | Copy all log lines to the terminal clipboard (same as Flutter's "Copy All") |
| `Ctrl+O` in **Chat** / **DM** / **Group chat** | Open the selected row's attachment; with no cursor, the newest row with an attachment |
| `Esc` in **Chat** / **DM** / **Group chat** | Clear the active conversation's reply draft, then that conversation's message selection, one press per layer |
| `Delete` in **Chat** / **DM** / **Group chat** | Hide your selected sent row (newest sent if none selected), local only |
| `Ctrl+E` in **Chat** / **DM** / **Group chat** | Edit your selected sent row (newest sent if none selected), local only |

`Ctrl+R`, `Ctrl+Y`, `Ctrl+C`, `Ctrl+O`, `Delete` and `Ctrl+E` all act on the
selected row (newest if none selected). On a DM tab that row is always from the
bottom DM transcript; the top broadcast pane keeps its hover-based bare scroll.
`Ctrl+P` is deliberately outside this slice — it stays newest/sender-targeted
(newest known sender on Chat/Group, the DM partner on a DM tab), so it never
follows the cursor. A selected anchor that goes stale (its row is trimmed or
deleted locally) makes the verbs fall back to the newest row rather than aiming
at whatever inherited the old index.

### Reply / quote

`Ctrl+R` captures the selected row (newest if none selected) that has a stable
wire id into a draft for the current conversation. If a cursor is armed, that
row must itself carry an id — no older row is substituted. The input title shows
the quoted line, and the next message you send carries it as an immutable
snapshot (`sender` + short preview) alongside the source id. The quote renders
as a `↳` header above the body in all three frontends, even if the original row
was later deleted locally. `Esc` clears the active conversation's draft without
sending; a second `Esc` clears that conversation's message cursor. Switching
tabs keeps drafts and cursors separate — each conversation has its own.

### Reactions

`Ctrl+Y` opens a prompt to react to the selected row (newest if none selected)
with a stable wire id in the current conversation. Press `1`–`6` for the quick
palette (`👍`, `❤️`, `😂`, `🎉`, `😮`, `😢`), or type any valid token and press
`Enter` for a custom one; applying the same emoji again removes it. The palette
is Rust-owned and shared with Flutter's reaction menu. The reaction is stored
locally and fanned out to the message's recipients over the swarm, so it appears
as a chip after the message body in the TUI and Flutter. `Esc` cancels the
prompt; a second `Esc` clears the active conversation's cursor.
`Ctrl+Up`/`Ctrl+Down` can now target any row with a wire id; Flutter's
per-message chips still target any message.

### Clipboard

`Ctrl+C` copies the full rendered line of the selected row (newest if none
selected) in the current conversation (time, sender tag and body, including
continuation lines) to the terminal clipboard via OSC 52. Raw mode means this
arrives as a key event, not SIGINT. The sequence is emitted after the frame, so
it never corrupts the screen. OSC 52 support depends on the terminal (kitty,
iTerm2, WezTerm, Alacritty, foot and modern xterm work; tmux needs clipboard
passthrough enabled). This is the TUI's counterpart to Flutter's per-message
copy action; with `Ctrl+Up`/`Ctrl+Down` it can target any row.
On the **Log** tab the same key copies every buffered log line joined with
newlines, matching Flutter's "Copy All"; an empty buffer is a no-op.

### Local message deletion

`Delete` targets the selected row (newest sent if none selected) and opens a
confirmation when that row is one you sent. `y` / `Enter` confirms; `Esc`, `n`,
or any other key cancels. Confirming writes a local tombstone and removes the
line from this install only — no retraction is sent, so other peers keep their
copy. Incoming rows cannot be deleted even when selected; if no sent row is
targeted, a short explanation appears instead.

The delete prompt sits below the device-binding and group-leave prompts but
above the call overlay and ordinary popup in the input precedence chain.

### Local message editing

`Ctrl+E` targets the selected row (newest sent if none selected) and opens a
single-line editor pre-filled with that row's stored body (display-only
decorations such as file chips and the edit marker are not included). Enter
saves, Esc cancels. Saving replaces the body in this install's history, stamps
`edited_at` and appends the canonical `(edited)` marker after the rendered body;
the line keeps its timestamp, sender tag and quoted header, and the marker
survives a reload. Peers are not notified, so they keep the original text.
Incoming rows cannot be edited even when selected; if no sent row is targeted,
a short explanation appears instead. A bare `e` still types normally in the
composer.

## Contacts tab

The contact roster is a sortable table (columns: Name, DMs, Broadcasts, Last
seen, First seen). A contact with several bound devices is one row with
expandable device sub-rows.

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Move the selection |
| `PgUp` / `PgDn` | Page the selection |
| `Home` / `End` | Jump to the first / last row |
| `Enter` | Open a DM with the selected contact or device |
| `i` / `Ctrl+P` | Open the selected row's Peer Info |
| `e` | Expand / collapse the selected contact's device rows |
| `c` | Call the selected contact's best device |
| `d` | Send this install's device-binding intro to the selected device (see [Sending a device intro](#sending-a-device-intro)) |
| `/` | Open the search box |
| `Esc` | Clear the search box (when open), otherwise return to the Chat tab |
| `Click` a row | Open a DM with that contact or device |
| `Click` a column header | Sort by that column; click again to toggle the order |

### Sorting

| Key | Sort by |
|-----|---------|
| `1` / `n` / `N` | Name |
| `2` / `m` / `M` | DM count |
| `3` / `b` / `B` | Broadcast count |
| `4` / `l` / `L` | Last seen |
| `5` / `f` / `F` | First seen |
| `o` / `O` | Toggle ascending / descending |

### Searching

`/` opens a search box above the table. While it is open it owns every
keystroke — typing `c`, `e`, or `i` edits the query instead of triggering the
corresponding action.

| Key | Action |
|-----|--------|
| `Enter` | Keep the query and close the box |
| `Esc` | Clear the query and close the box |
| `Backspace` | Delete a character |

The filter matches, case-insensitively, the contact id, the contact display
name, and every device's peer id and display name — so a contact stays
findable by one of its machines' peer ids.

## Groups tab

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Move the selection |
| `Enter` | Open the selected group's chat |
| `g` | Open the create-or-join group prompt |
| `d` | Arm the leave-group confirmation for the selected group |
| `y` / `Y` / `Enter` | (in the leave prompt) Confirm the leave |
| `Esc` | (in the leave prompt) Cancel the leave |
| `Click` a group | Open that group's chat |

Leaving a group deletes the local rows (messages, membership, the group) and
unsubscribes from its topic. It is local-only — other members keep their copy
of the history.

## Peer Info tab

| Key | Action |
|-----|--------|
| `Enter` | Open a DM with that peer |
| `n` | Edit the **local** nickname for that peer (the name only this install shows) |
| `c` | Call that peer |
| `d` | Send this install's device-binding intro to that peer (see [Sending a device intro](#sending-a-device-intro)) |
| `b` | Block / unblock that peer (mute: its traffic is dropped at the network boundary) |
| `Esc` | Return to the Chat tab |

## Log tab

| Key/Action | Behavior |
|------------|----------|
| `PgUp` / `PgDn` | Page the log lines |
| `Home` / `End` | Jump to the first / last line |
| `Ctrl+C` | Copy all log lines to the clipboard (same action as Flutter's Log tab "Copy All") |
| `c` | Clear the in-memory log buffer (same action as Flutter's Log tab "Clear") |
| `Mouse wheel` | Scroll the log lines |

## DM / Direct tab

| Key | Action |
|-----|--------|
| `Enter` | Send the message |
| `Shift+Enter` | Insert a newline |
| `Ctrl+I` | Open the DM partner's Peer Info — **kitty-keyboard terminals only** (WezTerm) |
| `Ctrl+P` | Open the DM partner's Peer Info — **works on every terminal** |
| `Ctrl+Up` / `Ctrl+Down` | Move the DM message cursor (bottom pane only, clamped at both ends) |
| `Ctrl+R` | Quote the selected DM (newest if none selected); Esc clears this DM's draft |
| `Ctrl+Y` | React to the selected DM (newest if none selected) |
| `Ctrl+C` | Copy the selected DM's rendered line (newest if none selected) |
| `Delete` | Hide your selected sent DM (newest sent if none selected), local only |
| `Ctrl+E` | Edit your selected sent DM (newest sent if none selected), local only |
| `Ctrl+L` | Export this exact device's location track to GPX after a Rust-parsed `from;to` filter |

`Ctrl+L` captures the DM partner's peer id when the prompt opens, so switching
tabs cannot mix two devices of the same account. Empty input or `all` exports
the full stored history; `24 hours ago;now` uses the same Rust time grammar as
Flutter. The GPX and a plain-text map URL are shown in a notice — terminal
hyperlinks are deliberately not used. `Esc` closes the prompt without writing.

`Ctrl+I` collapses to the same byte as `Tab` on terminals without the kitty
keyboard protocol (Konsole), so bare `Tab` would stop cycling tabs. `Ctrl+P`
is the universal fallback; use it if `Ctrl+I` does nothing. It stays
newest/partner-targeted by design and is outside the message-selection slice.
The `Ctrl+Up`/`Ctrl+Down` cursor targets only the bottom DM transcript — bare
arrows and the wheel still scroll whichever pane the pointer hovers (top
broadcast or bottom DM) and never move the cursor.

## Settings tab

| Key | Action |
|-----|--------|
| `n` | Edit your own nickname |
| `Ctrl+P` | Open the first connected peer's Peer Info (or the first bound device if none are connected) |
| `Click` a connected-peer or bound-device row | Open that peer's Peer Info |
| `d` | Open the "Dial peer" prompt — type a multiaddr (`/ip4/1.2.3.4/tcp/4001/p2p/<peer-id>`) and press `Enter` to dial it |
| `b` | Add a bootstrap multiaddr to the persisted network-peer list |
| `r` | Add a relay multiaddr (must end in `/p2p/<peer-id>`) |
| `x` | Remove the highlighted network peer |
| `↑` / `↓`, `Home` / `End` | Move the network-peer highlight |
| `l` | Toggle LAN-only mode (applies on next node start) |
| `t` | Toggle dark / light theme |
| `c` | Cycle the accent color |
| `s` | Toggle the notification sound |
| `o` | Toggle desktop notifications (OSC 777 on supported terminals) |

Connected-peer and bound-device rows are ordered by peer ID, and each opens that
peer's Peer Info even when its display name wraps onto several lines. When nobody
is connected, the `Last connected` line is informational — it does not open a
stale peer's info. `Ctrl+P` uses the same ordering: the first connected peer,
else the first bound device.

Incoming messages fire a notification only when their conversation is not the
active tab; the payload and the two delivery flags are Rust-owned and shared
with Flutter. The theme is a single persisted choice shared with Flutter; Rust
owns the palette, so both frontends always agree on the valid accents. The dial prompt
uses the shared input box; `Esc` cancels it without dialing.
The same box serves `b`/`r`; saved network peers are only read when the node
starts, so an added or removed entry applies on the next start. Outside the
add prompt these keys act immediately, so `b`/`r`/`x` cannot be typed into a
message while the Settings tab is active.

## Shares tab

A selectable list of this install's shares on the left and the highlighted
share's details on the right: owner, local root, revision, visibility,
private-share grants, manifest files and the connected peers available for
network actions. The latest `share_offered` / `share_updated` /
`share_rejected` event appears as a "Last event" line. Locally owned shares can
be toggled between private/public and can allow anonymous writes while public;
the canonical store refuses changes to accepted peer shares. Grants are
editable here too, and connected peers are ordered by peer ID. Creation,
binding, snapshots, discovery and publish actions remain Flutter/web actions.

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move the share selection |
| `←` / `→` | Move the connected-peer selection for network actions |
| `Ctrl+Up` / `Ctrl+Down` | Move the grant cursor in the detail pane |
| `PgUp` / `PgDn` | Page the share selection |
| `Home` / `End` | Jump to the first / last share |
| `Click` in the list pane | Select that share (the detail pane follows) |
| `a` | Add or upgrade a grant on the selected private share |
| `x` | Revoke the highlighted grant |
| `p` | Toggle the selected owned share between private and public |
| `w` | Toggle anonymous writes on the selected owned public share |
| `m` | Ask the selected peer for the share's current manifest |
| `o` | Re-advertise the share to the selected peer with a Rust-built offer |
| `r` | Re-read the share list from Rust |
| `Esc` | Return to the Chat tab |

The add prompt takes a verified user id. `Tab` switches between `readonly`
and `readwrite`, `Enter` saves through Rust's canonical grant store, and `Esc`
cancels. The revoke prompt is a confirmation: its prefilled id cannot be
edited, and `Enter` removes the highlighted user's grant. Both keep the
prompt open with an error popup if the store rejects the change. Changing the
share resets the grant cursor, so `x` cannot retarget after a wheel or click.

`m` and `o` target the highlighted connected peer; both require a selected
share and fail with a popup when no peer is connected. Re-advertising uses the
same Rust offer builder as the mobile/web binding, so no local-only owner marker
is put on the wire. Clicking the detail pane does not change the selection. `r`
is tab-specific: on Settings it adds a relay, on Shares it refreshes.
Share events and grant commits also refresh the list automatically, so `r` is
only needed after an action whose result arrives asynchronously.

## Input and editing

| Key | Action |
|-----|--------|
| `Enter` | Send the message / save a local message edit / save the nickname / confirm the group or share-grant prompt |
| `Shift+Enter` | New line in a multi-line message |
| `Esc` | Cancel the edit in progress, or unwind one layer (see the chain below) |
| `/file <path>` | Attach a local file and send it (see below) |

### Attaching a file

Type `/file /path/to/file.pdf` in the chat input and press `Enter`. The file
is hashed (SHA-256), stored content-addressed, and sent as a reference; the
message carries a descriptor and the bytes travel over the separate file
lane. Both frontends render the result as a chip:
`[file] report.pdf (1 MiB)` — the size label comes from Rust's canonical
`files::format_file_size`, so both frontends always agree.

### Opening a received file

`Ctrl+O` in **Chat** / **DM** / **Group chat** opens the attachment on the
selected row (newest row with an attachment if none selected) with the system
default app (`xdg-open` on Linux, `open` on macOS, `start` on Windows) — the TUI
equivalent of tapping the chip in Flutter. With a cursor armed, only that row
qualifies: an unrelated file is never opened. It resolves the descriptor from
the persisted message row, so a file must have finished downloading and the row
must still exist; otherwise a short "No downloaded attachment" popup appears. A
failed launch reports the error in the same popup.

## UI control

| Key | Action |
|-----|--------|
| `?` | Open the compact shortcut-help overlay |
| `F12` | Toggle mouse capture |
| `Esc` | Cancel one active layer — see the precedence below |
| `Ctrl+Q` | Exit the application |

`?` is a plain key, so it opens the help overlay only when no input box owns
the keyboard. It does nothing while an editor (including a share-grant prompt),
the Contacts search, a dial or network-peer prompt, the
device-binding/group-leave prompt, or a call overlay is active. The overlay
uses the ordinary popup slot: press any key or click to dismiss it, which is
why typing `?` inside a prompt still inserts a question mark.

`Esc` unwinds one level at a time. The single-slot prompts own it first —
decline a device-binding request, cancel a group leave, or cancel a local
message deletion — followed by a live call overlay. Otherwise it cancels the
innermost active layer: dismiss the popup → cancel a local message edit →
cancel a reaction → cancel a share-grant edit → clear the Contacts search →
cancel a nickname edit → cancel the group create/join prompt → cancel a peer
dial → cancel network-peer entry → clear the active conversation's reply draft
→ clear that conversation's message selection → return to the Chat tab. A second `Esc` is needed for the
next layer; for example, dismissing a network-peer validation popup leaves the
rejected address open for correction.

### Device-binding prompt

When a peer asks to be bound into this install's user, a prompt window
appears. It takes priority over every other input.

| Key | Action |
|-----|--------|
| `a` / `A` | Accept — mint the signed claim and send it |
| any other key | Decline |

One prompt at a time: a second request arriving while one is active is
dropped (and logged).

### Sending a device intro

To add this install as a second device of an account, press `d` on the
**Contacts** tab (with the account holder's device selected) or on a
**Peer Info** tab. That sends this install's signed device-binding intro to the
selected peer; their account holder can then accept it with `a`. This is the
same "send my identity" action Flutter exposes on each contact's device row and
the web client exposes on its contact card. The recipient must already be in
your roster — run this on the *new* device, targeting the account holder.

## Calls

| Key | Action |
|-----|--------|
| `c` | On the **Contacts** tab: call the selected contact's best device. On a **Peer Info** tab: call that peer |
| `a` / `A` | (ringing overlay) Accept the incoming call |
| any other key | (ringing overlay) Decline the incoming call |
| `c` / `C` / `Esc` | (calling overlay) Cancel the outgoing call |
| `Esc` | (in-call overlay) Hang up |
| `m` / `M` | (in-call overlay) Toggle the microphone mute |

One call at a time: a live call overlay swallows all other input until it
resolves, and a second incoming invitation is auto-declined while one is
live.

Call signaling and the μ-law **media transport** are implemented (Rust owns the
session; the in-call overlay shows mic state and the decoded-frame count).
No device-level microphone/speaker adapter ships yet. See
[docs/backlog.md](docs/backlog.md) (US-044).

## DM tab layout

```
┌─ Broadcast from peer (top 50%)
│  └─ Click to open the sender's Peer Info
│  └─ Hover & scroll to navigate broadcast history
│
├─ Divider
│
└─ DM: peer (bottom 50%)
   └─ Your conversation with this peer
   └─ Hover & scroll to navigate DM history
```

---

**Pro tips**

- Press `?` for the compact table; this file remains the full reference.
- `End` returns you to live mode when catching up on messages.
- `Ctrl+Up`/`Ctrl+Down` arms the active conversation's message cursor; `Esc` clears that cursor before taking you back to Chat.
- `Home` takes you to the very first message — that can be a long way back.
- On a DM tab the pointer picks the pane that scrolls: hover the top half for
  broadcasts, the bottom half for the conversation.
- Clicking any peer message opens their Peer Info; `Enter` there opens the DM.
- The Peer Info shortcut differs by terminal encoding — use `Ctrl+P` if
  `Ctrl+I` does nothing.
