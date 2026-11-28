// Browser device adapter for the Rust-owned call media session (US-044).
//
// Compiled only on the web via the conditional import in `call_audio_device.dart`.
// No audio plugin: capture is `getUserMedia` -> MediaStreamAudioSourceNode ->
// ScriptProcessorNode (resampled to 16 kHz mono i16 frames in Dart), playback is
// one AudioBufferSourceNode per decoded frame. Rust already numbers, buffers
// and mutes frames; this file only moves samples between Web Audio and Rust.
//
// `getUserMedia` needs a secure context, so an HTTP LAN deployment gets
// playback but not capture. That failure is reported through [captureError]
// rather than silently leaving the call one-way.

import 'dart:js_interop';
import 'dart:js_interop_unsafe';
import 'dart:typed_data';

import 'call_audio.dart';
import 'web_audio.dart';

@JS('navigator.mediaDevices.getUserMedia')
external JSPromise<JSAny?> _getUserMedia(JSObject constraints);

CallAudioAdapter createPlatformCallAudioAdapter() => _BrowserCallAudio();

class _BrowserCallAudio implements CallAudioAdapter {
  WebAudioContext? _playback;
  WebAudioContext? _captureContext;
  JSObject? _stream;
  JSObject? _source;
  JSObject? _processor;
  JSObject? _silentGain;
  void Function(List<int> pcm)? _onPcm;
  num _nextPlayAt = 0;
  bool _started = false;
  String? _captureError;
  // Bumped by stop() so an in-flight start() (e.g. waiting on the browser
  // permission prompt) can tell it was cancelled and tear down the just-opened
  // devices instead of opening the mic after the call has ended.
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

    // Playback works on plain HTTP (only getUserMedia needs a secure context),
    // so open it first and keep it even when capture is unavailable.
    final playback = WebAudioContext();
    if (playback.isSuspended) await playback.resume().toDart;
    if (generation != _generation) {
      // Hang-up while the context was resuming: don't leave it open.
      await playback.close().toDart;
      return;
    }
    _playback = playback;
    _nextPlayAt = playback.currentTime;

    try {
      final audio = JSObject();
      audio['echoCancellation'] = true.toJS;
      audio['noiseSuppression'] = true.toJS;
      audio['autoGainControl'] = true.toJS;
      final constraints = JSObject();
      constraints['audio'] = audio;
      final stream = (await _getUserMedia(constraints).toDart) as JSObject;
      if (generation != _generation) {
        // The permission prompt returned after hang-up. Stop the tracks it
        // granted so the microphone is released immediately.
        _stopTracks(stream);
        return;
      }

      final context = WebAudioContext();
      if (context.isSuspended) await context.resume().toDart;
      if (generation != _generation) {
        _stopTracks(stream);
        await context.close().toDart;
        return;
      }
      final source = context.createMediaStreamSource(stream);
      // ScriptProcessor must reach the destination to fire; a zero-gain node
      // keeps the graph alive without playing the captured audio back out.
      final processor = context.createScriptProcessor(4096, 1, 1);
      final silent = context.createGain();
      final gainParam = silent.getProperty('gain'.toJS) as JSObject;
      gainParam['value'] = 0.toJS;
      source.callMethod('connect'.toJS, processor);
      processor.callMethod('connect'.toJS, silent);
      silent.callMethod('connect'.toJS, context.destination);

      final captureRate = context.sampleRate.toInt();
      if (captureRate <= 0) {
        throw StateError('invalid AudioContext sample rate');
      }
      final converter = PcmCaptureConverter(captureRate);
      void handleCapture(JSObject event) {
        final input = event.getProperty('inputBuffer'.toJS);
        if (input == null || !input.isA<JSObject>()) return;
        final channel = (input as JSObject).callMethod(
          'getChannelData'.toJS,
          0.toJS,
        );
        if (channel == null || !channel.isA<JSFloat32Array>()) return;
        final floats = (channel as JSFloat32Array).toDart;
        for (final frame in converter.feed(floats)) {
          _onPcm?.call(frame);
        }
      }

      processor['onaudioprocess'] = handleCapture.toJS;

      _stream = stream;
      _captureContext = context;
      _source = source;
      _processor = processor;
      _silentGain = silent;
    } catch (e) {
      // A late rejection belongs to a call that's already over — don't surface
      // a stale mic error on the new call screen.
      if (generation == _generation) {
        _captureError = 'Mic unavailable (allow access, use HTTPS)';
      }
    }
  }

  @override
  void play(List<int> pcm) {
    final playback = _playback;
    if (playback == null || pcm.isEmpty) return;

    // Resync after a gap or a backlog: a late frame is worthless in a call,
    // and waking up minutes later must not queue a wall of stale audio.
    final now = playback.currentTime.toDouble();
    if (_nextPlayAt < now || _nextPlayAt > now + 0.5) _nextPlayAt = now;

    final buffer = playback.createBuffer(1, pcm.length, callAudioSampleRate);
    final floats = Float32List(pcm.length);
    for (var i = 0; i < pcm.length; i++) {
      floats[i] = pcm[i] / 32768.0;
    }
    buffer.copyToChannel(floats.toJS, 0);
    final node = playback.createBufferSource();
    node['buffer'] = buffer;
    node.callMethod('connect'.toJS, playback.destination);
    node.callMethod('start'.toJS, _nextPlayAt.toJS);
    _nextPlayAt += pcm.length / callAudioSampleRate;
  }

  @override
  Future<void> stop() async {
    // Invalidate any in-flight start() before touching the devices it may still
    // be about to create, so its generation checks make it bail out cleanly.
    _generation++;
    _started = false;
    _onPcm = null;

    _processor?['onaudioprocess'] = null;
    _processor?.callMethod('disconnect'.toJS);
    _source?.callMethod('disconnect'.toJS);
    _silentGain?.callMethod('disconnect'.toJS);
    _stopTracks(_stream);
    _processor = null;
    _source = null;
    _silentGain = null;
    _stream = null;

    // Clear the fields before awaiting close: play() must become a no-op the
    // instant we stop, not after a closing context could throw on a late frame.
    final captureContext = _captureContext;
    final playback = _playback;
    _captureContext = null;
    _playback = null;
    await captureContext?.close().toDart;
    await playback?.close().toDart;
  }

  /// Stop every track on a `MediaStream`, releasing the microphone hardware.
  void _stopTracks(JSObject? stream) {
    final tracks = stream?.callMethod('getTracks'.toJS);
    if (tracks != null && tracks.isA<JSArray<JSObject>>()) {
      for (final track in (tracks as JSArray<JSObject>).toDart) {
        track.callMethod('stop'.toJS);
      }
    }
  }
}
