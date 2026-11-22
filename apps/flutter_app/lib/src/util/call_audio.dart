// Platform-neutral device audio for a live call (calls lane, US-044).
//
// Rust owns the media session: it makes the μ-law frames, numbers them, buffers
// the peer's inbound frames and mutes outbound audio. This file owns only the
// *device* half that cannot live in Rust — capturing microphone PCM at 16 kHz
// mono and handing decoded peer PCM to a speaker. The concrete implementations
// are selected by the conditional import in `call_audio_device.dart`; the pure
// resampler below runs everywhere so widget/unit tests can exercise it without
// a browser or an audio device.

import 'dart:typed_data';

/// Capture/playback sample rate of the Rust media session (16 kHz mono).
const int callAudioSampleRate = 16000;

/// Samples in one 20 ms wire frame: `16000 * 0.02`.
const int callAudioFrameSamples = 320;

/// The device half of a live call.
///
/// A `null` [captureError] means the microphone is available (or that the
/// platform has no concept of the error — native stubs use that). A non-null
/// value is a short, user-facing reason capture failed, shown on the call
/// screen; playback may still work.
abstract class CallAudioAdapter {
  /// Why microphone capture is unavailable, or `null` when it is fine.
  String? get captureError => null;

  /// Open the devices. [onPcm] receives one [callAudioFrameSamples]-sample
  /// i16 frame per 20 ms of captured audio.
  Future<void> start({required void Function(List<int> pcm) onPcm});

  /// Hand one decoded frame from the peer to the speaker.
  void play(List<int> pcm);

  /// Release the devices. Called on every call-exit path, including `dispose`.
  Future<void> stop();
}

/// Resamples a capture stream to 16 kHz mono and chunks it into wire frames.
///
/// Browsers hand out Float32 PCM at their own context rate (usually 44.1 or
/// 48 kHz); Rust only accepts 320 i16 samples at 16 kHz. Feed successive
/// `onaudioprocess` buffers to [feed] and forward every returned frame to
/// `NodeApi.sendCallMedia`. A small carry keeps interpolation continuous across
/// block boundaries and works for rates above *or* below 16 kHz.
class PcmCaptureConverter {
  PcmCaptureConverter(this.sourceRate) : assert(sourceRate > 0);

  /// The context's sample rate, in Hz.
  final int sourceRate;

  Float32List _carry = Float32List(0);
  // Absolute source index of `_carry[0]`, and the absolute source position of
  // the next output sample. Both are needed so the fractional resampling phase
  // survives block boundaries instead of restarting at every buffer.
  int _base = 0;
  double _next = 0;
  final List<int> _pending = <int>[];

  /// Consume one float PCM block, returning zero or more 320-sample i16 frames.
  List<Int16List> feed(Float32List input) {
    final frames = <Int16List>[];
    if (input.isEmpty) return frames;

    final ratio = sourceRate / callAudioSampleRate;
    final all = Float32List(_carry.length + input.length);
    all.setRange(0, _carry.length, _carry);
    all.setRange(_carry.length, all.length, input);

    var lastIndex = 0;
    while (true) {
      final local = _next - _base;
      if (local >= all.length) break;
      final index = local.floor();
      final frac = local - index;
      final a = all[index];
      final b = index + 1 < all.length ? all[index + 1] : a;
      _pending.add(_toPcm16(a + (b - a) * frac));
      lastIndex = index;
      _next += ratio;
      if (_pending.length == callAudioFrameSamples) {
        frames.add(Int16List.fromList(_pending));
        _pending.clear();
      }
    }

    // Keep the last consumed sample so interpolation can continue into the
    // next block. For upsampling (sourceRate < 16 kHz) this is the sample the
    // next interval starts from; for the usual 44.1/48 kHz case it is simply
    // unused headroom.
    final keepFrom = lastIndex.clamp(0, all.length - 1);
    _carry = Float32List.fromList(all.sublist(keepFrom));
    _base += keepFrom;
    return frames;
  }

  static int _toPcm16(double sample) {
    final scaled = (sample * 32767).round();
    if (scaled > 32767) return 32767;
    if (scaled < -32768) return -32768;
    return scaled;
  }
}
