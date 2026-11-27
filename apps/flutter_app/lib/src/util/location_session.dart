// Platform-neutral controller for one continued location-sharing session.
//
// Rust owns every policy decision: the canonical message body, mode/expiry,
// sensor and send cadence, eligibility of the destination devices. This
// controller only pumps a Timer, asks the injected adapter for raw fixes and
// calls the Rust-backed `NodeApi` helpers. The clock is injected so tests can
// drive time without sleeping.

import 'dart:async';

import 'package:flutter/foundation.dart' show VoidCallback, debugPrint;

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/location.dart' show LocationShareInput;
import 'package:p2p_app_flutter/src/rust/settings.dart' show LocationDefaults;

import 'location.dart' show LocationAdapter, LocationSample;
import 'location_device.dart' show locationDeviceUnavailableReason;

/// What a dialog asks the session controller to send to.
///
/// Recipients are always resolved by Rust from the local user's device
/// bindings; the DM partner is never added implicitly.
class LocationShareConfig {
  const LocationShareConfig({
    required this.mode,
    this.durationSecs,
    required this.sensorIntervalSecs,
    required this.sendIntervalSecs,
    required this.motionGate,
    required this.selectedPeerIds,
    required this.allDevices,
  });

  /// Build a session config from validated Rust defaults plus UI selection.
  factory LocationShareConfig.fromDefaults(
    LocationDefaults defaults, {
    required List<String> selectedPeerIds,
    required bool allDevices,
    String? modeOverride,
    int? durationOverride,
  }) => LocationShareConfig(
    mode: modeOverride ?? defaults.mode,
    durationSecs: modeOverride == 'once'
        ? null
        : (durationOverride ?? defaults.durationSecs),
    sensorIntervalSecs: defaults.sensorIntervalSecs,
    sendIntervalSecs: defaults.sendIntervalSecs,
    motionGate: defaults.motionGate,
    selectedPeerIds: List<String>.from(selectedPeerIds),
    allDevices: allDevices,
  );

  final String mode;
  final int? durationSecs;
  final int sensorIntervalSecs;
  final int sendIntervalSecs;
  final bool motionGate;
  final List<String> selectedPeerIds;
  final bool allDevices;
}

/// Drives one sharing session (a single fix for `once`, a polled session for
/// `limited`/`unlimited`). Every cadence question goes back to Rust; no policy
/// is implemented here.
class LocationSessionController {
  LocationSessionController({
    required this.api,
    required this.adapter,
    required this.config,
    required this.onError,
    this.onSent,
    this.onFinished,
    double Function()? nowSeconds,
    Duration? tickInterval,
    String? sessionId,
  }) : _now = nowSeconds ?? _wallClockSeconds,
       _tickInterval = tickInterval ?? const Duration(seconds: 1),
       _sessionId = sessionId;

  final NodeApi api;
  final LocationAdapter adapter;
  final LocationShareConfig config;

  /// Operator-visible failure (permission, no fix, rejected body/send).
  final void Function(String message) onError;

  /// Called after every successfully queued canonical sample.
  final void Function(String body)? onSent;

  /// Called when the session ends (stop, expiry or a failed permission check).
  final VoidCallback? onFinished;

  final double Function() _now;
  final Duration _tickInterval;
  final String? _sessionId;

  Timer? _timer;
  bool _active = false;
  bool _busy = false;
  bool _closed = false;
  bool _once = false;
  String _mode = 'once';
  int? _durationSecs;
  String? _startedAtSessionId;
  double? _startedAt;
  double? _lastQueryAt;
  double? _lastSentAt;
  LocationSample? _cachedSample;
  int _sequence = 0;

  bool get active => _active;

  /// Stable sender-minted id reused by every message of one session.
  String get sessionId => _sessionId ?? _startedAtSessionId ?? 'session';

  int get sequence => _sequence;

  static double _wallClockSeconds() =>
      DateTime.now().microsecondsSinceEpoch / 1_000_000.0;

  /// Sample once, mark the body final, send, then stop without a timer.
  Future<void> startOnce() => _start(once: true);

  /// Start a polled limited/unlimited session (first sample goes out at once).
  Future<void> startContinued() => _start(once: false);

  Future<void> _start({required bool once}) async {
    if (_active || _busy) return;
    _busy = true;
    try {
      final granted = await adapter.requestPermission();
      if (!granted) {
        _report(
          locationDeviceUnavailableReason(adapter) ??
              'Location permission was denied on this device.',
        );
        await _closeAdapter();
        onFinished?.call();
        return;
      }
      _once = once;
      _mode = once ? 'once' : config.mode;
      _durationSecs = once ? null : config.durationSecs;
      _startedAtSessionId = _sessionId ?? 'loc-${_now().toStringAsFixed(3)}';
      _active = true;
      _startedAt = _now();
      await _sendFreshSample(isFinal: once);
      if (once) {
        // A one-off share has no timer, but it still owns the device sensor.
        _active = false;
        await _closeAdapter();
        onFinished?.call();
      } else if (_active) {
        _timer = Timer.periodic(_tickInterval, (_) => unawaited(tick()));
      }
    } finally {
      _busy = false;
    }
  }

