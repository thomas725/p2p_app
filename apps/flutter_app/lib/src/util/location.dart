// Platform-neutral device location and motion sampling.
//
// Rust owns every policy decision (validation, canonical message text, expiry
// and cadence). These adapters only move raw sensor values across the boundary:
// a [LocationSample] is deliberately unvalidated until
// `NodeApi.buildLocationMessage` accepts it. Concrete implementations are
// selected by the conditional import in `location_device.dart`.

import 'dart:async';

/// One raw device fix. Validity is checked in Rust, never here.
class LocationSample {
  const LocationSample({
    required this.latitude,
    required this.longitude,
    required this.accuracy,
  });

  final double latitude;
  final double longitude;
  final double accuracy;

  @override
  String toString() => 'LocationSample($latitude, $longitude, ±${accuracy}m)';
}

/// The device half of location sharing.
abstract class LocationAdapter {
  /// Ask the platform for location permission, returning whether it is available.
  ///
  /// Browsers ask implicitly when [sample] calls Geolocation, so the default is
  /// `true`; native adapters can override this to surface a real grant dialog.
  Future<bool> requestPermission() async => true;

  /// One device fix, or `null` when no sensor/permission is available.
  Future<LocationSample?> sample();

  /// Whether the device is moving. Implementations fail open (`true`) when no
  /// motion sensor exists so a missing sensor never silently stops sharing.
  Future<bool> isMoving() async => true;

  /// Release any sensor listeners. Safe to call more than once.
  Future<void> close() async {}
}

/// Adapter that reports no sensor. Used on native targets without an adapter;
/// the UI checks for it before starting a session.
class NoopLocationAdapter implements LocationAdapter {
  const NoopLocationAdapter();

  @override
  Future<bool> requestPermission() async => true;

  @override
  Future<LocationSample?> sample() async => null;

  @override
  Future<bool> isMoving() async => true;

  @override
  Future<void> close() async {}
}

/// Optional thin reporting seam for adapters that can say why a fix is missing.
///
/// This is diagnostics only — the adapter never invents validation or retry
/// policy; Rust still owns all of that. Keeping the getter here (rather than in
/// the platform files) lets VM tests exercise the reason path without importing
/// `dart:js_interop`.
abstract class LocationUnavailableReporter {
  String? get locationUnavailableReason;
}

/// Why this device cannot sample location, or `null` when it can.
///
/// Kept separate from [LocationAdapter] so the UI can explain a failure before
/// starting a session (Android: permission, browser: secure context).
String? locationDeviceUnavailableReason(LocationAdapter adapter) {
  if (adapter is LocationUnavailableReporter) {
    return (adapter as LocationUnavailableReporter).locationUnavailableReason;
  }
  return adapter is NoopLocationAdapter
      ? 'Location sharing is unavailable on this device.'
      : null;
}

// The two pure helpers below describe *browser facts* only. They live in this
// platform-neutral file so unit tests can assert them on the VM (a test cannot
// import the web adapter, which pulls in `dart:js_interop`). The web adapter
// re-exports them so callers of that file see the same names.

/// Gravity-compensated acceleration (m/s²) above which a browser `devicemotion`
/// sample counts as movement. A raw sensor debounce, not sharing policy: Rust
/// decides how often to sample and what to do with the answer.
const double webMotionAccelerationThreshold = 0.5;

/// Whether one gravity-compensated browser acceleration sample is movement.
bool webAccelerationIndicatesMotion({
  required double x,
  required double y,
  required double z,
  double threshold = webMotionAccelerationThreshold,
}) => x * x + y * y + z * z > threshold * threshold;

/// Why the browser Geolocation API cannot be used, or `null` when it can.
String? webLocationUnavailableReason({
  required bool secureContext,
  required bool permissionGranted,
}) {
  if (!secureContext) {
    return 'Location sharing needs a secure context (HTTPS or localhost).';
  }
  if (!permissionGranted) {
    return 'Location permission was denied in the browser.';
  }
  return null;
}
