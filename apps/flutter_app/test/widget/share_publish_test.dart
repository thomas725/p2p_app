// Widget tests for publishing a manifest deletion. The widget owns only the
// selection/peer/error state; every node call is an injected callback, so no
// FFI or live peer is needed.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/shares_json.dart'
    show shareRevisionToPlatformInt64;
import 'package:p2p_app_flutter/src/rust/api.dart' show MobileShare;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareEntry, ShareManifest;
import 'package:p2p_app_flutter/src/screens/share_publish.dart';

MobileShare makeShare({String id = 'share-a', int revision = 2}) => MobileShare(
  shareId: id,
  ownerUserId: 'user-owner',
  name: 'Docs',
  kind: 'private',
  publicWrite: false,
  localRoot: null,
  revision: revision,
);

ShareManifest makeManifest({
  String id = 'share-a',
  int revision = 2,
  List<String> paths = const ['docs/a.md', 'docs/b.md'],
}) => ShareManifest(
  shareId: id,
  name: 'Docs',
  revision: BigInt.from(revision),
  entries: [
    for (final path in paths)
      ShareEntry(
        path: path,
        fileId: 'abcdef0123456789',
        size: BigInt.from(12),
        mime: 'text/plain',
      ),
  ],
);

Widget publishApp({
  MobileShare? share,
  required ShareManifest? manifest,
  required List<String> peerIds,
  Future<ShareManifest> Function(String, List<String>)? onReviseManifest,
  required Future<void> Function(String, BigInt, ShareManifest) onPublish,
  Future<void> Function()? onCompleted,
}) => MaterialApp(
  home: Scaffold(
    body: SingleChildScrollView(
      child: SharePublishSection(
        share: share ?? makeShare(),
        manifest: manifest,
        peerIds: peerIds,
        onReviseManifest:
            onReviseManifest ??
            (_, _) async => throw Exception('unexpected revision call'),
        onPublish: onPublish,
        onCompleted: onCompleted,
      ),
    ),
  ),
);

Future<void> selectPeer(WidgetTester tester, String peerId) async {
  await tester.tap(find.byKey(const Key('share-publish-peer')));
  await tester.pumpAndSettle();
  await tester.tap(find.text(peerId).last);
  await tester.pumpAndSettle();
}

Future<void> selectFile(WidgetTester tester, String path) async {
  await tester.tap(find.byKey(Key('share-publish-delete-$path')));
  await tester.pumpAndSettle();
}

Future<void> submit(WidgetTester tester) async {
  await tester.ensureVisible(find.byKey(const Key('share-publish-submit')));
  await tester.pumpAndSettle();
  await tester.tap(find.byKey(const Key('share-publish-submit')));
  await tester.pumpAndSettle();
}

