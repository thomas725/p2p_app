import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/network_config.dart';
import 'package:p2p_app_flutter/src/screens/settings.dart';

import '../helpers/fake_node_api.dart';

const _live = LiveStatus(
  connectedCount: 0,
  connectedPeerIds: [],
  listenAddresses: [],
  lastConnectionAt: null,
);

Widget _app() => MaterialApp(
  home: Scaffold(
    body: Settings(
      liveStatus: ValueNotifier(_live),
      status: ValueNotifier<MobilePeerStatus?>(null),
      serviceRunning: false,
      onToggleService: () {},
      onShareApk: () {},
      isAndroid: false,
      peers: const [],
      networkName: 'test',
      onOpenPeerInfo: (_) {},
      userId: null,
      devices: const [],
    ),
  ),
);

NetworkPeer _peer({required int id, required String kind, String? addr}) =>
    NetworkPeer(
      id: id,
      kind: kind,
      multiaddr: addr ?? '/ip4/1.2.3.4/tcp/4001',
      createdAt: '2026-11-08 12:00:00',
    );

void main() {
  late FakeNodeApi fake;

  setUp(() {
    fake = FakeNodeApi();
    nodeApi = fake;
  });

  /// Tall viewport so the whole Settings list (the network-peers card sits
  /// below the fold on the default 800x600 surface) is laid out.
  void useTallSurface(WidgetTester tester) {
    tester.view.physicalSize = const Size(1000, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
  }

  testWidgets('lists persisted network peers and removes one', (tester) async {
    useTallSurface(tester);
    fake.networkPeers = [
      _peer(id: 1, kind: 'bootstrap', addr: '/ip4/1.2.3.4/tcp/4001'),
      _peer(id: 2, kind: 'relay', addr: '/ip4/5.6.7.8/tcp/4001/p2p/abc'),
    ];

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    expect(find.text('Network peers'), findsOneWidget);
    expect(find.text('/ip4/1.2.3.4/tcp/4001'), findsOneWidget);
    expect(find.text('relay'), findsOneWidget);

    await tester.tap(find.byKey(const Key('network-peer-remove-1')));
    await tester.pumpAndSettle();

    expect(fake.networkPeers.map((peer) => peer.id), [2]);
    expect(find.text('/ip4/1.2.3.4/tcp/4001'), findsNothing);
  });

  testWidgets('adds a bootstrap peer through the dialog', (tester) async {
    useTallSurface(tester);
    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('network-peers-empty')), findsOneWidget);

    await tester.tap(find.byKey(const Key('network-peers-add')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('network-peer-multiaddr')),
      '/ip4/9.9.9.9/tcp/4001',
    );
    await tester.tap(find.byKey(const Key('network-peer-save')));
    await tester.pumpAndSettle();

    expect(fake.networkPeers, hasLength(1));
    expect(fake.networkPeers.single.kind, 'bootstrap');
    expect(fake.networkPeers.single.multiaddr, '/ip4/9.9.9.9/tcp/4001');
    expect(find.text('/ip4/9.9.9.9/tcp/4001'), findsOneWidget);
    expect(find.textContaining('applies on the next node start'), findsOneWidget);
  });

  testWidgets('shows Rust validation errors without adding a row', (
    tester,
  ) async {
    useTallSurface(tester);
    fake.addNetworkPeerError = 'relay address needs a /p2p/<peer-id> suffix';

    await tester.pumpWidget(_app());
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('network-peers-add')));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.byKey(const Key('network-peer-multiaddr')),
      '/ip4/1.2.3.4/tcp/4001',
    );
    await tester.tap(find.byKey(const Key('network-peer-save')));
    await tester.pumpAndSettle();

    expect(fake.networkPeers, isEmpty);
    expect(
      find.text('relay address needs a /p2p/<peer-id> suffix'),
      findsOneWidget,
    );
  });
}
