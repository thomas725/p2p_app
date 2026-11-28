// Native Flutter share folders are wired through the same NodeApi facade as
// the rest of the share UI. This test drives real Home → Settings → File shares
// → detail navigation with FakeNodeApi and an injected directory picker, so it
// checks the transport gate and the Rust-boundary calls without FFI or plugins.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart'
    show BindShareParams, MobileShare;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart' show MobilePeerStatus;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareEntry, ShareManifest;
import 'package:p2p_app_flutter/src/screens/home.dart';
import 'package:p2p_app_flutter/src/util/share_folders.dart'
    show pickDirectoryOverride;

import '../helpers/fake_node_api.dart';

MobileShare _share({required String? root, required int revision}) =>
    MobileShare(
      shareId: 'share-a',
      ownerUserId: 'user-owner',
      name: 'Docs',
      kind: 'private',
      publicWrite: false,
      localRoot: root,
      revision: revision,
    );

void main() {
  testWidgets('native share detail binds and snapshots through NodeApi', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final previousApi = nodeApi;
    final previousPicker = pickDirectoryOverride;
    addTearDown(() => nodeApi = previousApi);
    addTearDown(() => pickDirectoryOverride = previousPicker);

    var share = _share(root: null, revision: 1);
    final fake = FakeNodeApi(isRemoteTransport: false)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..networkSizeLabelOverride = ((_) async => 'Small')
      ..listSharesOverride = (() => [share])
      ..loadShareManifestOverride = ((_) async => null)
      ..listShareGrantsOverride = ((_) async => const [])
      ..getShareOverride = ((id) async => id == share.shareId ? share : null)
      ..bindShareOverride = ((id, root) async {
        share = _share(root: root, revision: share.revision);
      })
      ..snapshotShareOverride = ((id, root) async {
        share = _share(root: root, revision: share.revision + 1);
        return ShareManifest(
          shareId: id,
          name: share.name,
          revision: BigInt.from(share.revision),
          entries: [
            ShareEntry(
              path: 'readme.md',
              fileId: 'abcdef0123456789',
              size: BigInt.from(12),
            ),
          ],
        );
      });
    nodeApi = fake;
    var picks = 0;
    pickDirectoryOverride = () async {
      picks++;
      return '/tmp/native-share';
    };

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();

    await tester.tap(find.byIcon(Icons.settings));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('settings-shares')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-row-share-a')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-folder-bind')), findsOneWidget);
    await tester.tap(find.byKey(const Key('share-folder-bind')));
    await tester.pumpAndSettle();

    expect(picks, 1);
    final bindCalls = fake.calls
        .where((call) => call.method == 'bindShare')
        .map((call) => call.argument as BindShareParams)
        .toList();
    expect(bindCalls, hasLength(1));
    expect(bindCalls.single.shareId, 'share-a');
    expect(bindCalls.single.localRoot, '/tmp/native-share');
    expect(find.text('/tmp/native-share'), findsOneWidget);

    await tester.tap(find.byKey(const Key('share-folder-snapshot')));
    await tester.pumpAndSettle();

    final snapshotCalls = fake.calls
        .where((call) => call.method == 'snapshotShare')
        .map((call) => call.argument as ({String shareId, String root}))
        .toList();
    expect(snapshotCalls, hasLength(1));
    expect(snapshotCalls.single.shareId, 'share-a');
    expect(snapshotCalls.single.root, '/tmp/native-share');
    expect(find.text('Revision'), findsOneWidget);
    expect(find.text('2'), findsWidgets);
  });
}
