# Notification sounds + desktop alerts (2026-11-21)

> **ARCHIVED** — a historical record of completed work, kept for context.
> It is not maintained and may describe behaviour that no longer exists.
> Current documentation: [docs/architecture.md](../architecture.md),
> [docs/README.md](../README.md).

This closes **US-027** (notification sounds) and **US-028** (desktop/system
notifications). Rust owns the entire policy: it decides *which* events notify,
builds the display text, and carries two delivery flags on the event envelope.
Each frontend is a thin adapter that only decides whether the conversation is
on screen and how to hand the payload to the platform. That keeps the "new
message → notification" rule identical in the TUI, Android and the browser.

## Canonical model

`src/notifications.rs` owns everything:

- **`Notification`** is the payload attached to every incoming broadcast, DM
  and group event as `SwarmEventJson.notification: Option<Notification>`: a
  display `title` (sender, or `Group <id> · <sender>`), a single-line `body`
  preview (control characters flattened, bounded to 140 chars), a stable
  `conversationKey` (`broadcast`, `dm:<peer-id>` or `group:<group-id>`), and the
  two booleans `sound` / `desktop`. `for_message` returns `None` for control and
  call events and when both channels are off, so no frontend ever re-derives a
  display name or a preview.
- **`NotificationSettings`** is the persisted pair of channels. Sound defaults
  to `true` (no permission needed); desktop defaults to `false` because the
  browser and Android 13+ both require a one-time permission grant.
- **`terminal_sequence`** encodes one notification for the TUI: an optional bell
  followed by an OSC 777 `notify` sequence. Titles/bodies are sanitized so a
  `;` or a control character can never inject a second OSC command.

## Persistence and transport

The channels reuse the generic `app_settings` table: `src/settings.rs` adds
typed `notification_sound()` / `notification_desktop()` readers and setters;
`notifications::set_settings` writes both in one call so a DB error surfaces
instead of leaving a half-applied choice. FRB exposes
`getNotificationSettings` / `setNotificationSettings`, and the web JSON
dispatch mirrors both names with drift guards. The Dart side reaches them
through the shared `NodeApi` transport (in-process FRB or HTTP/WS), with a fake
in the widget tests.

## Focus rule (the only frontend policy)

Deliver unless the app is foregrounded **and** the notification's conversation
key equals the one currently on screen. A background app notifies even for the
last-focused conversation. This single rule lives in the Rust event helpers for
the TUI and in `lib/src/util/notifications.dart` for Flutter; screens just
publish which conversation is visible (`pushFocusedConversation` / dispose) and
never touch a platform channel.

## Frontend adapters

- **TUI**: the command processor queues notifications on incoming messages,
  skipping `active_conversation_key()`'s match. The render loop drains the queue
  after the frame is drawn and writes the bell / OSC sequence to a single locked
  stdout handle, so nothing interleaves with ratatui's drawing. Settings shows
  both channels; `s` toggles sound and `o` toggles desktop notifications.
- **Flutter/web**: `startEventPolling` hands `event.notification` to the adapter
  before the screen sink; a failure is swallowed so the message pump continues.
  Sound is a WebAudio beep (web) or a MethodChannel call (Android); a system
  notification is the browser `Notification` API (web) or a MethodChannel call
  (Android). The browser-only `dart:js_interop` code lives in
  `notifications_platform_web.dart`, selected by a conditional import so the
  adapter compiles on the native VM. Enabling the desktop channel prompts for
  browser permission; lifecycle callbacks track foreground state. The Settings
  card has two switches wired through the transport.
- **Android**: `MessageNotifications.kt` implements the two MethodChannel
  methods (`playSound`, `show(title, body)`) on a high-importance message
  channel, pinned by a unit test against the Dart-side channel/method strings.

## Tests

Rust unit tests cover preview flattening/bounding, conversation-key mapping,
control-event suppression and terminal-sequence encoding. Dart unit tests
exercise the focus rule, flag fan-out, null payloads, channel failure isolation
and focus push/restore through injectable seams (no platform channel). A widget
test toggles both Settings switches through the fake transport; a TUI state test
covers `active_conversation_key`; a Kotlin test pins the channel contract.