  /// One timer beat: ask Rust whether the session is still active and whether a
  /// sensor query / network send is due, then act on those answers only.
  Future<void> tick() async {
    if (!_active || _busy || _once) return;
    _busy = true;
    try {
      final now = _now();
      final startedAt = _startedAt;
      if (startedAt == null) return;
      final active = await api.locationSessionActive(
        startedAt: startedAt,
        mode: _mode,
        durationSecs: _durationSecs,
        now: now,
      );
      if (!active) {
        // Expiry is Rust's decision; the final sample closes the session.
        await _finalize();
        return;
      }
      final queryDue = await api.locationQueryDue(
        lastQueryAt: _lastQueryAt,
        now: now,
        intervalSecs: config.sensorIntervalSecs,
      );
      // Check the low-power gate and touch the sensor at most once per tick.
      // A stationary device keeps sending its last fix, but never re-queries
      // the GPS sensor just because a network send is also due.
      var queried = false;
      if (queryDue) {
        queried = true;
        if (await _sensorQueryAllowed()) {
          final sample = await adapter.sample();
          if (sample != null) {
            _cachedSample = sample;
            _lastQueryAt = now;
          } else {
            _report(_noFixReason());
          }
        }
      }
      final sendDue = await api.locationSendDue(
        lastSentAt: _lastSentAt,
        now: now,
        intervalSecs: config.sendIntervalSecs,
      );
      if (!sendDue) return;
      if (_cachedSample != null) {
        // Sending the cached fix is not a sensor query; `_lastQueryAt` stays
        // untouched so Rust's sensor cadence still governs the next fix.
        await _sendCachedSample(isFinal: false, now: now);
      } else if (!queried && await _sensorQueryAllowed()) {
        await _sendFreshSample(isFinal: false);
      }
    } finally {
      _busy = false;
    }
  }

  /// Stop an active session: one final canonical sample, then adapter/timer
  /// cleanup. Safe to call more than once.
  Future<void> stop() async {
    if (!_active || _busy) return;
    await _finalize();
  }

  /// Tear down without awaiting (used from `dispose`). An active session still
  /// emits its final sample so recipients are not left waiting for a close.
  void dispose() {
    _timer?.cancel();
    _timer = null;
    if (_active && !_busy) unawaited(_finalize());
  }

  Future<void> _finalize() async {
    if (!_active) return;
    _active = false;
    _timer?.cancel();
    _timer = null;
    try {
      if (_cachedSample != null) {
        // The final marker rides the last known fix. Re-querying a stationary
        // device just to repeat its coordinates would waste sensor power.
        await _sendCachedSample(isFinal: true, now: _now());
      } else if (await _sensorQueryAllowed()) {
        await _sendFreshSample(isFinal: true);
      }
    } finally {
      await _closeAdapter();
      onFinished?.call();
    }
  }

  Future<bool> _sensorQueryAllowed() async {
    if (!config.motionGate) return true;
    try {
      // A missing or broken motion sensor fails open: sharing must not stop
      // silently because the low-power gate is unavailable.
      return await adapter.isMoving();
    } catch (_) {
      return true;
    }
  }

  Future<void> _sendFreshSample({required bool isFinal}) async {
    final sample = await adapter.sample();
    if (sample == null) {
      _report(_noFixReason());
      return;
    }
    _cachedSample = sample;
    _lastQueryAt = _now();
    await _sendCachedSample(isFinal: isFinal, now: _lastQueryAt ?? _now());
  }

  Future<void> _sendCachedSample({
    required bool isFinal,
    required double now,
  }) async {
    final sample = _cachedSample;
    if (sample == null) return;
    final sequence = ++_sequence;
    try {
      final body = await api.buildLocationMessage(
        input: LocationShareInput(
          latitude: sample.latitude,
          longitude: sample.longitude,
          accuracy: sample.accuracy,
          mode: _mode,
          durationSecs: _durationSecs,
          sensorIntervalSecs: config.sensorIntervalSecs,
          sendIntervalSecs: config.sendIntervalSecs,
          motionGate: config.motionGate,
          sessionId: sessionId,
          sequence: sequence,
          isFinal: isFinal,
        ),
      );
      await api.sendLocationToDevices(
        selectedPeerIds: config.selectedPeerIds,
        allDevices: config.allDevices,
        body: body,
      );
      // Only a real sensor read advances the query cadence; re-sending a
      // cached fix must not postpone the next GPS query.
      _lastSentAt = now;
      onSent?.call(body);
    } catch (e) {
      _report('Location send failed: $e');
    }
  }

  String _noFixReason() =>
      locationDeviceUnavailableReason(adapter) ?? 'No location fix available.';

  void _report(String message) => onError(message);

  Future<void> _closeAdapter() async {
    if (_closed) return;
    _closed = true;
    try {
      await adapter.close();
    } catch (e) {
      debugPrint('Failed to close location adapter: $e');
    }
  }
}
