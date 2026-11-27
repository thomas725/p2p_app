// Tests for the platform-neutral sharing controller. The adapter and Rust
// boundary are faked so these assertions cover session mechanics (permission,
// motion gating, cached samples and cadence delegation) without GPS, FFI or a
// browser.

import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/location.dart' show LocationShareInput;
import 'package:p2p_app_flutter/src/util/location.dart'
    show LocationAdapter, LocationSample, LocationUnavailableReporter;
import 'package:p2p_app_flutter/src/util/location_session.dart'
    show LocationSessionController, LocationShareConfig;

import '../helpers/fake_node_api.dart';

const _fix = LocationSample(latitude: 48.137, longitude: 11.575, accuracy: 12);

class _ControllerAdapter extends LocationAdapter
    implements LocationUnavailableReporter {
  _ControllerAdapter({
    this.grantPermission = true,
    this.reason,
    this.moving = true,
    this.motionThrows = false,
    List<LocationSample?>? samples,
  }) : _samples = samples ?? const [_fix];

  final bool grantPermission;
  final String? reason;
  final bool moving;
  final bool motionThrows;
  final List<LocationSample?> _samples;

  int permissionRequests = 0;
  int sampleRequests = 0;
  int motionChecks = 0;
  int closes = 0;

  @override
  String? get locationUnavailableReason => reason;

  @override
  Future<bool> requestPermission() async {
    permissionRequests++;
    return grantPermission;
  }

  @override
  Future<LocationSample?> sample() async {
    final index = sampleRequests++;
    return _samples[index < _samples.length ? index : _samples.length - 1];
  }

  @override
  Future<bool> isMoving() async {
    motionChecks++;
    if (motionThrows) throw StateError('motion sensor missing');
    return moving;
  }

  @override
  Future<void> close() async {
    closes++;
  }
}

LocationShareConfig _config({bool motionGate = true}) => LocationShareConfig(
  mode: 'limited',
  durationSecs: 900,
  sensorIntervalSecs: 10,
  sendIntervalSecs: 30,
  motionGate: motionGate,
  selectedPeerIds: const ['bound-phone'],
  allDevices: false,
);

({
  FakeNodeApi api,
  List<LocationShareInput> built,
  List<({List<String> peerIds, bool allDevices, String body})> sent,
})
_api({bool active = true, bool queryDue = true, bool sendDue = true}) {
  final api = FakeNodeApi();
  final built = <LocationShareInput>[];
  final sent = <({List<String> peerIds, bool allDevices, String body})>[];
  api.buildLocationMessageOverride = (input) async {
    built.add(input);
    return 'body-${input.sequence}';
  };
  api.sendLocationToDevicesOverride = (peerIds, allDevices, body) {
    sent.add((peerIds: peerIds, allDevices: allDevices, body: body));
    return peerIds;
  };
  api.locationSessionActiveOverride =
      (startedAt, mode, durationSecs, now) async => active;
  api.locationQueryDueOverride = (lastQueryAt, now, intervalSecs) => queryDue;
  api.locationSendDueOverride = (lastSentAt, now, intervalSecs) async =>
      sendDue;
  return (api: api, built: built, sent: sent);
}

