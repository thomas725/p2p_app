// Widget tests for the file-share list and detail routes, including the
// optional private-share grant editor.
//
// Both widgets are pure UI: the list takes share rows from Home, and the detail
// page takes injected manifest/grant loaders and edit callbacks (the same seam
// pattern as PeerInfoScreen). No Rust library or NodeApi is needed here.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/api.dart'
    show MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareEntry, ShareManifest;
import 'package:p2p_app_flutter/src/screens/share_list.dart';

MobileShare makeShare(
  String id, {
  required String name,
  required String kind,
  int revision = 1,
  String? localRoot,
}) => MobileShare(
  shareId: id,
  ownerUserId: 'user-owner',
  name: name,
  kind: kind,
  publicWrite: false,
  localRoot: localRoot,
  revision: revision,
);

Widget listApp({
  required List<MobileShare> shares,
  required void Function(MobileShare) onOpen,
}) => MaterialApp(
  home: Scaffold(
    body: ShareList(shares: shares, onOpenShare: onOpen),
  ),
);

Widget detailApp({
  required Future<List<MobileShareGrant>> Function(String) loadGrants,
  Future<void> Function(String, String, String)? onSetGrant,
  Future<void> Function(String, String)? onRemoveGrant,
}) => MaterialApp(
  home: ShareDetailScreen(
    share: makeShare('share-a', name: 'Docs', kind: 'private'),
    loadManifest: (_) async => null,
    loadGrants: loadGrants,
    onSetGrant: onSetGrant,
    onRemoveGrant: onRemoveGrant,
  ),
);

