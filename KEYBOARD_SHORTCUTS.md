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
| `↑` / `↓` | Scroll up/down one line |
| `PgUp` / `PgDn` | Scroll one page (5 lines) |
| `Home` | Jump to oldest message |
| `End` | Jump to newest message (auto-scroll) |
| `Mouse Wheel Up/Down` | Scroll 3 lines |

### Note on DM Tab:
- Scrolling targets the **DM section** (bottom half)
- Broadcast messages (top) always show newest

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
│  └─ Click to start DM with sender
│
├─ Divider (auto-scrolls with newest messages)
│
└─ DM: peer (bottom 50%)
   └─ Your conversation with this peer
   └─ Keyboard/mouse scrolling targets this section
```

---

**Pro Tips:**
- Use `End` to return to live mode when catching up on messages
- `Home` takes you to the very first message (warning: can be far back!)
- In DM tabs, all scrolling targets the conversation (bottom), keeping it focused
- Click any peer message in Broadcast to instantly start a conversation