void main() {
  test('one-shot sends one final sample then closes the adapter', () async {
    final harness = _api();
    final adapter = _ControllerAdapter();
    final errors = <String>[];
    var finished = 0;
    final controller = LocationSessionController(
      api: harness.api,
      adapter: adapter,
      config: _config(),
      onError: errors.add,
      onFinished: () => finished++,
      nowSeconds: () => 100,
      sessionId: 'one-shot',
    );

    await controller.startOnce();

    expect(adapter.permissionRequests, 1);
    expect(adapter.sampleRequests, 1);
    expect(harness.built, hasLength(1));
    expect(harness.built.single.mode, 'once');
    expect(harness.built.single.durationSecs, isNull);
    expect(harness.built.single.isFinal, isTrue);
    expect(harness.built.single.sessionId, 'one-shot');
    final sent = harness.sent.single;
    expect(sent.peerIds, ['bound-phone']);
    expect(sent.allDevices, isFalse);
    expect(sent.body, 'body-1');
    expect(errors, isEmpty);
    expect(finished, 1);
    expect(adapter.closes, 1);
    expect(controller.active, isFalse);
    expect(
      harness.api.calls.where((call) => call.method == 'locationSessionActive'),
      isEmpty,
    );
  });

  test(
    'stationary session reuses its fix and keeps final on the last fix',
    () async {
      final harness = _api();
      final adapter = _ControllerAdapter(moving: false);
      final queryCalls =
          <({double? lastQueryAt, double now, int intervalSecs})>[];
      harness.api.locationQueryDueOverride = (lastQueryAt, now, intervalSecs) {
        queryCalls.add((
          lastQueryAt: lastQueryAt,
          now: now,
          intervalSecs: intervalSecs,
        ));
        return true;
      };
      var now = 100.0;
      final controller = LocationSessionController(
        api: harness.api,
        adapter: adapter,
        config: _config(),
        onError: (_) => fail('unexpected location error'),
        nowSeconds: () => now,
        sessionId: 'continued',
      );

      await controller.startContinued();
      now = 160;
      await controller.tick();
      now = 170;
      await controller.tick();
      await controller.stop();

      // One real fix, then two cached sends and a cached final. The low-power
      // gate is checked only when a sensor query is actually due.
      expect(adapter.sampleRequests, 1);
      expect(adapter.motionChecks, 2);
      expect(harness.built.map((input) => input.sequence), [1, 2, 3, 4]);
      expect(harness.built.map((input) => input.isFinal), [
        false,
        false,
        false,
        true,
      ]);
      expect(
        harness.built.every((input) => input.sessionId == 'continued'),
        isTrue,
      );
      // Re-sending a cached fix never advances Rust's sensor-query clock.
      expect(queryCalls.map((call) => call.lastQueryAt), [100.0, 100.0]);
      expect(harness.sent, hasLength(4));
    },
  );

  test('tick delegates mode, expiry and both cadences to Rust', () async {
    final harness = _api(queryDue: false, sendDue: false);
    final adapter = _ControllerAdapter();
    final activeCalls = <Object?>[];
    final sendCalls = <Object?>[];
    harness.api.locationSessionActiveOverride =
        (startedAt, mode, durationSecs, now) async {
          activeCalls.add((
            startedAt: startedAt,
            mode: mode,
            durationSecs: durationSecs,
            now: now,
          ));
          return true;
        };
    harness.api.locationSendDueOverride =
        (lastSentAt, now, intervalSecs) async {
          sendCalls.add((
            lastSentAt: lastSentAt,
            now: now,
            intervalSecs: intervalSecs,
          ));
          return false;
        };
    final controller = LocationSessionController(
      api: harness.api,
      adapter: adapter,
      config: _config(),
      onError: (_) => fail('unexpected location error'),
      nowSeconds: () => 100,
      sessionId: 'cadence',
    );

    await controller.startContinued();
    await controller.tick();

    expect(activeCalls.single, (
      startedAt: 100.0,
      mode: 'limited',
      durationSecs: 900,
      now: 100.0,
    ));
    final query = harness.api.calls
        .where((call) => call.method == 'locationQueryDue')
        .single;
    expect(query.argument, {
      'lastQueryAt': 100.0,
      'now': 100.0,
      'intervalSecs': 10,
    });
    expect(sendCalls.single, (lastSentAt: 100.0, now: 100.0, intervalSecs: 30));
    // Rust decided neither query nor send is due: no extra sensor work.
    expect(adapter.sampleRequests, 1);
    expect(harness.built, hasLength(1));
  });

  test('a broken motion sensor fails open for the first fix', () async {
    final harness = _api();
    final adapter = _ControllerAdapter(
      motionThrows: true,
      samples: const [null, _fix],
    );
    final errors = <String>[];
    final controller = LocationSessionController(
      api: harness.api,
      adapter: adapter,
      config: _config(),
      onError: errors.add,
      nowSeconds: () => 100,
      sessionId: 'fail-open',
    );

    await controller.startContinued();
    expect(harness.built, isEmpty);
    expect(errors, ['No location fix available.']);
    errors.clear();

    await controller.tick();

    expect(adapter.motionChecks, 1);
    expect(adapter.sampleRequests, 2);
    expect(harness.built, hasLength(1));
    expect(harness.built.single.sequence, 1);
    expect(errors, isEmpty);
  });

  test(
    'permission denial surfaces the reason and stops before sampling',
    () async {
      final harness = _api();
      final adapter = _ControllerAdapter(
        grantPermission: false,
        reason: 'Location permission was denied.',
      );
      final errors = <String>[];
      var finished = 0;
      final controller = LocationSessionController(
        api: harness.api,
        adapter: adapter,
        config: _config(),
        onError: errors.add,
        onFinished: () => finished++,
        nowSeconds: () => 100,
      );

      await controller.startContinued();

      expect(errors, ['Location permission was denied.']);
      expect(adapter.sampleRequests, 0);
      expect(adapter.closes, 1);
      expect(finished, 1);
      expect(harness.built, isEmpty);
      expect(controller.active, isFalse);
    },
  );

  test('a rejected location body is reported to the operator', () async {
    final harness = _api();
    harness.api.buildLocationMessageOverride = (_) async =>
        throw StateError('invalid coordinates');
    final adapter = _ControllerAdapter();
    final errors = <String>[];
    final controller = LocationSessionController(
      api: harness.api,
      adapter: adapter,
      config: _config(),
      onError: errors.add,
      nowSeconds: () => 100,
    );

    await controller.startOnce();

    expect(errors.single, contains('invalid coordinates'));
    expect(harness.sent, isEmpty);
  });
}