void main() {
  testWidgets('empty state explains where shares come from', (tester) async {
    await tester.pumpWidget(listApp(shares: const [], onOpen: (_) {}));
    await tester.pump();

    expect(find.text('Shares (0)'), findsOneWidget);
    expect(
      find.text(
        'No shares yet. Shares appear when the node creates or accepts one.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('renders share rows and opens the tapped share', (tester) async {
    final opened = <MobileShare>[];
    await tester.pumpWidget(
      listApp(
        shares: [
          makeShare('share-a', name: 'Docs', kind: 'private', revision: 2),
          makeShare(
            'share-b',
            name: 'Photos',
            kind: 'public',
            localRoot: '/storage/photos',
          ),
        ],
        onOpen: opened.add,
      ),
    );
    await tester.pump();

    expect(find.text('Shares (2)'), findsOneWidget);
    expect(find.text('Docs'), findsOneWidget);
    expect(find.text('private · rev 2'), findsOneWidget);
    expect(find.text('public · rev 1 · bound'), findsOneWidget);

    await tester.tap(find.byKey(const Key('share-row-share-b')));
    await tester.pump();
    expect(opened.single.shareId, 'share-b');
  });

  testWidgets('detail shows overview, grants and manifest files', (
    tester,
  ) async {
    final manifest = ShareManifest(
      shareId: 'share-a',
      name: 'Docs',
      revision: BigInt.from(2),
      entries: [
        ShareEntry(
          path: 'docs/readme.md',
          fileId: 'abcdef0123456789',
          size: BigInt.from(12),
          mime: 'text/plain',
        ),
      ],
    );
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare(
            'share-a',
            name: 'Docs',
            kind: 'private',
            revision: 2,
          ),
          loadManifest: (_) async => manifest,
          loadGrants: (_) async => const [
            MobileShareGrant(userId: 'user-friend', mode: 'readonly'),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Share ID'), findsOneWidget);
    expect(find.text('share-a'), findsOneWidget);
    expect(find.text('Owner'), findsOneWidget);
    expect(find.text('user-owner'), findsOneWidget);
    expect(find.text('Revision'), findsOneWidget);
    expect(find.text('Not bound on this device'), findsOneWidget);
    expect(find.text('Grants'), findsOneWidget);
    expect(find.text('user-friend'), findsOneWidget);
    expect(find.text('readonly'), findsOneWidget);
    expect(find.text('Manifest files'), findsOneWidget);
    expect(find.text('docs/readme.md'), findsOneWidget);
    expect(find.text('12 B · abcdef01…'), findsOneWidget);
  });

  testWidgets('public shares do not render a grants section', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare('share-b', name: 'Photos', kind: 'public'),
          loadManifest: (_) async => null,
          loadGrants: (_) async => const [],
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Grants'), findsNothing);
    expect(find.text('No manifest received yet.'), findsOneWidget);
  });

  testWidgets('refresh button re-runs both loaders', (tester) async {
    var manifestLoads = 0;
    var grantLoads = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare('share-a', name: 'Docs', kind: 'private'),
          loadManifest: (_) async {
            manifestLoads++;
            return null;
          },
          loadGrants: (_) async {
            grantLoads++;
            return const [];
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(manifestLoads, 1);
    expect(grantLoads, 1);

    await tester.tap(find.byKey(const Key('share-detail-refresh')));
    await tester.pumpAndSettle();
    expect(manifestLoads, 2);
    expect(grantLoads, 2);
  });

  testWidgets('detail without edit callbacks stays read-only', (tester) async {
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [
          MobileShareGrant(userId: 'user-friend', mode: 'readonly'),
        ],
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Grants'), findsOneWidget);
    expect(find.byKey(const Key('share-grant-add')), findsNothing);
    expect(
      find.byKey(const Key('share-grant-revoke-user-friend')),
      findsNothing,
    );
    expect(find.text('Network'), findsNothing);
  });

  testWidgets('network actions reload the detail after success', (
    tester,
  ) async {
    var manifestLoads = 0;
    var grantLoads = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare('share-a', name: 'Docs', kind: 'private'),
          loadManifest: (_) async {
            manifestLoads++;
            return null;
          },
          loadGrants: (_) async {
            grantLoads++;
            return const [];
          },
          peerIds: const ['peer-1'],
          onRequestManifest: (_, _) async {},
          onReadvertise: (_, _) async {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Network'), findsOneWidget);
    expect(find.byKey(const Key('share-network-refresh')), findsOneWidget);
    expect(find.byKey(const Key('share-network-readvertise')), findsOneWidget);
    expect(manifestLoads, 1);
    expect(grantLoads, 1);

    await tester.ensureVisible(find.byKey(const Key('share-network-refresh')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-network-refresh')));
    await tester.pumpAndSettle();
    expect(manifestLoads, 2);
    expect(grantLoads, 2);
  });

  testWidgets('add grant sends the trimmed user and selected mode', (
    tester,
  ) async {
    final calls = <List<String>>[];
    var grants = <MobileShareGrant>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => List.unmodifiable(grants),
        onSetGrant: (shareId, userId, mode) async {
          calls.add([shareId, userId, mode]);
          grants = [MobileShareGrant(userId: userId, mode: mode)];
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-grant-add')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('share-grant-user-id')),
      '  user-friend  ',
    );
    await tester.tap(find.byKey(const Key('share-grant-mode')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('readwrite').last);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-grant-submit')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', 'user-friend', 'readwrite'],
    ]);
    expect(find.text('user-friend'), findsOneWidget);
    expect(find.text('readwrite'), findsOneWidget);
  });

  testWidgets('empty user ID never calls the editor', (tester) async {
    final calls = <List<String>>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onSetGrant: (shareId, userId, mode) async {
          calls.add([shareId, userId, mode]);
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-grant-add')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('share-grant-user-id')), '   ');
    await tester.tap(find.byKey(const Key('share-grant-submit')));
    await tester.pumpAndSettle();

    expect(calls, isEmpty);
    expect(find.text('Add share grant'), findsOneWidget);
  });

  testWidgets('revoke grant calls the editor and reloads the list', (
    tester,
  ) async {
    final calls = <List<String>>[];
    var grants = [
      const MobileShareGrant(userId: 'user-friend', mode: 'readonly'),
    ];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => List.unmodifiable(grants),
        onRemoveGrant: (shareId, userId) async {
          calls.add([shareId, userId]);
          grants = [];
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-grant-revoke-user-friend')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', 'user-friend'],
    ]);
    expect(find.text('No explicit grants.'), findsOneWidget);
  });

  testWidgets('add failure stays in the dialog with an error', (tester) async {
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onSetGrant: (_, _, _) async => throw Exception('unknown user'),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-grant-add')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('share-grant-user-id')),
      'user-friend',
    );
    await tester.tap(find.byKey(const Key('share-grant-submit')));
    await tester.pumpAndSettle();

    expect(find.text('Add share grant'), findsOneWidget);
    expect(find.byKey(const Key('share-grant-dialog-error')), findsOneWidget);
    expect(find.textContaining('Failed to add grant'), findsOneWidget);
  });

  testWidgets('revoke failure shows an inline error', (tester) async {
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [
          MobileShareGrant(userId: 'user-friend', mode: 'readonly'),
        ],
        onRemoveGrant: (_, _) async => throw Exception('database locked'),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-grant-revoke-user-friend')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-grant-error')), findsOneWidget);
    expect(find.textContaining('Failed to revoke user-friend'), findsOneWidget);
    expect(find.text('user-friend'), findsOneWidget);
  });
}
