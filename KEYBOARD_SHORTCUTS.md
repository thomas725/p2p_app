# Keyboard & Mouse Shortcuts

## Tab Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next tab |
| `Shift+Tab` / `BackTab` | Previous tab |
| `Mouse Click` on tab | Jump to tab |
| `X` button on tab | Close DM or Peer Info tab |
| `Ctrl+W` | Close current DM or Peer Info tab |

## Message Scrolling (Broadcast & DM Tabs)

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | **Hover-based:** Scroll whichever section mouse is over one line |
| `PgUp` / `PgDn` | **Hover-based:** Page-scroll whichever section mouse is over |
| `Home` | **Hover-based:** Jump to first message in hovered section |
| `End` | **Hover-based:** Jump to newest in hovered section (auto-scroll) |
| `Mouse Wheel Up/Down` | **Hover-based:** Scroll whichever section mouse is over |

### Hover-Based Scrolling in DM Tab:
- Keyboard & mouse both respect hover position
- Hover over **top half (broadcast)** + scroll → scrolls broadcast messages
- Hover over **bottom half (DM)** + scroll → scrolls DM messages
- Works with: arrow keys, Page Up/Down, Home, End, mouse wheel
- Both sections maintain independent scroll position and auto-scroll state

### Unread Broadcast Banner
- When broadcasts arrive while you are scrolled up, a cyan `▼ N new message(s) — End: jump to latest` banner appears at the bottom of the Broadcast Chat.
- Press `End` (or scroll to the bottom) to jump to the newest message; the count clears automatically.

## Message Interaction

| Action | Result |
|--------|--------|
| `Click` on a peer's message in Broadcast tab | Open that sender's Peer Info |
| `Click` on a message in the DM tab's broadcast section | Open the sender's Peer Info |
| `Click` on a message in the DM tab's DM section | Open the partner's Peer Info |
| `Click` on a message in the Log tab | Open the sender's Peer Info |

## Peer Info Tab

| Key | Action |
|-----|--------|
| `Ctrl+I` (kitty terminals, e.g. WezTerm) | Open the DM partner's Peer Info on a Direct tab |
| `Ctrl+P` (non-kitty terminals) | Open the DM partner's Peer Info on a Direct tab |
| `i` | On the **Peers** tab: open the selected peer's Peer Info |
| `Enter` | Open a DM with that peer |
| `Esc` | Return to Broadcast Chat |

## Input & Editing

| Key | Action |
|-----|--------|
| `Enter` | Send message / Save nickname edit |
| `Shift+Enter` | New line in multi-line message |
| `n` | (Settings tab) Edit your nickname |

## Peers Tab

The peer list is a sortable table (columns: Name, DM count, Broadcast count, Last seen).

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Navigate peer list |
| `PgUp` / `PgDn` | Page the peer list up / down |
| `Home` / `End` | Jump to first / last peer |
| `Enter` | Open DM with selected peer |
| `i` | Open the selected peer's Peer Info |
| `Click` on peer | Open DM with that peer |
| `Click` on a column header | Sort by that column (click again to toggle order) |
| `1` / `n` | Sort by Name |
| `2` / `m` | Sort by DM count |
| `3` / `b` | Sort by Broadcast count |
| `4` / `l` | Sort by Last seen |
| `o` | Toggle ascending / descending order |

## UI Control

| Key | Action |
|-----|--------|
| `F12` | Toggle mouse capture |
| `Esc` | Dismiss popup / cancel nickname edit / return to Broadcast Chat |
| `Ctrl+Q` | Exit application |

## DM Tab Layout

```
┌─ Broadcast from peer (top 50%)
│  └─ Click to switch to Broadcast tab and show message
│  └─ Hover & scroll to navigate broadcast history
│
├─ Divider
│
└─ DM: peer (bottom 50%)
   └─ Your conversation with this peer
   └─ Hover & scroll to navigate DM history
   └─ Keyboard shortcuts target DM section
```

---

**Pro Tips:**
- Use `End` to return to live mode when catching up on messages
- `Home` takes you to the very first message (warning: can be far back!)
- In DM tabs, all scrolling targets the conversation (bottom), keeping it focused
- Click any peer message in Broadcast to open their Peer Info (then `Enter` to open a DM)
- The Peer Info shortcut differs by terminal encoding — see the Peer Info Tab section for which applies to you
