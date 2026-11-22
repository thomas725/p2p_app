// Native device-adapter selection. The web build selects
// `call_audio_platform_web.dart` through the conditional import in
// `call_audio_device.dart`; here we pick the Android channel adapter on Android
// and keep a no-op stub for the other native targets (iOS/desktop), where the
// Rust media transport still works but no device audio ships yet.

import 'call_audio.dart';
import 'call_audio_platform_android.dart';
import 'ui_utils.dart' show isAndroid;

CallAudioAdapter createPlatformCallAudioAdapter() =>
    isAndroid ? AndroidCallAudio() : _NoopCallAudio();

class _NoopCallAudio implements CallAudioAdapter {
  @override
  String? get captureError => null;

  @override
  Future<void> start({required void Function(List<int> pcm) onPcm}) async {}

  @override
  void play(List<int> pcm) {}

  @override
  Future<void> stop() async {}
}
