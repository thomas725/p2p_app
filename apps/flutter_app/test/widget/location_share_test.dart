import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/location.dart' show LocationRecipient;
import 'package:p2p_app_flutter/src/rust/settings.dart' show LocationDefaults;
import 'package:p2p_app_flutter/src/screens/location_share.dart';
import 'package:p2p_app_flutter/src/util/location.dart'
    show LocationAdapter, LocationSample, LocationUnavailableReporter;
import 'package:p2p_app_flutter/src/util/location_session.dart'
    show LocationShareConfig;

import '../helpers/fake_node_api.dart';

class _FakeAdapter extends LocationAdapter
    implements LocationUnavailableReporter {
  _FakeAdapter({this.reason});

  final String? reason;

  @override
  String? get locationUnavailableReason => reason;

  @override
  Future<LocationSample?> sample() async =>
      const LocationSample(latitude: 48.1, longitude: 11.5, accuracy: 12);
}

void main() {
  late NodeApi previousApi;
  late FakeNodeApi api;
  final saved = <LocationDefaults>[];
  LocationShareConfig? onceConfig;
  LocationShareConfig? startConfig;
  var stops = 0;
  var adapter = _FakeAdapter();

  final defaults = LocationDefaults(
    mode: 'limited',
    durationSecs: 900,
    sensorIntervalSecs: 10,
    sendIntervalSecs: 30,
    motionGate: true,
  );

  setUp(() {
    previousApi = nodeApi;
    api = FakeNodeApi();
    saved.clear();
    onceConfig = null;
    startConfig = null;
    stops = 0;
    adapter = _FakeAdapter();
    api.getLocationDefaultsOverride = () => defaults;
    api.setLocationDefaultsOverride = (input) async => saved.add(input);
    api.listLocationRecipientsOverride = () => const [
      LocationRecipient(peerId: 'bound-phone', displayName: 'My phone'),
      LocationRecipient(peerId: 'bound-laptop', displayName: 'My laptop'),
    ];
    nodeApi = api;
  });

  tearDown(() => nodeApi = previousApi);

  Widget dialog({bool active = false}) => MaterialApp(
    home: Scaffold(
      body: LocationShareDialog(
        adapter: adapter,
        loadDefaults: nodeApi.getLocationDefaults,
        saveDefaults: (input) => nodeApi.setLocationDefaults(input: input),
        loadRecipients: nodeApi.listLocationRecipients,
        onOnce: (config) async => onceConfig = config,
        onStart: (config) async => startConfig = config,
        onStop: () async => stops++,
        active: active,
        activeLabel: active ? 'Sharing location · sample 2' : null,
      ),
    ),
  );

  testWidgets('loads Rust defaults and bound-device recipients only', (
    tester,
  ) async {
    await tester.pumpWidget(dialog());
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('location-share-mode')), findsOneWidget);
    expect(
      find.byKey(const Key('location-share-sensor-interval')),
      findsOneWidget,
    );
    expect(
      find.byKey(const Key('location-share-send-interval')),
      findsOneWidget,
    );
    expect(find.text('My phone'), findsOneWidget);
    expect(find.text('My laptop'), findsOneWidget);
    expect(
      find.text(
        'Destinations are your other bound devices, not this chat partner.',
      ),
      findsOneWidget,
    );
    expect(find.byKey(const Key('location-share-device-error')), findsNothing);
  });

  testWidgets('selecting one device shares once to exactly that device', (
    tester,
  ) async {
    await tester.pumpWidget(dialog());
    await tester.pumpAndSettle();

    final allDevices = find.byKey(const Key('location-share-all-devices'));
    await tester.ensureVisible(allDevices);
    await tester.tap(allDevices);
    await tester.pumpAndSettle();

    final recipient = find.byKey(
      const Key('location-share-recipient-bound-phone'),
    );
    await tester.ensureVisible(recipient);
    await tester.tap(recipient);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('location-share-once')));
    await tester.pumpAndSettle();

    expect(onceConfig, isNotNull);
    final config = onceConfig!;
    expect(config.mode, 'once');
    expect(config.durationSecs, isNull);
    expect(config.allDevices, isFalse);
    expect(config.selectedPeerIds, ['bound-phone']);
    // The one-off action overrides the persisted mode for this share only.
    expect(saved, isNotEmpty);
    expect(
      api.calls.where((call) => call.method == 'setLocationDefaults'),
      isNotEmpty,
    );
  });

  testWidgets('start uses the validated defaults with all devices selected', (
    tester,
  ) async {
    await tester.pumpWidget(dialog());
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('location-share-start')));
    await tester.pumpAndSettle();

    expect(startConfig, isNotNull);
    final config = startConfig!;
    expect(config.mode, 'limited');
    expect(config.durationSecs, 900);
    expect(config.sensorIntervalSecs, 10);
    expect(config.sendIntervalSecs, 30);
    expect(config.allDevices, isTrue);
    expect(config.selectedPeerIds, isEmpty);
  });

  testWidgets('an active session offers only stop and shows its status', (
    tester,
  ) async {
    await tester.pumpWidget(dialog(active: true));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('location-share-active')), findsOneWidget);
    expect(find.text('Sharing location · sample 2'), findsOneWidget);
    expect(find.byKey(const Key('location-share-start')), findsNothing);

    await tester.tap(find.byKey(const Key('location-share-stop')));
    await tester.pumpAndSettle();
    expect(stops, 1);
  });

  testWidgets('surfaces a device-unavailable reason before starting', (
    tester,
  ) async {
    adapter = _FakeAdapter(reason: 'Location permission was denied.');
    await tester.pumpWidget(dialog());
    await tester.pumpAndSettle();

    expect(
      find.byKey(const Key('location-share-device-error')),
      findsOneWidget,
    );
    expect(find.text('Location permission was denied.'), findsOneWidget);
  });
}
