// Unit tests for the Android call-audio adapter's channel contract.
//
// The native side is mocked with `setMockMethodCallHandler`, and captured PCM is
// delivered as a real platform message on the events channel. No Android
// runtime, AudioRecord or Rust library is involved.

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:p2p_app_flutter/src/util/call_audio.dart';
import 'package:p2p_app_flutter/src/util/call_audio_platform_android.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const commandChannel = MethodChannel(callAudioChannelName);
  final messenger = TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  late List<MethodCall> commands;
  late String? startResult;
  late Object? startError;

  setUp(() {
    commands = <MethodCall>[];
    startResult = null;
    startError = null;
    messenger.setMockMethodCallHandler(commandChannel, (call) async {
      commands.add(call);
      if (call.method == 'start') {
        if (startError != null) throw startError!;
        return startResult;
      }
      return null;
    });
  });

  tearDown(() {
    messenger.setMockMethodCallHandler(commandChannel, null);
  });

  /// Encode a captured frame exactly as `CallAudio.kt` does and deliver it.
  Future<void> deliverPcm(List<int> samples) async {
    final bytes = Uint8List(samples.length * 2);
    final view = ByteData.view(bytes.buffer);
    for (var i = 0; i < samples.length; i++) {
      view.setInt16(i * 2, samples[i], Endian.little);
    }
    await TestWidgetsFlutterBinding.instance.defaultBinaryMessenger
        .handlePlatformMessage(
      callAudioEventsChannelName,
      const StandardMethodCodec()
          .encodeMethodCall(MethodCall('pcm', bytes)),
      (_) {},
    );
  }

  test('start invokes the native command and has no error on success', () async {
    final adapter = AndroidCallAudio();
    final captured = <List<int>>[];

    await adapter.start(onPcm: captured.add);

    expect(commands, hasLength(1));
    expect(commands.single.method, 'start');
    expect(adapter.captureError, isNull);
  });

  test('a native mic reason is surfaced as captureError', () async {
    startResult = 'Mic unavailable (allow microphone access)';
    final adapter = AndroidCallAudio();

    await adapter.start(onPcm: (_) {});

    expect(adapter.captureError, startResult);
  });

  test('platform errors are mapped to a user-facing reason', () async {
    startError = PlatformException(code: 'UNAVAILABLE', message: 'Activity gone');
    final adapter = AndroidCallAudio();

    await adapter.start(onPcm: (_) {});

    expect(adapter.captureError, 'Activity gone');
  });

  test('a missing plugin is reported instead of crashing the call', () async {
    startError = MissingPluginException('No implementation for $callAudioChannelName');
    final adapter = AndroidCallAudio();

    await adapter.start(onPcm: (_) {});

    expect(adapter.captureError, 'Mic unavailable (platform channel missing)');
  });

  test('captured frames are delivered as 320 little-endian i16 samples', () async {
    final adapter = AndroidCallAudio();
    final captured = <List<int>>[];
    await adapter.start(onPcm: captured.add);

    final samples = List<int>.generate(callAudioFrameSamples, (i) => i - 160);
    await deliverPcm(samples);

    expect(captured, hasLength(1));
    expect(captured.single, samples);
  });

  test('a malformed frame is ignored', () async {
    final adapter = AndroidCallAudio();
    final captured = <List<int>>[];
    await adapter.start(onPcm: captured.add);

    await deliverPcm(<int>[1, 2, 3]);

    expect(captured, isEmpty);
  });

  test('play sends little-endian bytes to the native command channel', () async {
    final adapter = AndroidCallAudio();
    await adapter.start(onPcm: (_) {});

    adapter.play(<int>[-1, 0, 1234]);

    expect(commands.map((c) => c.method), ['start', 'play']);
    final bytes = commands.last.arguments as Uint8List;
    final view = ByteData.view(bytes.buffer, bytes.offsetInBytes, bytes.lengthInBytes);
    expect(view.getInt16(0, Endian.little), -1);
    expect(view.getInt16(2, Endian.little), 0);
    expect(view.getInt16(4, Endian.little), 1234);
  });

  test('play is a no-op before start', () async {
    final adapter = AndroidCallAudio();

    adapter.play(<int>[1, 2]);

    expect(commands, isEmpty);
  });

  test('stop clears the handler so a late frame is ignored', () async {
    final adapter = AndroidCallAudio();
    final captured = <List<int>>[];
    await adapter.start(onPcm: captured.add);
    await adapter.stop();

    await deliverPcm(List<int>.filled(callAudioFrameSamples, 1));

    expect(commands.map((c) => c.method), ['start', 'stop']);
    expect(captured, isEmpty);
  });

  test('a start that settles after stop drops its late error', () async {
    final gate = Completer<String?>();
    messenger.setMockMethodCallHandler(commandChannel, (call) async {
      commands.add(call);
      return call.method == 'start' ? gate.future : null;
    });
    final adapter = AndroidCallAudio();

    final starting = adapter.start(onPcm: (_) {});
    await adapter.stop();
    gate.complete('Mic unavailable (allow microphone access)');
    await starting;

    expect(adapter.captureError, isNull);
  });
}
