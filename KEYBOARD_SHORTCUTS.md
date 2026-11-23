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
Peer Info tabs, **Log**, **Settings**. Peer Info and DM tabs are transient —
they are inserted, deduplicated by peer, and closed with `Ctrl+W`.

## Scrolling

Scrolling is **hover-based**: the section under the mouse pointer is the one
that scrolls. If the pointer has never been over the content, the focused
section is used.

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Scroll the hovered section one line |
| `PgUp` / `PgDn` | Page-scroll the hovered section |
| `Home` | Jump to the first message in the hovered section |
| `End` | Jump to the newest message in the hovered section (re-enters auto-scroll) |
| `Mouse wheel` | Scroll the hovered section |

Which section that is, per tab:

| Tab | Sections |
|-----|----------|
| Chat | broadcast messages |
| DM / Direct | broadcast messages (top half) and the conversation (bottom half) — the pointer picks |
| Group chat | the group's messages |
| Log | the log lines |
| Contacts | the contact table |
| Groups | the group list |

Peer Info and Settings do not scroll.

### Unread broadcast banner

When broadcasts arrive while you are scrolled up, a cyan
`▼ N new message(s) — End: jump to latest` banner appears at the bottom of
the broadcast pane. Press `End` (or scroll to the bottom) to jump to the
newest message; the counter clears.

## Message interaction

