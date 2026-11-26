// Widget tests for the share detail network actions. The widget only talks to
// injected callbacks, so no NodeApi, Rust library or live peer is needed.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/api.dart' show MobileShare;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareKind, ShareOffer;
import 'package:p2p_app_flutter/src/screens/share_network.dart';

MobileShare makeShare() => const MobileShare(
  shareId: 'share-a',
  ownerUserId: 'user-owner',
  name: 'Docs',
  kind: 'private',
  publicWrite: false,
  localRoot: null,
  revision: 4,
);

Widget networkApp({
  required List<String> peerIds,
  required Future<void> Function(String shareId, String peerId)
  onRequestManifest,
  required Future<void> Function(String peerId, ShareOffer offer) onReadvertise,
  Future<void> Function()? onCompleted,
}) => MaterialApp(
  home: Scaffold(
    body: ShareNetworkSection(
      share: makeShare(),
      peerIds: peerIds,
      onRequestManifest: onRequestManifest,
      onReadvertise: onReadvertise,
      onCompleted: onCompleted,
    ),
  ),
);

Future<void> selectPeer(WidgetTester tester, String peerId) async {
  await tester.tap(find.byKey(const Key('share-network-peer')));
  await tester.pumpAndSettle();
  await tester.tap(find.text(peerId).last);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('no peers explains why actions are unavailable', (tester) async {
    await tester.pumpWidget(
      networkApp(
        peerIds: const [],
        onRequestManifest: (_, _) async {},
        onReadvertise: (_, _) async {},
      ),
    );
    await tester.pump();

    expect(
      find.text('No connected peers. Network actions need a peer.'),
      findsOneWidget,
    );
    expect(find.byKey(const Key('share-network-peer')), findsNothing);
    expect(find.byKey(const Key('share-network-refresh')), findsNothing);
  });

  testWidgets('manifest refresh sends the share id and chosen peer', (
    tester,
  ) async {
    final calls = <List<String>>[];
    var completions = 0;
    await tester.pumpWidget(
      networkApp(
        peerIds: const ['peer-1', 'peer-2'],
        onRequestManifest: (shareId, peerId) async {
          calls.add([shareId, peerId]);
        },
        onReadvertise: (_, _) async {},
        onCompleted: () async => completions++,
      ),
    );
    await tester.pump();

    await selectPeer(tester, 'peer-2');
    await tester.tap(find.byKey(const Key('share-network-refresh')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', 'peer-2'],
    ]);
    expect(completions, 1);
  });

  testWidgets('readvertise sends a canonical offer without owner sentinel', (
    tester,
  ) async {
    final calls = <(String, ShareOffer)>[];
    await tester.pumpWidget(
      networkApp(
        peerIds: const ['peer-1', 'peer-2'],
        onRequestManifest: (_, _) async {},
        onReadvertise: (peerId, offer) async {
          calls.add((peerId, offer));
        },
      ),
    );
    await tester.pump();

    await selectPeer(tester, 'peer-2');
    await tester.tap(find.byKey(const Key('share-network-readvertise')));
    await tester.pumpAndSettle();

    expect(calls.length, 1);
    final (peerId, offer) = calls.single;
    expect(peerId, 'peer-2');
    expect(offer.shareId, 'share-a');
    expect(offer.name, 'Docs');
    expect(offer.kind, ShareKind.private);
    expect(offer.revision, BigInt.from(4));
    expect(offer.publicWrite, isFalse);
    expect(offer.ownerUserId, isNull);
  });

  testWidgets('a stale peer selection resets when peers change', (
    tester,
  ) async {
    final calls = <List<String>>[];
    var peerIds = ['peer-1', 'peer-2'];
    late StateSetter rebuild;
    await tester.pumpWidget(
      StatefulBuilder(
        builder: (context, setState) {
          rebuild = setState;
          return networkApp(
            peerIds: peerIds,
            onRequestManifest: (shareId, peerId) async {
              calls.add([shareId, peerId]);
            },
            onReadvertise: (_, _) async {},
          );
        },
      ),
    );
    await tester.pump();

    await selectPeer(tester, 'peer-2');
    rebuild(() => peerIds = ['peer-3']);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('share-network-refresh')));
    await tester.pumpAndSettle();

    expect(calls, [
      ['share-a', 'peer-3'],
    ]);
  });

  testWidgets('network failures show an inline error', (tester) async {
    var completions = 0;
    await tester.pumpWidget(
      networkApp(
        peerIds: const ['peer-1'],
        onRequestManifest: (_, _) async => throw Exception('access denied'),
        onReadvertise: (_, _) async {},
        onCompleted: () async => completions++,
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('share-network-refresh')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-network-error')), findsOneWidget);
    expect(find.textContaining('Manifest refresh failed'), findsOneWidget);
    expect(find.textContaining('access denied'), findsOneWidget);
    expect(completions, 0);
  });
}
