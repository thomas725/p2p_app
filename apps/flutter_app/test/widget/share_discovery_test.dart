// Widget tests for peer-share discovery. The widget owns only the peer picker
// and error state; offers arrive through a notifier and every node call is an
// injected callback, so no FFI or live peer is needed.

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareKind, ShareOffer;
import 'package:p2p_app_flutter/src/screens/share_discovery.dart';

ShareOffer offer(String id, String name) => ShareOffer(
  shareId: id,
  name: name,
  kind: ShareKind.public,
  revision: BigInt.from(2),
  publicWrite: false,
  ownerUserId: null,
);

Widget discoveryApp({
  required List<String> peerIds,
  required ValueNotifier<List<DiscoveredShare>> offers,
  required Future<void> Function(String peerId) onDiscover,
  required Future<void> Function(DiscoveredShare discovered) onAccept,
}) => MaterialApp(
  home: Scaffold(
    body: ShareDiscoverySection(
      peerIds: peerIds,
      offers: offers,
      onDiscover: onDiscover,
      onAccept: onAccept,
    ),
  ),
);

Future<void> selectPeer(WidgetTester tester, String peerId) async {
  await tester.tap(find.byKey(const Key('share-discovery-peer')));
  await tester.pumpAndSettle();
  await tester.tap(find.text(peerId).last);
  await tester.pumpAndSettle();
}

void main() {
  testWidgets('no peers explains why discovery is unavailable', (tester) async {
    final offers = ValueNotifier<List<DiscoveredShare>>(const []);
    addTearDown(offers.dispose);
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const [],
        offers: offers,
        onDiscover: (_) async {},
        onAccept: (_) async {},
      ),
    );
    await tester.pump();

    expect(
      find.text('No connected peers. Discovery needs a peer.'),
      findsOneWidget,
    );
    expect(find.byKey(const Key('share-discovery-peer')), findsNothing);
    expect(find.byKey(const Key('share-discovery-search')), findsNothing);
  });

  testWidgets('search sends the chosen peer id', (tester) async {
    final offers = ValueNotifier<List<DiscoveredShare>>(const []);
    addTearDown(offers.dispose);
    final calls = <String>[];
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const ['peer-1', 'peer-2'],
        offers: offers,
        onDiscover: (peerId) async => calls.add(peerId),
        onAccept: (_) async {},
      ),
    );
    await tester.pump();

    await selectPeer(tester, 'peer-2');
    await tester.tap(find.byKey(const Key('share-discovery-search')));
    await tester.pumpAndSettle();

    expect(calls, ['peer-2']);
  });

  testWidgets('accept sends the exact advertised offer', (tester) async {
    final advertised = offer('share-a', 'Docs');
    final offers = ValueNotifier<List<DiscoveredShare>>([
      (peerId: 'peer-1', offer: advertised),
    ]);
    addTearDown(offers.dispose);
    final calls = <DiscoveredShare>[];
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const ['peer-1'],
        offers: offers,
        onDiscover: (_) async {},
        onAccept: (discovered) async => calls.add(discovered),
      ),
    );
    await tester.pump();

    expect(find.text('Docs'), findsOneWidget);
    expect(find.textContaining('owner unknown (unverified)'), findsOneWidget);
    await tester.tap(find.byKey(const Key('discovered-accept-peer-1-share-a')));
    await tester.pumpAndSettle();

    expect(calls.length, 1);
    expect(calls.single.peerId, 'peer-1');
    expect(calls.single.offer, advertised);
  });

  testWidgets('offers arriving later appear without a rebuild', (tester) async {
    final offers = ValueNotifier<List<DiscoveredShare>>(const []);
    addTearDown(offers.dispose);
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const ['peer-1'],
        offers: offers,
        onDiscover: (_) async {},
        onAccept: (_) async {},
      ),
    );
    await tester.pump();
    expect(
      find.text('No offers received yet. Search a peer to see its shares.'),
      findsOneWidget,
    );

    offers.value = [(peerId: 'peer-1', offer: offer('share-a', 'Photos'))];
    await tester.pumpAndSettle();

    expect(find.text('Photos'), findsOneWidget);
    expect(
      find.byKey(const Key('discovered-share-peer-1-share-a')),
      findsOneWidget,
    );
  });

  test('event payload decodes camelCase and rejects malformed offers', () {
    final payload = jsonEncode({
      'shareId': 'share-a',
      'name': 'Docs',
      'kind': 'public',
      'revision': 3,
      'publicWrite': false,
      'ownerUserId': null,
    });

    final parsed = discoveredShareFromEvent('peer-1', payload);
    expect(parsed, isNotNull);
    expect(parsed!.peerId, 'peer-1');
    expect(parsed.offer.shareId, 'share-a');
    expect(parsed.offer.name, 'Docs');
    expect(parsed.offer.kind, ShareKind.public);
    expect(parsed.offer.revision, BigInt.from(3));

    expect(discoveredShareFromEvent(null, payload), isNull);
    expect(discoveredShareFromEvent('peer-1', 'not json'), isNull);
    expect(
      discoveredShareFromEvent('peer-1', jsonEncode({'shareId': 'x'})),
      isNull,
    );
  });

  testWidgets('merge deduplicates retransmits but keeps peers separate', (
    tester,
  ) async {
    final first = offer('share-a', 'Docs');
    final renamed = offer('share-a', 'Renamed');
    final merged = mergeDiscoveredShares(
      [(peerId: 'peer-1', offer: first), (peerId: 'peer-2', offer: first)],
      'peer-1',
      renamed,
    );

    expect(merged, hasLength(2));
    expect(merged.first.offer.name, 'Renamed');
    expect(merged.last.peerId, 'peer-2');
    expect(merged.last.offer.name, 'Docs');
  });

  testWidgets('discovery failure shows an inline error', (tester) async {
    final offers = ValueNotifier<List<DiscoveredShare>>(const []);
    addTearDown(offers.dispose);
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const ['peer-1'],
        offers: offers,
        onDiscover: (_) async => throw Exception('node not running'),
        onAccept: (_) async {},
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('share-discovery-search')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-discovery-error')), findsOneWidget);
    expect(find.textContaining('Discovery failed'), findsOneWidget);
    expect(find.textContaining('node not running'), findsOneWidget);
  });

  testWidgets('accept failure shows an inline error and offers stay', (
    tester,
  ) async {
    final offers = ValueNotifier<List<DiscoveredShare>>([
      (peerId: 'peer-1', offer: offer('share-a', 'Docs')),
    ]);
    addTearDown(offers.dispose);
    await tester.pumpWidget(
      discoveryApp(
        peerIds: const ['peer-1'],
        offers: offers,
        onDiscover: (_) async {},
        onAccept: (_) async => throw Exception('invalid share'),
      ),
    );
    await tester.pump();

    await tester.tap(find.byKey(const Key('discovered-accept-peer-1-share-a')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('share-discovery-error')), findsOneWidget);
    expect(find.textContaining('Add share failed'), findsOneWidget);
    expect(find.text('Docs'), findsOneWidget);
  });
}
