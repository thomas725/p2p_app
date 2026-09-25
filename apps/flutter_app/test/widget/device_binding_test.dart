import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/screens/home.dart';

void main() {
  const peerId = '12D3Koooodevice-peer';
  final devicePubkey = Uint8List.fromList([0x99, 0x01, 0x02]);

  Widget harness() => MaterialApp(
        home: Scaffold(
          body: Builder(builder: (context) => const Text('host')),
        ),
      );

  final accepted = <({String peerId, Uint8List devicePubkey})>[];

  Future<void> accepting(String id, Uint8List pubkey) async {
    accepted.add((peerId: id, devicePubkey: pubkey));
  }

  setUp(accepted.clear);

  testWidgets('Allow binds the incoming device into the account',
      (tester) async {
    await tester.pumpWidget(harness());
    final context = tester.element(find.byType(Text));
    final prompt = promptDeviceBinding(
      context,
      peerId: peerId,
      devicePubkey: devicePubkey,
      accept: accepting,
    );
    await tester.pumpAndSettle();

    expect(find.textContaining(peerId), findsOneWidget);
    expect(find.text('Link device to my account?'), findsOneWidget);

    await tester.tap(find.text('Allow'));
    await tester.pumpAndSettle();
    await prompt;

    expect(accepted, hasLength(1));
    expect(accepted.single.peerId, peerId);
    expect(accepted.single.devicePubkey, devicePubkey);
  });

  testWidgets('Decline leaves the account unchanged', (tester) async {
    await tester.pumpWidget(harness());
    final context = tester.element(find.byType(Text));
    final prompt = promptDeviceBinding(
      context,
      peerId: peerId,
      devicePubkey: devicePubkey,
      accept: accepting,
    );
    await tester.pumpAndSettle();

    await tester.tap(find.text('Decline'));
    await tester.pumpAndSettle();
    await prompt;

    expect(accepted, isEmpty);
    expect(
      find.text('Link device to my account?'),
      findsNothing,
      reason: 'dialog should be dismissed',
    );
  });
}