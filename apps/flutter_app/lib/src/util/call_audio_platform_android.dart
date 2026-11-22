// Android microphone/speaker adapter for the Rust-owned call media session
// (US-044). The native half is `CallAudio.kt`; this file only speaks the
// method-channel contract and converts between Dart i16 PCM and the bytes the
// platform channel carries. Rust still owns framing, jitter and mute — and
// Android's AudioRecord/AudioTrack run at the wire rate (16 kHz mono), so there
// is no resampling here (unlike the web adapter).

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:flutter/services.dart';

import 'call_audio.dart';

/// Channels shared with `CallAudio.kt`. Drift silently drops all call audio, so
/// they are pinned by `CallAudioTest.kt` and by the Dart tests below. Commands
/// are Dart -> native; events are native -> Dart (captured PCM frames).
const String callAudioChannelName = 'com.example.p2p_app_flutter/call_audio';
const String callAudioEventsChannelName =
    'com.example.p2p_app_flutter/call_audio_events';

class AndroidCallAudio implements CallAudioAdapter {
  /// Channels are injectable so unit tests can mock the native side.
  AndroidCallAudio({MethodChannel? channel, MethodChannel? events})
    : _channel = channel ?? const MethodChannel(callAudioChannelName),
      _events = events ?? const MethodChannel(callAudioEventsChannelName);

  final MethodChannel _channel;
  final MethodChannel _events;
  void Function(List<int> pcm)? _onPcm;
  bool _started = false;
  String? _captureError;
  int _generation = 0;

  @override
  String? get captureError => _captureError;

  @override
  Future<void> start({required void Function(List<int> pcm) onPcm}) async {
    if (_started) return;
    _started = true;
    final generation = _generation;
    _onPcm = onPcm;
    _captureError = null;

    // Native -> Dart frames arrive on the dedicated events channel so its
    // handler never competes with the command channel's mock in tests.
    _events.setMethodCallHandler(_onNativeCall);
    try {
      // Kotlin replies with null on success or a user-facing reason on a mic
      // failure. Playback still runs in the latter case.
      final error = await _channel.invokeMethod<String?>('start');
      if (generation == _generation) _captureError = error;
    } on PlatformException catch (e) {
      if (generation == _generation) {
        _captureError = e.message ?? 'Mic unavailable (audio device error)';
      }
    } on MissingPluginException {
      if (generation == _generation) {
        _captureError = 'Mic unavailable (platform channel missing)';
      }
    }
  }

  /// Native -> Dart: one captured 320-sample frame as a byte array.
  @visibleForTesting
  Future<dynamic> onNativeCall(MethodCall call) => _onNativeCall(call);

  Future<dynamic> _onNativeCall(MethodCall call) async {
    if (call.method != 'pcm' || !_started) return null;
    final raw = call.arguments;
    if (raw is! Uint8List) return null;
    final expected = callAudioFrameSamples * 2;
    if (raw.lengthInBytes != expected) return null;

    // Copy so the view is aligned and owns its buffer; AudioRecord bytes are
    // little-endian, matching Dart's Int16List on Android.
    final bytes = Uint8List.fromList(raw);
    _onPcm?.call(Int16List.view(bytes.buffer, 0, callAudioFrameSamples));
    return null;
  }

  @override
  void play(List<int> pcm) {
    if (!_started || pcm.isEmpty) return;
    final bytes = Uint8List(pcm.length * 2);
    final view = ByteData.view(bytes.buffer);
    for (var i = 0; i < pcm.length; i++) {
      view.setInt16(i * 2, pcm[i], Endian.little);
    }
    unawaited(_channel.invokeMethod<void>('play', bytes));
  }

  @override
  Future<void> stop() async {
    if (!_started) return;
    // Invalidate an in-flight start() before clearing the devices it may still
    // be awaiting, so its generation check drops a late permission result.
    _generation++;
    _started = false;
    _onPcm = null;
    _events.setMethodCallHandler(null);
    await _channel.invokeMethod<void>('stop');
  }
}
