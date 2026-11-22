// Widget tests for the call session screen `CallScreen`.
//
// The real send path calls the Rust `sendCallSignal` FFI function, which is
// not loaded in a `flutter test` harness. The widget therefore exposes a
// `@visibleForTesting` `sendSignalOverride` seam that replaces the FFI call
// (mirroring `ContactList.sortOverride`), and a `deliverEvent` seam that
// drives the exact event-sink handler installed by the poll loop. Sessions are
// pushed as a route and their end/decline actions assert the pop.

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/call_screen.dart';

import '../helpers/fake_node_api.dart';

SwarmEventJson callEvent(String type, {required String peerId, required String callId}) =>
    SwarmEventJson(
      eventType: type,
      peerId: peerId,
      content: callId,
      encrypted: false,
    );

SwarmEventJson mediaEvent(String peerId, String callId, List<int> pcm) =>
    SwarmEventJson(
      eventType: 'call_media',
      peerId: peerId,
      content: callId,
      audio: Int16List.fromList(pcm),
      encrypted: false,
    );

void main() {
  const peerId = 'peer-abc';
  const callId = 'call-123';

  // Media commands go through the shared `nodeApi`; a fake keeps existing
  // signaling tests off the unloaded Rust library.
  setUp(() => nodeApi = FakeNodeApi());

  testWidgets(
    'ringing shows Accept/Decline; Accept sends accept and reaches in-call',
    (tester) async {
      final sent = <String>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: CallScreen(
              peerId: peerId,
              displayName: 'Alice',
              callId: callId,
              initialPhase: CallScreenPhase.ringing,
              sendSignalOverride: (peerId, callId, kind) {
                sent.add(kind);
                return null;
              },
            ),
          ),
        ),
      );
      expect(find.text('Incoming call'), findsOneWidget);
      expect(find.text('Accept'), findsOneWidget);
      expect(find.text('Decline'), findsOneWidget);

      await tester.tap(find.byKey(const Key('accept')));
      await tester.pumpAndSettle();
      expect(sent, ['accept']);
      expect(find.text('In call'), findsOneWidget);
      expect(find.text('Hang up'), findsOneWidget);
    },
  );

  testWidgets('ringing Decline sends reject and pops the screen',
      (tester) async {
    final sent = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                key: const Key('open'),
                child: const Text('open'),
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => CallScreen(
                      peerId: peerId,
                      displayName: 'Alice',
                      callId: callId,
                      initialPhase: CallScreenPhase.ringing,
                      sendSignalOverride: (peerId, callId, kind) {
                        sent.add(kind);
                        return null;
                      },
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();
    expect(find.text('Decline'), findsOneWidget);

    await tester.tap(find.byKey(const Key('decline')));
    await tester.pumpAndSettle();
    expect(sent, ['reject']);
    expect(find.byType(CallScreen), findsNothing);
  });

  testWidgets('outgoing Cancel sends end and pops the screen',
      (tester) async {
    final sent = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                key: const Key('open'),
                child: const Text('open'),
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => CallScreen(
                      peerId: peerId,
                      displayName: 'Alice',
                      callId: callId,
                      initialPhase: CallScreenPhase.calling,
                      sendSignalOverride: (peerId, callId, kind) {
                        sent.add(kind);
                        return null;
                      },
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();

    expect(find.text('Calling'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);

    await tester.tap(find.byKey(const Key('cancel')));
    await tester.pumpAndSettle();
    expect(sent, ['end']);
    expect(find.byType(CallScreen), findsNothing);
  });

  testWidgets('in-call Hang up sends end and pops the screen',
      (tester) async {
    final sent = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                key: const Key('open'),
                child: const Text('open'),
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => CallScreen(
                      peerId: peerId,
                      displayName: 'Alice',
                      callId: callId,
                      initialPhase: CallScreenPhase.inCall,
                      sendSignalOverride: (peerId, callId, kind) {
                        sent.add(kind);
                        return null;
                      },
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();

    expect(find.text('In call'), findsOneWidget);
    await tester.tap(find.byKey(const Key('hang-up')));
    await tester.pumpAndSettle();
    expect(sent, ['end']);
    expect(find.byType(CallScreen), findsNothing);
  });

  testWidgets('call_accepted for this session transitions to in-call',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.calling,
            sendSignalOverride: (peerId, callId, kind) => null,
          ),
        ),
      ),
    );
    final state = tester.state<CallScreenState>(find.byType(CallScreen));
    state.deliverEvent(callEvent('call_accepted', peerId: peerId, callId: callId));
    await tester.pumpAndSettle();
    expect(find.text('In call'), findsOneWidget);
    expect(find.text('Hang up'), findsOneWidget);
  });

  testWidgets('call_rejected for this session pops the screen',
      (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                key: const Key('open'),
                child: const Text('open'),
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => CallScreen(
                      peerId: peerId,
                      displayName: 'Alice',
                      callId: callId,
                      initialPhase: CallScreenPhase.calling,
                      sendSignalOverride: (peerId, callId, kind) => null,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();

    final state = tester.state<CallScreenState>(find.byType(CallScreen));
    state.deliverEvent(
      callEvent('call_rejected', peerId: peerId, callId: callId),
    );
    await tester.pumpAndSettle();
    expect(find.byType(CallScreen), findsNothing);
  });

  testWidgets('foreign session invite is auto-declined and not adopted',
      (tester) async {
    final sent = <String>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.ringing,
            sendSignalOverride: (peerId, callId, kind) {
              sent.add('$kind:$peerId:$callId');
              return null;
            },
          ),
        ),
      ),
    );
    final state = tester.state<CallScreenState>(find.byType(CallScreen));
    final foreignId = 'call-other';
    // A second caller rings while we're already in a session: auto-decline.
    state.deliverEvent(
      callEvent('call_invite', peerId: 'peer-zzz', callId: foreignId),
    );
    await tester.pumpAndSettle();
    expect(sent, ['reject:peer-zzz:$foreignId']);
    // Our own session is untouched.
    expect(find.text('Incoming call'), findsOneWidget);

    // A session event for the other call also does not touch our state.
    state.deliverEvent(
      callEvent('call_accepted', peerId: 'peer-zzz', callId: foreignId),
    );
    await tester.pumpAndSettle();
    expect(find.text('Incoming call'), findsOneWidget);
  });

  testWidgets('non-call events (e.g. a DM) are forwarded downstream',
      (tester) async {
    final forwarded = <SwarmEventJson>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.ringing,
            sendSignalOverride: (peerId, callId, kind) => null,
          ),
        ),
      ),
    );
    final state = tester.state<CallScreenState>(find.byType(CallScreen));
    final dm = SwarmEventJson(
      eventType: 'dm',
      peerId: peerId,
      content: 'hello',
      encrypted: false,
    );
    // No previous sink is installed in the harness, so a DM reaching the
    // screen must not crash or mutate call state.
    state.deliverEvent(dm);
    await tester.pumpAndSettle();
    expect(find.text('Incoming call'), findsOneWidget);
    expect(forwarded, isEmpty);
  });

  // --- Rust-owned media transport (calls lane, US-044) ---

  testWidgets('accept opens the media session and hang-up closes it',
      (tester) async {
    final started = <(String, String)>[];
    var stops = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Builder(
            builder: (context) => Center(
              child: ElevatedButton(
                key: const Key('open'),
                child: const Text('open'),
                onPressed: () => Navigator.of(context).push(
                  MaterialPageRoute<void>(
                    builder: (_) => CallScreen(
                      peerId: peerId,
                      displayName: 'Alice',
                      callId: callId,
                      initialPhase: CallScreenPhase.ringing,
                      sendSignalOverride: (_, _, _) => null,
                      mediaOverrides: CallMediaOverrides(
                        start: (id, pid) => started.add((id, pid)),
                        stop: () => stops++,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('open')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('accept')));
    await tester.pumpAndSettle();
    expect(started, [(callId, peerId)]);
    expect(stops, 0);

    await tester.tap(find.byKey(const Key('hang-up')));
    await tester.pumpAndSettle();
    expect(stops, 1);
  });

  testWidgets('mute toggles the Rust session and the status line', (tester) async {
    final mutes = <bool>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.inCall,
            mediaOverrides: CallMediaOverrides(
              mute: (muted) => mutes.add(muted),
            ),
          ),
        ),
      ),
    );
    expect(find.textContaining('Mic on'), findsOneWidget);

    await tester.tap(find.byKey(const Key('mute-toggle')));
    await tester.pumpAndSettle();
    expect(mutes, [true]);
    expect(find.textContaining('Mic muted'), findsOneWidget);

    await tester.tap(find.byKey(const Key('mute-toggle')));
    await tester.pumpAndSettle();
    expect(mutes, [true, false]);
  });

  testWidgets('inbound PCM reaches the audio seam; other sessions are ignored',
      (tester) async {
    final audio = <List<int>>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.inCall,
            mediaOverrides: CallMediaOverrides(audio: audio.add),
          ),
        ),
      ),
    );
    final state = tester.state<CallScreenState>(find.byType(CallScreen));

    state.deliverEvent(mediaEvent('other-peer', callId, [1, 2, 3]));
    state.deliverEvent(mediaEvent(peerId, 'other-call', [4, 5, 6]));
    await tester.pump();
    expect(audio, isEmpty);

    final pcm = List<int>.filled(320, 7);
    state.deliverEvent(mediaEvent(peerId, callId, pcm));
    await tester.pump();
    expect(audio, hasLength(1));
    expect(audio.single, pcm);
    expect(find.textContaining('peer frames 1'), findsOneWidget);
  });

  testWidgets('sendPcm forwards 16 kHz blocks only while in call',
      (tester) async {
    final sent = <(String, List<int>)>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: CallScreen(
            peerId: peerId,
            displayName: 'Alice',
            callId: callId,
            initialPhase: CallScreenPhase.ringing,
            sendSignalOverride: (_, _, _) => null,
            mediaOverrides: CallMediaOverrides(
              start: (_, _) => null,
              stop: () => null,
              sendPcm: (id, pcm) => sent.add((id, pcm)),
            ),
          ),
        ),
      ),
    );
    final state = tester.state<CallScreenState>(find.byType(CallScreen));

    await state.sendPcm(List<int>.filled(320, 1));
    expect(sent, isEmpty);

    await tester.tap(find.byKey(const Key('accept')));
    await tester.pumpAndSettle();
    final pcm = List<int>.filled(320, 2);
    await state.sendPcm(pcm);
    expect(sent, hasLength(1));
    expect(sent.single.$1, callId);
    expect(sent.single.$2, pcm);
  });
}