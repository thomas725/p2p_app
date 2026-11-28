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
  String ownerUserId = 'user-owner',
  bool publicWrite = false,
}) => MobileShare(
  shareId: id,
  ownerUserId: ownerUserId,
  name: name,
  kind: kind,
  publicWrite: publicWrite,
  localRoot: localRoot,
  revision: revision,
);

Widget listApp({
  required List<MobileShare> shares,
  required void Function(MobileShare) onOpen,
  Future<void> Function(String name)? onCreate,
}) => MaterialApp(
  home: Scaffold(
    body: ShareList(
      shares: shares,
      onOpenShare: onOpen,
      onCreateShare: onCreate,
    ),
  ),
);

Widget detailApp({
  required Future<List<MobileShareGrant>> Function(String) loadGrants,
  Future<ShareManifest?> Function(String)? loadManifest,
  Future<void> Function(String, String, String)? onSetGrant,
  Future<void> Function(String, String)? onRemoveGrant,
  Future<void> Function(String)? onDeleteShare,
  Future<MobileShare> Function(String, String?)? onBindShare,
  Future<MobileShare> Function(String, String)? onSnapshotShare,
  Future<String?> Function()? pickDirectory,
  String? localRoot,
  int revision = 1,
}) => MaterialApp(
  home: ShareDetailScreen(
    share: makeShare(
      'share-a',
      name: 'Docs',
      kind: 'private',
      revision: revision,
      localRoot: localRoot,
    ),
    loadManifest: loadManifest ?? ((_) async => null),
    loadGrants: loadGrants,
    onSetGrant: onSetGrant,
    onRemoveGrant: onRemoveGrant,
    onDeleteShare: onDeleteShare,
    onBindShare: onBindShare,
    onSnapshotShare: onSnapshotShare,
    pickDirectory: pickDirectory,
  ),
);

