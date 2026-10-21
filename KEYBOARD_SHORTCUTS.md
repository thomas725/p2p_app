# Keyboard & Mouse Shortcuts

Every binding in the TUI. Where a binding only exists on one tab, the table
says so.

The in-app shortcut bar at the bottom of the screen shows the active
bindings for the current tab; this file is the complete reference.

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
| `Esc` | Return to the Chat tab |

## DM / Direct tab

| Key | Action |
|-----|--------|
| `Enter` | Send the message |
| `Shift+Enter` | Insert a newline |
| `Ctrl+I` | Open the DM partner's Peer Info — **kitty-keyboard terminals only** (WezTerm) |
| `Ctrl+P` | Open the DM partner's Peer Info — **works on every terminal** |

`Ctrl+I` collapses to the same byte as `Tab` on terminals without the kitty
keyboard protocol (Konsole), so bare `Tab` would stop cycling tabs. `Ctrl+P`
is the universal fallback; use it if `Ctrl+I` does nothing.

## Settings tab

| Key | Action |
|-----|--------|
| `n` | Edit your own nickname |

## Input and editing

| Key | Action |
|-----|--------|
| `Enter` | Send the message / save the nickname / confirm the group prompt |
| `Shift+Enter` | New line in a multi-line message |
| `Esc` | Cancel the edit in progress, or return to the Chat tab |
| `/file <path>` | Attach a local file and send it (see below) |

### Attaching a file

Type `/file /path/to/file.pdf` in the chat input and press `Enter`. The file
is hashed (SHA-256), stored content-addressed, and sent as a reference; the
message carries a descriptor and the bytes travel over the separate file
lane. Both frontends render the result as a chip:
`[file] report.pdf (1048576)`.

## UI control

| Key | Action |
|-----|--------|
| `F12` | Toggle mouse capture |
| `Esc` | Dismiss the popup / decline the device-binding prompt / cancel the current edit / return to the Chat tab |
| `Ctrl+Q` | Exit the application |

`Esc` unwinds one level at a time, in this order: dismiss the popup →
decline the device-binding prompt → cancel a group leave → clear the contacts
search → cancel a nickname edit → cancel the group create/join prompt →
return to the Chat tab.

### Device-binding prompt

When a peer asks to be bound into this install's user, a prompt window
appears. It takes priority over every other input.

| Key | Action |
|-----|--------|
| `a` / `A` | Accept — mint the signed claim and send it |
| any other key | Decline |

One prompt at a time: a second request arriving while one is active is
dropped (and logged).

## Calls

| Key | Action |
|-----|--------|
| `c` | On the **Contacts** tab: call the selected contact's best device. On a **Peer Info** tab: call that peer |
| `a` / `A` | (ringing overlay) Accept the incoming call |
| any other key | (ringing overlay) Decline the incoming call |
| `c` / `C` / `Esc` | (calling overlay) Cancel the outgoing call |
| `Esc` | (in-call overlay) Hang up |

One call at a time: a live call overlay swallows all other input until it
resolves, and a second incoming invitation is auto-declined while one is
live.

Call **signaling** is implemented; audio/video media is not. See
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

- `End` returns you to live mode when catching up on messages.
- `Home` takes you to the very first message — that can be a long way back.
- On a DM tab the pointer picks the pane that scrolls: hover the top half for
  broadcasts, the bottom half for the conversation.
- Clicking any peer message opens their Peer Info; `Enter` there opens the DM.
- The Peer Info shortcut differs by terminal encoding — use `Ctrl+P` if
  `Ctrl+I` does nothing.
