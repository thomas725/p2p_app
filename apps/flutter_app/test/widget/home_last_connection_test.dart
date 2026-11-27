// The disconnect event carries Rust's canonical display label in `content`.
// This test drives the real Home event sink and checks that Settings renders
// that label through the shared LiveStatus notifier, even when the peer never
// appears in the roster.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/home.dart';

import '../helpers/fake_node_api.dart';

void main() {
  testWidgets('a disconnect event supplies Settings with the Rust label', (
    tester,
  ) async {
    // Tall enough for the "Last connection lost" row without scrolling.
    tester.view.physicalSize = const Size(1000, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final previousApi = nodeApi;
    addTearDown(() => nodeApi = previousApi);
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..networkSizeLabelOverride = (_) async => 'Small';
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'peer_disconnected',
        peerId: '12D3KooWDepartedPeer',
        content: 'Brave Otter (er9)',
        encrypted: false,
      ),
    );
    await tester.pump();
    await tester.pump();

    await tester.tap(find.byIcon(Icons.settings));
    await tester.pump();
    await tester.pump();

    expect(find.text('Last connection lost'), findsOneWidget);
    expect(find.textContaining('Brave Otter (er9)'), findsOneWidget);
    expect(find.textContaining('12D3KooWDepartedPeer'), findsNothing);
  });
}