void main() {
  test(
    'share action peers are the connected set in deterministic ID order',
    () {
      expect(
        connectedShareActionPeerIds(const ['peer-b', 'peer-a', 'peer-b']),
        ['peer-a', 'peer-b'],
      );
      expect(connectedShareActionPeerIds(const <String>[]), isEmpty);
    },
  );

  testWidgets('empty state explains where shares come from', (tester) async {
    await tester.pumpWidget(listApp(shares: const [], onOpen: (_) {}));
    await tester.pump();

    expect(find.text('Shares (0)'), findsOneWidget);
    expect(
      find.text(
        'No shares yet. Create one here, or wait for a peer share to be '
        'accepted.',
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

  testWidgets('list screen exposes discovery when Home supplies a callback', (
    tester,
  ) async {
    var taps = 0;
    await tester.pumpWidget(
      MaterialApp(
        home: ShareListScreen(
          shares: const [],
          onOpenShare: (_) {},
          onDiscoverShares: () => taps++,
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('shares-discover')), findsOneWidget);
    await tester.tap(find.byKey(const Key('shares-discover')));
    expect(taps, 1);
  });

  testWidgets(
    'list screen exposes creation only when Home supplies a callback',
    (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: ShareListScreen(shares: const [], onOpenShare: (_) {}),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('shares-create')), findsNothing);

      await tester.pumpWidget(
        MaterialApp(
          home: ShareListScreen(
            shares: const [],
            onOpenShare: (_) {},
            onCreateShare: (_) async {},
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const Key('shares-create')), findsOneWidget);
    },
  );

  testWidgets('list screen follows the share notifier while open', (
    tester,
  ) async {
    final rows = ValueNotifier<List<MobileShare>>([
      makeShare('share-a', name: 'Docs', kind: 'private'),
    ]);
    addTearDown(rows.dispose);
    await tester.pumpWidget(
      MaterialApp(
        home: ShareListScreen(
          shares: rows.value,
          shareRows: rows,
          onOpenShare: (_) {},
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Docs'), findsOneWidget);

    rows.value = const [];
    await tester.pumpAndSettle();

    expect(
      find.text(
        'No shares yet. Create one here, or wait for a peer share to be '
        'accepted.',
      ),
      findsOneWidget,
    );
  });

  testWidgets('create sends the trimmed name and closes the dialog', (
    tester,
  ) async {
    final names = <String>[];
    await tester.pumpWidget(
      listApp(
        shares: const [],
        onOpen: (_) {},
        onCreate: (name) async => names.add(name),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('shares-create')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('share-create-name')),
      '  Holiday photos  ',
    );
    await tester.tap(find.byKey(const Key('share-create-submit')));
    await tester.pumpAndSettle();

    expect(names, ['Holiday photos']);
    expect(find.byKey(const Key('share-create-name')), findsNothing);
  });

  testWidgets('empty name never calls the creator', (tester) async {
    final names = <String>[];
    await tester.pumpWidget(
      listApp(
        shares: const [],
        onOpen: (_) {},
        onCreate: (name) async => names.add(name),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('shares-create')));
    await tester.pumpAndSettle();
    await tester.enterText(find.byKey(const Key('share-create-name')), '   ');
    await tester.tap(find.byKey(const Key('share-create-submit')));
    await tester.pumpAndSettle();

    expect(names, isEmpty);
    expect(find.text('Create private share'), findsOneWidget);
  });

  testWidgets('create failure stays in the dialog with an error', (
    tester,
  ) async {
    await tester.pumpWidget(
      listApp(
        shares: const [],
        onOpen: (_) {},
        onCreate: (_) async => throw Exception('invalid name'),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('shares-create')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('share-create-name')),
      'Photos',
    );
    await tester.tap(find.byKey(const Key('share-create-submit')));
    await tester.pumpAndSettle();

    expect(find.text('Create private share'), findsOneWidget);
    expect(find.byKey(const Key('share-create-dialog-error')), findsOneWidget);
    expect(find.textContaining('Failed to create share'), findsOneWidget);
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

  testWidgets('bind uses the picked directory and shows Rust-returned root', (
    tester,
  ) async {
    final calls = <List<String?>>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        pickDirectory: () async => '/tmp/docs',
        onBindShare: (shareId, root) async {
          calls.add([shareId, root]);
          return makeShare(
            shareId,
            name: 'Docs',
            kind: 'private',
            localRoot: root,
          );
        },
        onSnapshotShare: (_, _) async => throw StateError('not used'),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Local folder'), findsOneWidget);
    expect(find.byKey(const Key('share-folder-bind')), findsOneWidget);
    expect(find.byKey(const Key('share-folder-clear')), findsNothing);

    await tester.tap(find.byKey(const Key('share-folder-bind')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', '/tmp/docs'],
    ]);
    expect(find.text('/tmp/docs'), findsOneWidget);
    expect(find.byKey(const Key('share-folder-clear')), findsOneWidget);
  });

  testWidgets('cancelling the directory picker never calls bind', (
    tester,
  ) async {
    var picks = 0;
    var binds = 0;
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        pickDirectory: () async {
          picks++;
          return null;
        },
        onBindShare: (_, _) async {
          binds++;
          return makeShare('share-a', name: 'Docs', kind: 'private');
        },
        onSnapshotShare: (_, _) async => throw StateError('not used'),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-folder-bind')));
    await tester.pumpAndSettle();

    expect(picks, 1);
    expect(binds, 0);
    expect(find.byKey(const Key('share-folder-error')), findsNothing);
  });

  testWidgets('clearing a bound folder sends a null root', (tester) async {
    final calls = <List<String?>>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        localRoot: '/tmp/docs',
        onBindShare: (shareId, root) async {
          calls.add([shareId, root]);
          return makeShare(
            shareId,
            name: 'Docs',
            kind: 'private',
            localRoot: root,
          );
        },
        onSnapshotShare: (_, _) async => throw StateError('not used'),
        pickDirectory: () async => '/tmp/never-picked',
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-folder-clear')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', null],
    ]);
    expect(find.text('Not bound on this device'), findsOneWidget);
    expect(find.byKey(const Key('share-folder-clear')), findsNothing);
  });

  testWidgets('snapshot uses the bound root and refreshes the detail', (
    tester,
  ) async {
    final calls = <List<String>>[];
    var manifestLoads = 0;
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        loadManifest: (_) async {
          manifestLoads++;
          return null;
        },
        localRoot: '/tmp/docs',
        revision: 2,
        onBindShare: (_, _) async => throw StateError('not used'),
        onSnapshotShare: (shareId, root) async {
          calls.add([shareId, root]);
          return makeShare(
            shareId,
            name: 'Docs',
            kind: 'private',
            revision: 3,
            localRoot: root,
          );
        },
      ),
    );
    await tester.pumpAndSettle();
    expect(manifestLoads, 1);

    await tester.tap(find.byKey(const Key('share-folder-snapshot')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', '/tmp/docs'],
    ]);
    expect(find.text('Revision'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(manifestLoads, 2);
  });

  testWidgets('snapshot without a bound root stays local and explains why', (
    tester,
  ) async {
    var snapshots = 0;
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onBindShare: (_, _) async => throw StateError('not used'),
        onSnapshotShare: (_, _) async {
          snapshots++;
          return makeShare('share-a', name: 'Docs', kind: 'private');
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-folder-snapshot')));
    await tester.pumpAndSettle();

    expect(snapshots, 0);
    expect(
      find.text('Bind a local folder before taking a snapshot.'),
      findsOneWidget,
    );
  });

  testWidgets('folder action failure stays inline', (tester) async {
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        pickDirectory: () async => '/tmp/docs',
        onBindShare: (_, _) async => throw Exception('not a directory'),
        onSnapshotShare: (_, _) async => throw StateError('not used'),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-folder-bind')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-folder-error')), findsOneWidget);
    expect(find.textContaining('Failed to bind folder'), findsOneWidget);
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

  testWidgets('owner can change visibility through the injected setter', (
    tester,
  ) async {
    final calls = <List<dynamic>>[];
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare('share-a', name: 'Docs', kind: 'private'),
          loadManifest: (_) async => null,
          loadGrants: (_) async => const [],
          ownUserId: 'user-owner',
          onSetVisibility: (shareId, kind, publicWrite) async {
            calls.add([shareId, kind, publicWrite]);
            return makeShare(
              shareId,
              name: 'Docs',
              kind: kind,
              publicWrite: publicWrite,
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Access'), findsOneWidget);
    expect(find.byKey(const Key('share-visibility-private')), findsOneWidget);
    expect(find.byKey(const Key('share-visibility-public')), findsOneWidget);
    expect(find.byKey(const Key('share-public-write')), findsNothing);

    await tester.tap(find.byKey(const Key('share-visibility-public')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', 'public', false],
    ]);
    expect(find.byKey(const Key('share-public-write')), findsOneWidget);
    expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);

    await tester.tap(find.byKey(const Key('share-public-write')));
    await tester.pumpAndSettle();
    expect(calls.last, ['share-a', 'public', true]);
    expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
  });

  testWidgets('peer-owned shares have no visibility controls', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: ShareDetailScreen(
          share: makeShare(
            'share-a',
            name: 'Docs',
            kind: 'private',
            ownerUserId: 'peer-owner',
          ),
          loadManifest: (_) async => null,
          loadGrants: (_) async => const [],
          ownUserId: 'user-owner',
          onSetVisibility: (_, _, _) async => throw StateError('must not run'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Access'), findsNothing);
    expect(find.byKey(const Key('share-visibility-private')), findsNothing);
    expect(find.byKey(const Key('share-visibility-public')), findsNothing);
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
    expect(find.text('Publish revision'), findsNothing);
    expect(find.text('Local folder'), findsNothing);
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

  testWidgets('publish editor submits the loaded manifest revision', (
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
        ),
        ShareEntry(
          path: 'docs/old.md',
          fileId: 'abcdef0123456788',
          size: BigInt.from(3),
        ),
      ],
    );
    final calls = <(String, BigInt, ShareManifest)>[];
    final revisions = <(String, List<String>)>[];
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
          loadGrants: (_) async => const [],
          peerIds: const ['peer-1'],
          onReviseManifest: (shareId, removedPaths) async {
            revisions.add((shareId, removedPaths));
            return ShareManifest(
              shareId: shareId,
              name: 'Docs',
              revision: BigInt.from(3),
              entries: [
                ShareEntry(
                  path: 'docs/readme.md',
                  fileId: 'abcdef0123456789',
                  size: BigInt.from(12),
                ),
              ],
            );
          },
          onPublish: (peerId, baseRevision, revised) async {
            calls.add((peerId, baseRevision, revised));
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.text('Publish revision'), findsOneWidget);
    await tester.ensureVisible(
      find.byKey(const Key('share-publish-delete-docs/old.md')),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-publish-delete-docs/old.md')));
    await tester.ensureVisible(find.byKey(const Key('share-publish-submit')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-publish-submit')));
    await tester.pumpAndSettle();

    expect(calls, hasLength(1));
    expect(calls.single.$1, 'peer-1');
    expect(calls.single.$2, BigInt.from(2));
    expect(calls.single.$3.entries.map((e) => e.path), ['docs/readme.md']);
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

  testWidgets('delete action is hidden without a Home callback', (
    tester,
  ) async {
    await tester.pumpWidget(detailApp(loadGrants: (_) async => const []));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-detail-delete')), findsNothing);
  });

  testWidgets('cancel keeps the share and never calls the deleter', (
    tester,
  ) async {
    final ids = <String>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onDeleteShare: (id) async => ids.add(id),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-detail-delete')));
    await tester.pumpAndSettle();
    expect(find.text('Delete "Docs"?'), findsOneWidget);
    expect(find.textContaining('Files in the bound folder'), findsOneWidget);

    await tester.tap(find.byKey(const Key('share-delete-cancel')));
    await tester.pumpAndSettle();

    expect(ids, isEmpty);
    expect(find.text('Share ID'), findsOneWidget);
  });

  testWidgets('confirmed delete calls the seam and leaves the detail route', (
    tester,
  ) async {
    final ids = <String>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onDeleteShare: (id) async => ids.add(id),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-delete-submit')));
    await tester.pumpAndSettle();

    expect(ids, ['share-a']);
    expect(find.byKey(const Key('share-detail-delete')), findsNothing);
    expect(find.text('Share ID'), findsNothing);
  });

  testWidgets('delete failure stays in the dialog with an error', (
    tester,
  ) async {
    final ids = <String>[];
    await tester.pumpWidget(
      detailApp(
        loadGrants: (_) async => const [],
        onDeleteShare: (id) async {
          ids.add(id);
          throw Exception('database locked');
        },
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('share-detail-delete')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-delete-submit')));
    await tester.pumpAndSettle();

    expect(ids, ['share-a']);
    expect(find.text('Delete "Docs"?'), findsOneWidget);
    expect(find.byKey(const Key('share-delete-dialog-error')), findsOneWidget);
    expect(find.textContaining('Failed to delete share'), findsOneWidget);
    expect(find.text('Share ID'), findsOneWidget);
  });
}
