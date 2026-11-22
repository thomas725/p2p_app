// Native no-op device adapter. The web build selects
// `call_audio_platform_web.dart` through the conditional import in
// `call_audio_device.dart`; native targets have no browser AudioContext and
// this is the whole implementation. Android call audio is future work — the
// Rust media transport itself is available on every platform via FRB.

import 'call_audio.dart';

CallAudioAdapter createPlatformCallAudioAdapter() => _NoopCallAudio();

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