| Action | Result |
|--------|--------|
| `Click` a message in the **Chat** tab | Open the sender's Peer Info |
| `Click` a message in the **DM** tab's broadcast pane | Open the sender's Peer Info |
| `Click` a message in the **DM** tab's conversation pane | Open the partner's Peer Info |
| `Click` a message in a **Group chat** tab | Open the sender's Peer Info |
| `Click` a line in the **Log** tab | Nothing — log lines have no sender |
| `Ctrl+R` in **Chat** / **DM** / **Group chat** | Quote the newest message in that conversation |
| `Ctrl+Y` in **Chat** / **DM** / **Group chat** | Add or remove an emoji reaction on the newest message in that conversation |
| `Ctrl+C` in **Chat** / **DM** / **Group chat** | Copy the newest message's rendered line to the terminal clipboard |
| `Ctrl+C` in **Log** | Copy all log lines to the terminal clipboard (same as Flutter's "Copy All") |
| `Ctrl+O` in **Chat** / **DM** / **Group chat** | Open the newest downloaded attachment with the system default app |
| `Esc` in **Chat** / **DM** / **Group chat** | Clear the pending reply draft |
| `Delete` in **Chat** / **DM** / **Group chat** | Hide your most recent sent message in that conversation (local only) |
| `Ctrl+E` in **Chat** / **DM** / **Group chat** | Edit your most recent sent message in that conversation (local only) |

### Reply / quote

`Ctrl+R` captures the newest message that has a stable wire id into a draft for
the current conversation. The input title shows the quoted line, and the next
message you send carries it as an immutable snapshot (`sender` + short preview)
alongside the source id. The quote renders as a `↳` header above the body in all
three frontends, even if the original row was later deleted locally. `Esc`
clears the draft without sending; switching tabs keeps drafts separate.

### Reactions

`Ctrl+Y` opens a prompt to react to the newest message with a stable wire id in
the current conversation. Type an emoji and press `Enter` to add it; applying
the same emoji again removes it. The reaction is stored locally and fanned out
to the message's recipients over the swarm, so it appears as a chip after the
message body in the TUI and Flutter. `Esc` cancels without saving. Only the
newest message can be reacted to from the TUI; Flutter's per-message chips can
target any message.

### Clipboard

`Ctrl+C` copies the full rendered line of the newest message in the current
conversation (time, sender tag and body, including continuation lines) to the
terminal clipboard via OSC 52. Raw mode means this arrives as a key event, not
SIGINT. The sequence is emitted after the frame, so it never corrupts the
screen. OSC 52 support depends on the terminal (kitty, iTerm2, WezTerm,
Alacritty, foot and modern xterm work; tmux needs clipboard passthrough
enabled). This is the TUI's counterpart to Flutter's per-message copy action —
just limited to the newest message, like the other "newest message" shortcuts.
On the **Log** tab the same key copies every buffered log line joined with
newlines, matching Flutter's "Copy All"; an empty buffer is a no-op.

### Local message deletion

`Delete` scans the open conversation backwards for the newest message you sent
and opens a confirmation. `y` / `Enter` confirms; `Esc`, `n`, or any other key
cancels. Confirming writes a local tombstone and removes the line from this
install only — no retraction is sent, so other peers keep their copy. Incoming
messages cannot be deleted; if the conversation has no sent row, a short
explanation appears instead.

The delete prompt sits below the device-binding and group-leave prompts but
above the call overlay and ordinary popup in the input precedence chain.

### Local message editing

`Ctrl+E` scans the open conversation backwards for the newest message you sent
and opens a single-line editor pre-filled with the stored body (display-only
decorations such as file chips and the edit marker are not included). Enter
saves, Esc cancels. Saving replaces the body in this install's history, stamps
`edited_at` and appends the canonical `(edited)` marker after the rendered body;
the line keeps its timestamp, sender tag and quoted header, and the marker
survives a reload. Peers are not notified, so they keep the original text.
Incoming rows cannot be edited; if there is no sent row, a short explanation
appears instead. A bare `e` still types normally in the composer.

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
| `i` | Open the selected row's Peer Info |
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
| `Ctrl+R` | Quote the newest DM in this conversation (Esc clears) |
| `Ctrl+Y` | Add or remove an emoji reaction on the newest DM in this conversation |
| `Ctrl+C` | Copy the newest DM's rendered line to the terminal clipboard (OSC 52) |
| `Delete` | Hide your most recent sent DM locally (with confirmation) |
| `Ctrl+E` | Edit your most recent sent DM locally (Enter saves, Esc cancels) |

`Ctrl+I` collapses to the same byte as `Tab` on terminals without the kitty
keyboard protocol (Konsole), so bare `Tab` would stop cycling tabs. `Ctrl+P`
is the universal fallback; use it if `Ctrl+I` does nothing.

## Settings tab

| Key | Action |
|-----|--------|
| `n` | Edit your own nickname |
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

Incoming messages fire a notification only when their conversation is not the
active tab; the payload and the two delivery flags are Rust-owned and shared
with Flutter. The theme is a single persisted choice shared with Flutter; Rust
owns the palette, so both frontends always agree on the valid accents. The dial prompt
uses the shared input box; `Esc` cancels it without dialing.
The same box serves `b`/`r`; saved network peers are only read when the node
starts, so an added or removed entry applies on the next start. Outside the
add prompt these keys act immediately, so `b`/`r`/`x` cannot be typed into a
message while the Settings tab is active.

## Input and editing

| Key | Action |
|-----|--------|
| `Enter` | Send the message / save a local message edit / save the nickname / confirm the group prompt |
| `Shift+Enter` | New line in a multi-line message |
| `Esc` | Cancel the edit in progress, or return to the Chat tab |
| `/file <path>` | Attach a local file and send it (see below) |

### Attaching a file

Type `/file /path/to/file.pdf` in the chat input and press `Enter`. The file
is hashed (SHA-256), stored content-addressed, and sent as a reference; the
message carries a descriptor and the bytes travel over the separate file
lane. Both frontends render the result as a chip:
`[file] report.pdf (1 MiB)` — the size label comes from Rust's canonical
`files::format_file_size`, so both frontends always agree.

### Opening a received file

`Ctrl+O` in **Chat** / **DM** / **Group chat** opens the newest downloaded
attachment in that conversation with the system default app (`xdg-open` on
Linux, `open` on macOS, `start` on Windows) — the TUI equivalent of tapping the
chip in Flutter. It resolves the descriptor from the persisted message row, so
a file must have finished downloading and the row must still exist; otherwise a
short "No downloaded attachment" popup appears. A failed launch reports the
error in the same popup.

## UI control

| Key | Action |
|-----|--------|
| `?` | Open the compact shortcut-help overlay |
| `F12` | Toggle mouse capture |
| `Esc` | Dismiss the popup / decline the device-binding prompt / cancel a local-delete, local edit, group-leave, or current edit / return to the Chat tab |
| `Ctrl+Q` | Exit the application |

`?` is a plain key, so it opens the help overlay only when no input box owns
the keyboard. It does nothing while an editor, the Contacts search, a dial or
network-peer prompt, the device-binding/group-leave prompt, or a call overlay
is active. The overlay uses the ordinary popup slot: press any key or click to
dismiss it, which is why typing `?` inside a prompt still inserts a question
mark.

`Esc` unwinds one level at a time, in this order: dismiss the popup →
decline the device-binding prompt → cancel a group leave → cancel a local
message deletion → cancel a local message edit → clear the contacts search →
cancel a nickname edit →
cancel the group create/join prompt → cancel a peer dial → return to the Chat
tab.

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
- `Home` takes you to the very first message — that can be a long way back.
- On a DM tab the pointer picks the pane that scrolls: hover the top half for
  broadcasts, the bottom half for the conversation.
- Clicking any peer message opens their Peer Info; `Enter` there opens the DM.
- The Peer Info shortcut differs by terminal encoding — use `Ctrl+P` if
  `Ctrl+I` does nothing.
