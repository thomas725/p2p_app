// Widget tests for the peer-info page's call affordance. The screen is a pure
// `StatelessWidget`, so these tests only assert the `onCall` wiring (button
// rendered when the callback is present, invoked with the peer, absent when
// not).

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';

MobilePeerRecord peer({String id = 'peer-abc'}) => MobilePeerRecord(
  peerId: id,
  firstSeen: '',
  lastSeen: '',
  displayName: 'Alice',
);

void main() {
  testWidgets('call button renders and invokes onCall with the peer',
      (tester) async {
    MobilePeerRecord? called;
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          onCall: (p) => called = p,
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('call-peer-info')));
    expect(called?.peerId, 'peer-abc');
  });

  testWidgets('no call button renders without onCall', (tester) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    expect(find.byKey(const Key('call-peer-info')), findsNothing);
  });
}