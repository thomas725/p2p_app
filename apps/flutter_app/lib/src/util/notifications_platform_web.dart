// Browser notification primitives (US-027 sound, US-028 system notification).
//
// Compiled only on the web via the conditional import in `notifications.dart`;
// native targets use the no-op stubs in `notifications_platform.dart`. The JS
// `Notification` is named `_JsNotification` so it can never be confused with
// the generated Rust message-notification type.

import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'web_audio.dart';

@JS('Notification')
extension type _JsNotification._(JSObject _) implements JSObject {
  external static JSString get permission;
  external static JSPromise<JSString> requestPermission();
  external factory _JsNotification(JSString title, JSObject options);
}

extension _AudioNodeOps on JSObject {
  JSObject get frequency => getProperty('frequency'.toJS)! as JSObject;
  JSObject get gain => getProperty('gain'.toJS)! as JSObject;
  void connect(JSObject target) => callMethod('connect'.toJS, target);
  void start() => callMethod('start'.toJS);
  void stop(num when) => callMethod('stop'.toJS, when.toJS);
  void setValueAtTime(num value, num time) =>
      callMethod('setValueAtTime'.toJS, value.toJS, time.toJS);
  void exponentialRampToValueAtTime(num value, num time) =>
      callMethod('exponentialRampToValueAtTime'.toJS, value.toJS, time.toJS);
}

/// Ask the browser for permission to show system notifications. Idempotent.
Future<void> platformRequestPermission() async {
  if (_JsNotification.permission.toDart != 'default') return;
  await _JsNotification.requestPermission().toDart;
}

/// Post a browser notification, requesting permission if needed.
Future<void> platformShowNotification(String title, String body) async {
  var permission = _JsNotification.permission.toDart;
  if (permission != 'granted' && permission != 'denied') {
    permission = (await _JsNotification.requestPermission().toDart).toDart;
  }
  if (permission != 'granted') return;
  final options = JSObject();
  options.setProperty('body'.toJS, body.toJS);
  _JsNotification(title.toJS, options);
}

/// A short, low-volume two-tone beep. Browsers block audio until the user has
/// interacted with the page; the caller swallows any failure.
Future<void> platformBeep() async {
  final ctx = WebAudioContext();
  if (ctx.isSuspended) await ctx.resume().toDart;
  final osc = ctx.createOscillator();
  final gain = ctx.createGain();
  osc.frequency.setValueAtTime(880, ctx.currentTime);
  osc.frequency.exponentialRampToValueAtTime(660, ctx.currentTime + 0.12);
  gain.gain.setValueAtTime(0.08, ctx.currentTime);
  gain.gain.exponentialRampToValueAtTime(0.0001, ctx.currentTime + 0.18);
  osc.connect(gain);
  gain.connect(ctx.destination);
  osc.start();
  osc.stop(ctx.currentTime + 0.2);
}
