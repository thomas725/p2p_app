// Shared browser Web Audio bindings for the two web-only audio adapters: the
// notification beep and the call-media device adapter. Centralising the
// `AudioContext` type means both open and inspect it the same way. Native
// builds import neither adapter (the conditional imports pick their stubs), so
// this file's `dart:js_interop` dependency is never compiled there.

import 'dart:js_interop';
import 'dart:js_interop_unsafe';

/// The browser `AudioContext`, shared by the notification beep and the
/// call-media adapter.
@JS('AudioContext')
extension type WebAudioContext._(JSObject _) implements JSObject {
  external factory WebAudioContext();

  external JSPromise<JSAny?> resume();
  external JSPromise<JSAny?> close();

  external num get currentTime;
  external num get sampleRate;
  external JSObject get destination;

  external JSObject createOscillator();
  external JSObject createGain();
  external WebAudioBuffer createBuffer(
    int channels,
    int length,
    int sampleRate,
  );
  external JSObject createBufferSource();
  external JSObject createMediaStreamSource(JSObject stream);
  external JSObject createScriptProcessor(
    int bufferSize,
    int inputChannels,
    int outputChannels,
  );
}

/// A PCM buffer returned by [WebAudioContext.createBuffer].
extension type WebAudioBuffer._(JSObject _) implements JSObject {
  external void copyToChannel(JSFloat32Array source, int channel);
}

extension WebAudioContextState on WebAudioContext {
  /// Browsers block audio until a user gesture; a suspended context must be
  /// resumed before it will schedule or play anything.
  bool get isSuspended {
    final raw = getProperty('state'.toJS);
    return raw != null &&
        raw.isA<JSString>() &&
        (raw as JSString).toDart == 'suspended';
  }
}
