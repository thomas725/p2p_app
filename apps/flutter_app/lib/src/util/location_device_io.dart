// Native device-location selection and the Android method-channel adapter.
//
// Compiled on every non-web target via the conditional import in
// `location_device.dart`. Rust owns validation, message text, expiry and
// cadence; this file only turns a raw Android location map into a
// [LocationSample] and reports raw motion state (fail open when in doubt).

import 'package:flutter/services.dart';

import 'location.dart';
import 'ui_utils.dart' show isAndroid;

/// Channel shared with `LocationSensor.kt`; drift silently drops every fix.
const String locationChannelName = 'p2p_app/location';

LocationAdapter createPlatformLocationAdapter() =>
    isAndroid ? MethodChannelLocationAdapter() : const NoopLocationAdapter();

/// Speaks the `p2p_app/location` channel contract: `requestPermission` asks
/// Android for a runtime grant, `getLocation` returns a map with `latitude`,
/// `longitude` and `accuracy` (or `null`), `isMoving` returns a bool (defaulting
/// to `true`), and `close` releases any native listeners.
///
/// The channel is injectable so VM tests can fake the native side with
/// `setMockMethodCallHandler`.
class MethodChannelLocationAdapter
    implements LocationAdapter, LocationUnavailableReporter {
  MethodChannelLocationAdapter({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(locationChannelName);

  final MethodChannel _channel;
  String? _unavailableReason;

  @override
  String? get locationUnavailableReason => _unavailableReason;

  @override
  Future<bool> requestPermission() async {
    try {
      final granted = await _channel.invokeMethod<bool>('requestPermission');
      if (granted ?? false) {
        _unavailableReason = null;
        return true;
      }
      _unavailableReason = 'Location permission was denied on this device.';
      return false;
    } on MissingPluginException {
      _unavailableReason = 'Location is unavailable on this device.';
      return false;
    } on PlatformException catch (e) {
      _unavailableReason =
          e.message ?? 'Location permission was denied on this device.';
      return false;
    }
  }

  @override
  Future<LocationSample?> sample() async {
    Object? raw;
    try {
      raw = await _channel.invokeMethod<Map<dynamic, dynamic>>('getLocation');
    } on MissingPluginException {
      _unavailableReason = 'Location is unavailable on this device.';
      return null;
    } on PlatformException catch (e) {
      _unavailableReason =
          e.message ?? 'Location is unavailable on this device.';
      return null;
    }
    if (raw is! Map) {
      _unavailableReason = 'Location is unavailable on this device.';
      return null;
    }
    final latitude = _readDouble(raw['latitude']);
    final longitude = _readDouble(raw['longitude']);
    final accuracy = _readDouble(raw['accuracy']);
    if (latitude == null || longitude == null || accuracy == null) {
      // Rust rejects an implausible fix; here a malformed map is simply no fix.
      _unavailableReason = 'Location is unavailable on this device.';
      return null;
    }
    _unavailableReason = null;
    return LocationSample(
      latitude: latitude,
      longitude: longitude,
      accuracy: accuracy,
    );
  }

  @override
  Future<bool> isMoving() async {
    try {
      final moving = await _channel.invokeMethod<bool>('isMoving');
      // Missing sensor, missing plugin or any platform error all fail open.
      return moving ?? true;
    } on MissingPluginException {
      return true;
    } on PlatformException {
      return true;
    }
  }

  @override
  Future<void> close() async {
    try {
      await _channel.invokeMethod<void>('close');
    } on MissingPluginException {
      // An older native build has no listeners to release.
    } on PlatformException {
      // Releasing is best effort; a stale listener must never break the UI.
    }
  }

  static double? _readDouble(Object? value) {
    if (value is num) return value.toDouble();
    if (value is String) return double.tryParse(value);
    return null;
  }
}
