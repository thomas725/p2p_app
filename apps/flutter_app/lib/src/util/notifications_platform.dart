// Native stubs for the browser-only notification primitives (US-027/US-028).
//
// The web build selects `notifications_platform_web.dart` via the conditional
// import in `notifications.dart`; on native targets these no-ops are the whole
// implementation. Sound and system notifications on Android go through the
// MethodChannel in `notifications.dart`, so nothing is lost here — desktop
// Flutter is not a shipping target (the web frontend covers it).

/// No browser audio on native targets.
Future<void> platformBeep() async {}

/// No browser `Notification` API on native targets.
Future<void> platformShowNotification(String title, String body) async {}

/// No browser permission prompt on native targets (Android's POST_NOTIFICATIONS
/// grant is handled when the foreground service starts).
Future<void> platformRequestPermission() async {}
