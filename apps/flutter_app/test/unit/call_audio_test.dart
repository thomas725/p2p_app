// Unit tests for the pure capture converter shared by the browser adapter.
//
// These run on the VM: no AudioContext, microphone or FFI is involved. The
// converter is where wire-format correctness lives — 16 kHz mono, 320 i16
// samples per 20 ms frame — so it is worth testing without a browser.

import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:p2p_app_flutter/src/util/call_audio.dart';

/// A continuous ramp: block `start..start+length` carries those source indices
/// as sample values, so a phase-preserving converter maps them straight to i16.
Float32List ramp(int length, [int start = 0]) => Float32List.fromList(
      List<double>.generate(length, (i) => (start + i) / 32767),
    );

void main() {
  group('PcmCaptureConverter', () {
    test('48 kHz decimates to one 320-sample i16 frame per 20 ms block', () {
      final converter = PcmCaptureConverter(48000);
      final frames = converter.feed(ramp(960));

      expect(frames, hasLength(1));
      expect(frames.single.length, callAudioFrameSamples);
      // Output sample j is source sample 3j; the ramp maps index -> i16 value.
      expect(frames.single[0], 0);
      expect(frames.single[1], 3);
      expect(frames.single[319], 957);
    });

    test('resampling phase carries across a short block boundary', () {
      final converter = PcmCaptureConverter(48000);
      expect(converter.feed(ramp(500)), isEmpty);
      final frames = converter.feed(ramp(460, 500));

      expect(frames, hasLength(1));
      expect(frames.single[0], 0);
      expect(frames.single[160], 480);
      expect(frames.single[319], 957);
    });

    test('successive blocks continue frame numbering without a phase reset', () {
      final converter = PcmCaptureConverter(48000);
      final first = converter.feed(ramp(960)).single;
      final second = converter.feed(ramp(960, 960)).single;

      // 960 source samples at 48 kHz is exactly 320 output samples, so the
      // second frame starts at source index 960, not back at zero.
      expect(first[319], 957);
      expect(second[0], 960);
      expect(second[1], 963);
    });

    test('handles a non-integer 44.1 kHz ratio', () {
      final converter = PcmCaptureConverter(44100);
      expect(converter.feed(ramp(441)), isEmpty);
      final frames = converter.feed(ramp(441, 441));

      expect(frames, hasLength(1));
      expect(frames.single.length, callAudioFrameSamples);
      // 2 * 441 / 2.75625 == 320 output samples; interpolation keeps them near
      // the corresponding source indices.
      expect(frames.single[0], closeTo(0, 2));
      expect(frames.single[100], closeTo(276, 3));
      expect(frames.single[319], closeTo(879, 3));
    });

    test('clips float samples into the i16 range', () {
      final converter = PcmCaptureConverter(16000);
      final frames = converter.feed(
        Float32List.fromList(<double>[
          2.0,
          -2.0,
          ...List<double>.filled(callAudioFrameSamples - 2, 0),
        ]),
      );

      expect(frames, hasLength(1));
      expect(frames.single[0], 32767);
      expect(frames.single[1], -32768);
    });

    test('empty input produces no frames and keeps the phase', () {
      final converter = PcmCaptureConverter(48000);
      expect(converter.feed(Float32List(0)), isEmpty);
      final frames = converter.feed(ramp(960));

      expect(frames, hasLength(1));
      expect(frames.single[0], 0);
    });
  });
}
