// Unit tests for the notification delivery adapter (US-027/US-028). Rust owns
// the payload and policy; these exercise the one Dart-side rule — focus
// suppression — plus the sound/system fan-out through the injectable seams, so
// no MethodChannel or browser object is needed.

import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/notifications.dart' as api;
import 'package:p2p_app_flutter/src/util/notifications.dart' as notifications;

api.Notification notification({
  String key = 'dm:peer1',
  bool sound = true,
  bool desktop = true,
}) =>
    api.Notification(
      title: 'Alice',
      body: 'hello',
      conversationKey: key,
      sound: sound,
      desktop: desktop,
    );

void main() {
  late List<String> delivered;

  setUp(() {
    delivered = <String>[];
    notifications.soundDelivery = () async {
      delivered.add('sound');
    };
    notifications.systemDelivery = ({
      required String title,
      required String body,
    }) async {
      delivered.add('system:$title:$body');
    };
    notifications.setFocusedConversation(null);
    notifications.setAppInForeground(true);
  });

  test('delivers both channels when no conversation is focused', () async {
    await notifications.deliverNotification(notification());
    expect(delivered, ['sound', 'system:Alice:hello']);
  });

  test('suppresses only the currently focused conversation', () async {
    notifications.setFocusedConversation('dm:peer1');
    await notifications.deliverNotification(notification(key: 'dm:peer1'));
    expect(delivered, isEmpty);

    // A different conversation still notifies.
    await notifications.deliverNotification(notification(key: 'dm:peer2'));
    expect(delivered, ['sound', 'system:Alice:hello']);
  });

  test('background app notifies even for the focused conversation', () async {
    notifications.setFocusedConversation('dm:peer1');
    notifications.setAppInForeground(false);
    await notifications.deliverNotification(notification(key: 'dm:peer1'));
    expect(delivered, ['sound', 'system:Alice:hello']);
  });

  test('respects the two delivery flags independently', () async {
    await notifications.deliverNotification(notification(sound: false));
    expect(delivered, ['system:Alice:hello']);

    delivered.clear();
    await notifications.deliverNotification(notification(desktop: false));
    expect(delivered, ['sound']);
  });

  test('a null payload is a no-op', () async {
    await notifications.deliverNotification(null);
    expect(delivered, isEmpty);
  });

  test('a failing channel never breaks the other or throws', () async {
    notifications.soundDelivery = () async => throw StateError('no audio');
    // Deliver sound first; the exception is swallowed and system still runs.
    await notifications.deliverNotification(notification());
    expect(delivered, ['system:Alice:hello']);
  });

  test('pushFocusedConversation swaps focus and returns the previous key', () async {
    notifications.setFocusedConversation('broadcast');

    final previous = notifications.pushFocusedConversation('dm:peer1');
    expect(previous, 'broadcast');
    // The pushed key is now the active suppression target.
    await notifications.deliverNotification(notification(key: 'dm:peer1'));
    expect(delivered, isEmpty);

    // A screen restores the previous key in dispose.
    notifications.setFocusedConversation(previous);
    await notifications.deliverNotification(notification(key: 'dm:peer1'));
    expect(delivered, ['sound', 'system:Alice:hello']);
  });
}
