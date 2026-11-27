// VM tests for the device-location adapters: no browser APIs, no FFI and no
// Android runtime. The native side is faked with `setMockMethodCallHandler`,
// and the web helpers are pure functions in the platform-neutral contract file.

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:p2p_app_flutter/src/util/location_device.dart';
import 'package:p2p_app_flutter/src/util/location_device_io.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const channel = MethodChannel(locationChannelName);
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  tearDown(() {
    messenger.setMockMethodCallHandler(channel, null);
  });

  group('createLocationAdapter', () {
    test('returns the no-op adapter on an unsupported native host', () {
      // `flutter test` runs on a desktop VM, so there is no Android channel.
      expect(createLocationAdapter(), isA<NoopLocationAdapter>());
      final adapter = createLocationAdapter();
      expect(locationDeviceUnavailableReason(adapter), isNotNull);
    });

    test('no-op adapter fails open for motion and closes cleanly', () async {
      const adapter = NoopLocationAdapter();
      expect(await adapter.sample(), isNull);
      expect(await adapter.isMoving(), isTrue);
      await adapter.close();
      await adapter.close();
    });
  });

  group('web pure helpers', () {
    test('secure context and permission reason precedence', () {
      expect(
        webLocationUnavailableReason(
          secureContext: true,
          permissionGranted: true,
        ),
        isNull,
      );
      expect(
        webLocationUnavailableReason(
          secureContext: false,
          permissionGranted: true,
        ),
        contains('secure context'),
      );
      expect(
        webLocationUnavailableReason(
          secureContext: false,
          permissionGranted: false,
        ),
        contains('secure context'),
      );
      expect(
        webLocationUnavailableReason(
          secureContext: true,
          permissionGranted: false,
        ),
        contains('permission'),
      );
    });

    test('acceleration helper applies the gravity-compensated threshold', () {
      expect(webMotionAccelerationThreshold, 0.5);
      expect(webAccelerationIndicatesMotion(x: 0.1, y: -0.2, z: 0.1), isFalse);
      expect(webAccelerationIndicatesMotion(x: 0.6, y: 0, z: 0), isTrue);
      expect(
        webAccelerationIndicatesMotion(x: 0, y: 0, z: -0.7, threshold: 1),
        isFalse,
      );
    });
  });

  group('MethodChannelLocationAdapter', () {
    test('forwards the Android runtime permission request', () async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call);
        return true;
      });
      final adapter = MethodChannelLocationAdapter();

      expect(await adapter.requestPermission(), isTrue);
      expect(calls.single.method, 'requestPermission');
      expect(locationDeviceUnavailableReason(adapter), isNull);
    });

    test(
      'a denied permission reports a reason instead of silently failing',
      () async {
        messenger.setMockMethodCallHandler(channel, (call) async => false);
        final adapter = MethodChannelLocationAdapter();

        expect(await adapter.requestPermission(), isFalse);
        expect(
          locationDeviceUnavailableReason(adapter),
          contains('permission'),
        );
      },
    );

    test('converts a native fix map into an unvalidated sample', () async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call);
        if (call.method == 'getLocation') {
          return <dynamic, dynamic>{
            'latitude': 48.2082,
            'longitude': 16.3738,
            'accuracy': 12.5,
          };
        }
        return null;
      });

      final adapter = MethodChannelLocationAdapter();
      final sample = await adapter.sample();

      expect(calls.single.method, 'getLocation');
      expect(sample!.latitude, 48.2082);
      expect(sample.longitude, 16.3738);
      expect(sample.accuracy, 12.5);
      expect(locationDeviceUnavailableReason(adapter), isNull);
    });

    test(
      'a malformed or missing fix reports a reason, not an exception',
      () async {
        messenger.setMockMethodCallHandler(channel, (call) async {
          return <dynamic, dynamic>{'latitude': 'not-a-number'};
        });
        final adapter = MethodChannelLocationAdapter();
        expect(await adapter.sample(), isNull);
        expect(locationDeviceUnavailableReason(adapter), isNotNull);

        messenger.setMockMethodCallHandler(channel, (call) async => null);
        expect(await adapter.sample(), isNull);
      },
    );

    test('missing plugin fails open for location and motion', () async {
      // No handler installed: the channel throws MissingPluginException.
      final adapter = MethodChannelLocationAdapter();
      expect(await adapter.sample(), isNull);
      expect(await adapter.isMoving(), isTrue);
      await adapter.close();
      await adapter.close();
    });

    test('motion defaults open on null or platform errors', () async {
      messenger.setMockMethodCallHandler(channel, (call) async {
        if (call.method == 'isMoving') return null;
        return null;
      });
      final adapter = MethodChannelLocationAdapter();
      expect(await adapter.isMoving(), isTrue);

      messenger.setMockMethodCallHandler(channel, (call) async {
        throw PlatformException(code: 'SENSOR', message: 'no accelerometer');
      });
      expect(await adapter.isMoving(), isTrue);
      expect(await adapter.sample(), isNull);
    });

    test('forwards a real motion reading and the close command', () async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call);
        return call.method == 'isMoving' ? false : null;
      });
      final adapter = MethodChannelLocationAdapter();

      expect(await adapter.isMoving(), isFalse);
      await adapter.close();

      expect(calls.map((call) => call.method), <String>['isMoving', 'close']);
    });
  });
}
