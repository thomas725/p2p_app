// The File shares detail route is shared by FRB and browser transports. This
// test drives the real Home wiring with FakeNodeApi and checks that a confirmed
// delete goes through NodeApi, then refreshes the already-open list route.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart' show MobileShare;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart' show MobilePeerStatus;
import 'package:p2p_app_flutter/src/screens/home.dart';

import '../helpers/fake_node_api.dart';

void main() {
  testWidgets('confirmed share delete uses NodeApi and refreshes the open list', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final previousApi = nodeApi;
    addTearDown(() => nodeApi = previousApi);

    var shares = [
      MobileShare(
        shareId: 'share-a',
        ownerUserId: 'user-owner',
        name: 'Docs',
        kind: 'private',
        publicWrite: false,
        localRoot: null,
        revision: 1,
      ),
    ];
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..networkSizeLabelOverride = ((_) async => 'Small')
      ..listSharesOverride = (() => List.unmodifiable(shares))
      ..loadShareManifestOverride = ((_) async => null)
      ..listShareGrantsOverride = ((_) async => const [])
      ..deleteShareOverride = (id) async {
        shares = [];
      };
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();

    await tester.tap(find.byIcon(Icons.settings));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('settings-shares')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-row-share-a')), findsOneWidget);
    await tester.tap(find.byKey(const Key('share-row-share-a')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-delete-submit')));
    await tester.pumpAndSettle();

    expect(
      fake.calls
          .where((call) => call.method == 'deleteShare')
          .map((call) => call.argument),
      ['share-a'],
    );
    expect(find.byKey(const Key('share-row-share-a')), findsNothing);
    expect(
      find.text(
        'No shares yet. Create one here, or wait for a peer share to be '
        'accepted.',
      ),
      findsOneWidget,
    );
  });
}
