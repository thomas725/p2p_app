// Notification delivery adapter (US-027 sounds, US-028 system notifications).
//
// Rust owns the policy and the display text: incoming message events carry a
// ready-to-show [MessageNotification]. This file is the *only* place Dart
// decides whether to deliver it (focus suppression) and how (sound / system
// notification per platform). Screens just publish which conversation is on
// screen; they never touch a platform channel themselves.
//
// Both platform calls sit behind injectable seams so widget tests exercise the
// real focus/suppression logic without a MethodChannel or a browser. The
// browser-only JS primitives live in `notifications_platform_web.dart`, selected
// by a conditional import so this file compiles on the native VM too.
library;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/rust/notifications.dart' as api;
import 'ui_utils.dart' show isAndroid;
import 'notifications_platform.dart'
    if (dart.library.js_interop) 'notifications_platform_web.dart';

/// Alias so callers read naturally while avoiding the browser `Notification`
/// name that the generated Rust type would otherwise shadow.
typedef MessageNotification = api.Notification;

const MethodChannel _channel = MethodChannel(
  'com.example.p2p_app_flutter/notifications',
);

/// Injectable sound delivery. Replaced in tests.
typedef SoundDelivery = Future<void> Function();

/// Injectable system-notification delivery. Replaced in tests.
typedef SystemDelivery = Future<void> Function({
  required String title,
  required String body,
});

SoundDelivery soundDelivery = _deliverSound;
SystemDelivery systemDelivery = _deliverSystem;

/// The conversation currently visible (`broadcast`, `dm:<peer-id>`,
/// `group:<group-id>`), or null when no chat is on screen. Set by the chat
/// screens and reset on dispose.
String? _focusedConversation;

/// Whether the app is in the foreground. Background apps must notify even for
/// the previously-focused conversation.
bool _inForeground = true;

/// Publish the conversation currently on screen. `null` clears the focus.
void setFocusedConversation(String? key) => _focusedConversation = key;

/// Take over focus while a chat screen is open, returning the key it replaced.
/// The screen must restore it in `dispose` (mirrors the event-sink push/pop).
String? pushFocusedConversation(String key) {
  final previous = _focusedConversation;
  _focusedConversation = key;
  return previous;
}

/// Track foreground state so background messages notify even for the last
/// focused conversation (US-028).
void setAppInForeground(bool value) => _inForeground = value;

/// Deliver a Rust-built notification, applying the single suppression rule:
/// skip when the app is foregrounded *and* that exact conversation is visible.
/// A transport/permission failure is swallowed — notifications are best effort
/// and must never break the message pump.
Future<void> deliverNotification(MessageNotification? notification) async {
  final n = notification;
  if (n == null) return;
  if (_inForeground && _focusedConversation == n.conversationKey) return;

  if (n.sound) {
    try {
      await soundDelivery();
    } catch (e) {
      // ignore: avoid_print
      print('notification sound failed: $e');
    }
  }
  if (n.desktop) {
    try {
      await systemDelivery(title: n.title, body: n.body);
    } catch (e) {
      // ignore: avoid_print
      print('system notification failed: $e');
    }
  }
}

// --- Default platform adapters ---

Future<void> _deliverSound() async {
  if (kIsWeb) {
    await platformBeep();
  } else if (isAndroid) {
    await _channel.invokeMethod<void>('playSound');
  }
  // Native desktop builds have no platform audio adapter; the browser frontend
  // is the primary desktop path and delivers sound via notifications_platform_web.
}

Future<void> _deliverSystem({
  required String title,
  required String body,
}) async {
  if (kIsWeb) {
    await platformShowNotification(title, body);
  } else if (isAndroid) {
    await _channel.invokeMethod<void>('show', {'title': title, 'body': body});
  }
}

/// Ask the platform for permission to show system notifications. Called when
/// the user enables the desktop channel. Native Android's `POST_NOTIFICATIONS`
/// grant is handled when the foreground service is created; the web browser
/// prompt is triggered here.
Future<void> requestNotificationPermission() => platformRequestPermission();
