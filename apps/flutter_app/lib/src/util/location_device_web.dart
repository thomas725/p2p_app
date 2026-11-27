// Browser device-location adapter. Compiled only on the web via the conditional
// import in `location_device.dart`; no `dart:html`, no plugin and no new deps.
//
// Geolocation is callback-based in the browser, so Dart closures are handed to
// JS as `JSFunction`s. Rust still owns validation, message text, expiry and
// cadence: this file only reports raw fixes and raw motion state, failing open
// (`isMoving == true`) whenever the motion API is missing or silent.

library;

import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'location.dart';

// Pure helpers are declared in the platform-neutral `location.dart` so VM unit
// tests can assert them without importing `dart:js_interop`.
export 'location.dart'
    show
        webAccelerationIndicatesMotion,
        webLocationUnavailableReason,
        webMotionAccelerationThreshold;

@JS('navigator.geolocation')
external JSObject? get _geolocation;

@JS('DeviceMotionEvent')
external JSAny? get _deviceMotionEvent;

@JS('isSecureContext')
external JSAny? get _jsSecureContext;

@JS('globalThis')
external JSObject get _globalThis;

/// How long to wait for one motion event before deciding the device is still.
/// A raw sampling window, not sharing cadence (which Rust owns).
const Duration webMotionWindow = Duration(milliseconds: 800);

/// Safety net when a browser never invokes either geolocation callback.
const Duration webLocationTimeout = Duration(seconds: 12);

LocationAdapter createPlatformLocationAdapter() => _BrowserLocationAdapter();

class _BrowserLocationAdapter
    implements LocationAdapter, LocationUnavailableReporter {
  bool _permissionDenied = false;
  JSFunction? _motionListener;
  Completer<bool>? _motionCompleter;
  Timer? _motionTimer;
  bool _sawMotionEvent = false;

  @override
  String? get locationUnavailableReason => webLocationUnavailableReason(
    secureContext: _secureContext,
    permissionGranted: !_permissionDenied,
  );

  @override
  Future<bool> requestPermission() async {
    // Geolocation asks implicitly from `sample`; an insecure context or a
    // previously denied prompt is the only answer available up front. Both are
    // surfaced through `locationUnavailableReason` by the dialog/session.
    return _secureContext && !_permissionDenied;
  }

  bool get _secureContext {
    final raw = _jsSecureContext;
    if (raw.isA<JSBoolean>()) return (raw as JSBoolean).toDart;
    // Older browsers: HTTPS or loopback is the practical secure-context rule.
    final uri = Uri.base;
    return uri.scheme == 'https' || uri.host == 'localhost';
  }

  @override
  Future<LocationSample?> sample() async {
    final geolocation = _geolocation;
    if (geolocation == null) return null;

    final completer = Completer<LocationSample?>();
    final options = JSObject();
    options.setProperty('enableHighAccuracy'.toJS, true.toJS);
    options.setProperty('timeout'.toJS, 10000.toJS);
    options.setProperty('maximumAge'.toJS, 30000.toJS);

    void success(JSAny? position) {
      if (completer.isCompleted) return;
      try {
        if (!position.isA<JSObject>()) {
          completer.complete(null);
          return;
        }
        final rawCoords = (position as JSObject).getProperty('coords'.toJS);
        if (!rawCoords.isA<JSObject>()) {
          completer.complete(null);
          return;
        }
        final coords = rawCoords as JSObject;
        final latitude = _number(coords, 'latitude');
        final longitude = _number(coords, 'longitude');
        final accuracy = _number(coords, 'accuracy');
        if (latitude == null || longitude == null || accuracy == null) {
          completer.complete(null);
          return;
        }
        // Rust validates the fix; we only copy the numbers the browser gave us.
        completer.complete(
          LocationSample(
            latitude: latitude,
            longitude: longitude,
            accuracy: accuracy,
          ),
        );
      } catch (_) {
        completer.complete(null);
      }
    }

    void failure(JSAny? error) {
      if (error.isA<JSObject>()) {
        final code = (error as JSObject).getProperty('code'.toJS);
        // 1 is PERMISSION_DENIED; 2/3 are position/timeout failures.
        if (code.isA<JSNumber>() && (code as JSNumber).toDartDouble == 1) {
          _permissionDenied = true;
        }
      }
      if (!completer.isCompleted) completer.complete(null);
    }

    final timer = Timer(webLocationTimeout, () {
      if (!completer.isCompleted) completer.complete(null);
    });
    geolocation.callMethod(
      'getCurrentPosition'.toJS,
      success.toJS,
      failure.toJS,
      options,
    );
    return completer.future.whenComplete(timer.cancel);
  }

  @override
  Future<bool> isMoving() async {
    // No DeviceMotionEvent (desktop browser, disabled sensor): fail open.
    if (_deviceMotionEvent == null) return true;

    final completer = Completer<bool>();
    _motionCompleter = completer;
    _sawMotionEvent = false;

    final JSFunction listener = ((JSAny? event) {
      if (completer.isCompleted || !event.isA<JSObject>()) return;
      final acceleration = (event as JSObject).getProperty('acceleration'.toJS);
      if (!acceleration.isA<JSObject>()) return;
      final vector = acceleration as JSObject;
      final x = _number(vector, 'x');
      final y = _number(vector, 'y');
      final z = _number(vector, 'z');
      if (x == null || y == null || z == null) return;
      _sawMotionEvent = true;
      if (webAccelerationIndicatesMotion(x: x, y: y, z: z)) {
        _finishMotion(completer, true);
      }
    }).toJS;

    _motionListener = listener;
    _globalThis.callMethod(
      'addEventListener'.toJS,
      'devicemotion'.toJS,
      listener,
    );
    _motionTimer = Timer(webMotionWindow, () {
      // Silent sensor also fails open; a quiet but live sensor reports still.
      _finishMotion(completer, !_sawMotionEvent);
    });
    return completer.future;
  }

  @override
  Future<void> close() async {
    final pending = _motionCompleter;
    if (pending != null && !pending.isCompleted) _finishMotion(pending, true);
    _permissionDenied = false;
  }

  void _finishMotion(Completer<bool> completer, bool value) {
    if (completer.isCompleted) return;
    _motionTimer?.cancel();
    _motionTimer = null;
    final listener = _motionListener;
    if (listener != null) {
      _globalThis.callMethod(
        'removeEventListener'.toJS,
        'devicemotion'.toJS,
        listener,
      );
      _motionListener = null;
    }
    if (identical(_motionCompleter, completer)) _motionCompleter = null;
    completer.complete(value);
  }

  static double? _number(JSObject object, String key) {
    final value = object.getProperty(key.toJS);
    return value.isA<JSNumber>() ? (value as JSNumber).toDartDouble : null;
  }
}
