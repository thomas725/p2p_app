> **ARCHIVED** — historical record, not current behaviour.

# Flutter message-body taps open Peer Info (2026-11-28)

The TUI already opened the sender's (or DM partner's) Peer Info when a message
row was clicked; Flutter only did so from the small sender-name label. This
slice made the whole bubble body tappable in the shared Flutter chat UI while
keeping double-tap selection and every nested control intact.

## What shipped

- `MessageChatBody` accepts `onBubbleBodyTap`, passes the tapped `ChatMessage`
  to its host, and suppresses that action in selection mode. `BroadcastChat`
  forwards it from `HomeScreen`; `DmChatScreen` and `GroupChatScreen` supply
  their own screen-level handlers.
- `MessageBubble` uses `onBodyTap ?? onTap` for the outer gesture. Sender
  names, attachment chips, map links and reaction chips keep their own nested
  taps, so a tap on one of them never also opens Peer Info.
- Peer resolution stays at the screen boundary: broadcast and group rows use
  the signed sender in `message.peerId` (own rows have none); a DM always uses
  `widget.peer.peerId`, even when the stored row carries a different sender id.
- The lone-tap action fires after the double-tap window; a completed double
  tap only enters selection. No extra debounce was needed.

## Tests

`messages_test.dart` covers the shared body/selection callbacks, real lone and
double taps on a bare bubble, sender/attachment/map-link precedence, the
broadcast sender and own-row no-op, a DM row with a stale sender id, and group
sender resolution. `flutter analyze` and the full `flutter test` suite pass.
