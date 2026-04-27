# Keyboard & Mouse Shortcuts

## Tab Navigation

| Key | Action |
|-----|--------|
| `Tab` | Next tab |
| `Shift+Tab` / `BackTab` | Previous tab |
| `Mouse Click` on tab | Jump to tab |
| `X` button on tab | Close DM tab |

## Message Scrolling (Broadcast & DM Tabs)

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Scroll DM section up/down one line |
| `PgUp` / `PgDn` | Scroll DM section one page (5 lines) |
| `Home` | Jump to first DM message |
| `End` | Jump to newest DM (auto-scroll) |
| `Mouse Wheel Up/Down` | **Hover-based:** Scroll whichever section mouse is over |

### Mouse Hovering in DM Tab:
- **Top half (broadcast section):** Mouse wheel scrolls broadcast messages
- **Bottom half (DM section):** Mouse wheel scrolls direct messages
- Both sections maintain independent scroll position
- Hover naturally scrolls the section you're reading

## Message Interaction

| Action | Result |
|--------|--------|
| `Click` on message in Broadcast tab | Open DM with sender |
| `Click` on message in DM tab broadcast section | Open DM with sender |
| `Click` on message in DM tab DM section | (Interaction logged) |
| `Click` on own message | Edit nickname |

## Input & Editing

| Key | Action |
|-----|--------|
| `Enter` | Send message / Save nickname edit |
| `Shift+Enter` | New line in multi-line message |
| `Ctrl+W` | Close current DM tab |

## Peers Tab

| Key/Action | Behavior |
|------------|----------|
| `↑` / `↓` | Navigate peer list |
| `Enter` | Open DM with selected peer |
| `Click` on peer | Open DM with that peer |

## UI Control

| Key | Action |
|-----|--------|
| `F12` | Toggle mouse capture |
| `Esc` | Cancel nickname edit / Exit |
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
- Click any peer message in Broadcast to instantly start a conversation