void main() {
  test('revision conversion stays at the transport boundary', () {
    expect(shareRevisionToPlatformInt64(BigInt.from(42)), 42);
  });

  testWidgets('missing manifest is not publishable', (tester) async {
    await tester.pumpWidget(
      publishApp(
        manifest: null,
        peerIds: const ['peer-1'],
        onPublish: (_, _, _) async => throw Exception('unexpected'),
      ),
    );
    await tester.pump();

    expect(
      find.text('No manifest received yet. Refresh before publishing.'),
      findsOneWidget,
    );
    expect(find.byKey(const Key('share-publish-submit')), findsNothing);
  });

  testWidgets('manifest from another revision is marked stale', (tester) async {
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(revision: 7),
        peerIds: const ['peer-1'],
        onPublish: (_, _, _) async => throw Exception('unexpected'),
      ),
    );
    await tester.pump();

    expect(find.byKey(const Key('share-publish-stale')), findsOneWidget);
    expect(find.textContaining('Manifest is stale'), findsOneWidget);
    expect(find.byKey(const Key('share-publish-submit')), findsNothing);
  });

  testWidgets('no peer disables publishing', (tester) async {
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const [],
        onPublish: (_, _, _) async => throw Exception('unexpected'),
      ),
    );
    await tester.pump();

    expect(
      find.text('No connected peers. Publishing needs a peer.'),
      findsOneWidget,
    );
    expect(find.byKey(const Key('share-publish-peer')), findsNothing);
    final button = tester.widget<FilledButton>(
      find.byKey(const Key('share-publish-submit')),
    );
    expect(button.onPressed, isNull);
  });

  testWidgets('publish sends peer, base revision and deletion manifest', (
    tester,
  ) async {
    final calls = <(String, BigInt, ShareManifest)>[];
    final revisions = <(String, List<String>)>[];
    var completions = 0;
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-1', 'peer-2'],
        onReviseManifest: (shareId, removedPaths) async {
          revisions.add((shareId, removedPaths));
          return makeManifest(revision: 3, paths: const ['docs/a.md']);
        },
        onPublish: (peerId, base, manifest) async {
          calls.add((peerId, base, manifest));
        },
        onCompleted: () async => completions++,
      ),
    );
    await tester.pump();

    await selectPeer(tester, 'peer-2');
    await selectFile(tester, 'docs/b.md');
    await submit(tester);

    expect(revisions, hasLength(1));
    expect(revisions.single.$1, 'share-a');
    expect(revisions.single.$2, orderedEquals(['docs/b.md']));
    expect(calls, hasLength(1));
    expect(calls.single.$1, 'peer-2');
    expect(calls.single.$2, BigInt.from(2));
    expect(calls.single.$3.revision, BigInt.from(3));
    expect(calls.single.$3.entries.map((e) => e.path), ['docs/a.md']);
    expect(completions, 1);
    expect(find.byKey(const Key('share-publish-status')), findsOneWidget);
  });

  testWidgets('stale peer selection resets when the roster changes', (
    tester,
  ) async {
    final calls = <(String, BigInt, ShareManifest)>[];
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-1', 'peer-2'],
        onReviseManifest: (_, paths) async =>
            makeManifest(revision: 3, paths: paths),
        onPublish: (peerId, _, _) async {
          calls.add((peerId, BigInt.zero, makeManifest()));
        },
      ),
    );
    await tester.pump();
    await selectPeer(tester, 'peer-2');

    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-3', 'peer-4'],
        onReviseManifest: (_, paths) async =>
            makeManifest(revision: 3, paths: paths),
        onPublish: (peerId, _, _) async {
          calls.add((peerId, BigInt.zero, makeManifest()));
        },
      ),
    );
    await tester.pumpAndSettle();

    await selectFile(tester, 'docs/a.md');
    await submit(tester);

    expect(calls.single.$1, 'peer-3');
  });

  testWidgets('refreshed manifest clears deletions from the old revision', (
    tester,
  ) async {
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-1'],
        onPublish: (_, _, _) async {},
      ),
    );
    await tester.pump();
    await selectFile(tester, 'docs/b.md');
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const Key('share-publish-delete-docs/b.md')),
          )
          .value,
      isTrue,
    );

    await tester.pumpWidget(
      publishApp(
        share: makeShare(revision: 3),
        manifest: makeManifest(revision: 3),
        peerIds: const ['peer-1'],
        onPublish: (_, _, _) async {},
      ),
    );
    await tester.pumpAndSettle();

    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const Key('share-publish-delete-docs/b.md')),
          )
          .value,
      isFalse,
    );
  });

  testWidgets('revision failure blocks publishing and keeps the selection', (
    tester,
  ) async {
    final calls = <(String, BigInt, ShareManifest)>[];
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-1'],
        onReviseManifest: (_, _) async => throw Exception('unsafe path'),
        onPublish: (peerId, base, manifest) async {
          calls.add((peerId, base, manifest));
        },
      ),
    );
    await tester.pump();
    await selectFile(tester, 'docs/a.md');
    await submit(tester);

    expect(calls, isEmpty);
    expect(find.byKey(const Key('share-publish-error')), findsOneWidget);
    expect(find.textContaining('Publish failed'), findsOneWidget);
    expect(find.textContaining('unsafe path'), findsOneWidget);
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const Key('share-publish-delete-docs/a.md')),
          )
          .value,
      isTrue,
    );
  });

  testWidgets('publish failure shows an inline error and keeps the selection', (
    tester,
  ) async {
    await tester.pumpWidget(
      publishApp(
        manifest: makeManifest(),
        peerIds: const ['peer-1'],
        onReviseManifest: (_, paths) async =>
            makeManifest(revision: 3, paths: paths),
        onPublish: (_, _, _) async => throw Exception('stale revision'),
      ),
    );
    await tester.pump();
    await selectFile(tester, 'docs/a.md');
    await submit(tester);

    expect(find.byKey(const Key('share-publish-error')), findsOneWidget);
    expect(find.textContaining('Publish failed'), findsOneWidget);
    expect(find.textContaining('stale revision'), findsOneWidget);
    expect(
      tester
          .widget<CheckboxListTile>(
            find.byKey(const Key('share-publish-delete-docs/a.md')),
          )
          .value,
      isTrue,
    );
  });
}
