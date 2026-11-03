import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/screens/settings.dart';

const _live = LiveStatus(
  connectedCount: 0,
  connectedPeerIds: [],
  listenAddresses: [],
  lastConnectionAt: null,
);

Widget _app({
  required String? userId,
  required List<MobileDeviceInfo> devices,
  List<String> peerInfoOpened = const [],
  ValueNotifier<NetworkDetails?>? networkDetails,
}) {
  return MaterialApp(
    home: Settings(
      liveStatus: ValueNotifier(_live),
      status: ValueNotifier<MobilePeerStatus?>(null),
      serviceRunning: false,
      onToggleService: () {},
      onShareApk: () {},
      isAndroid: false,
      peers: const [],
      networkName: 'test',
      onOpenPeerInfo: (id) => peerInfoOpened.add(id),
      userId: userId,
      devices: devices,
      networkDetails: networkDetails,
    ),
  );
}

final _network = NetworkDetails(
  interfaces: const [
    NetworkInterfaceInfo(
      name: 'eth0',
      mac: 'aa:bb:cc:dd:ee:ff',
      ipv4: ['192.168.1.2'],
      ipv6: ['fe80::1'],
      internal: false,
    ),
    NetworkInterfaceInfo(
      name: 'lo',
      mac: null,
      ipv4: ['127.0.0.1'],
      ipv6: ['::1'],
      internal: true,
    ),
  ],
  listenAddrs: const ['/ip4/192.168.1.2/tcp/4001'],
  externalAddrs: const ['/ip4/203.0.113.7/tcp/4001'],
  externalCandidates: const [],
  reachability: 'Public (directly reachable)',
  natStatus: 'Public',
  natPublicAddr: '/ip4/203.0.113.7/tcp/4001',
  natProbeSuccesses: 2,
  natProbeFailures: 1,
  holePunchAttempts: 3,
  holePunchSuccesses: 1,
  relayReservations: const ['12D3KooWRelay'],
  relayStatus: 'Reserved via 12D3KooWRelay',
  dhtMode: 'Server',
  dhtConnected: true,
  dhtRoutingPeers: 7,
  dhtBootstrapSuccesses: 2,
  dhtBootstrapFailures: 0,
  dhtAnnounced: true,
  dhtAnnounceStatus: 'Published to DHT',
  dhtAnnounceAt: 1700000000,
);

final _device = MobileDeviceInfo(
  claimId: 'c1',
  userId: 'user-abc',
  peerId: '12D3KooWpeer-1',
  devicePubkey: Uint8List.fromList([1, 2, 3]),
  boundAt: '2026-09-25 10:00:00',
);

/// Give the test a tall viewport so the whole Settings list (including the
/// network-details card near the bottom) is laid out without scrolling.
void _useTallSurface(WidgetTester tester) {
  tester.view.physicalSize = const Size(1000, 4000);
  tester.view.devicePixelRatio = 1.0;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
}

void main() {
  testWidgets('Settings shows User ID when provisioned', (tester) async {
    await tester.pumpWidget(_app(userId: 'user-abc', devices: const []));
    await tester.pump();
    expect(find.text('User ID'), findsOneWidget);
    expect(find.text('user-abc'), findsOneWidget);
    expect(find.textContaining('Bound devices'), findsNothing);
  });

  testWidgets('Settings shows Not provisioned before provisioning', (
    tester,
  ) async {
    await tester.pumpWidget(_app(userId: null, devices: const []));
    await tester.pump();
    expect(find.text('Not provisioned'), findsOneWidget);
  });

  testWidgets('Settings renders live network details', (tester) async {
    _useTallSurface(tester);
    await tester.pumpWidget(_app(
      userId: 'user-abc',
      devices: const [],
      networkDetails: ValueNotifier(_network),
    ));
    await tester.pump();

    expect(find.text('Network details'), findsOneWidget);
    expect(find.text('Public (directly reachable)'), findsOneWidget);
    expect(
      find.text('Public — probes 2 ok / 1 failed'),
      findsOneWidget,
    );
    expect(find.text('1/3 succeeded'), findsOneWidget);
    expect(
      find.text('Reserved via 12D3KooWRelay (12D3KooWRelay)'),
      findsOneWidget,
    );
    expect(
      find.text(
        'connected (Server, 7 routing peers; bootstrap 2 ok / 0 failed)',
      ),
      findsOneWidget,
    );
    expect(find.text('Announced (Published to DHT)'), findsOneWidget);
    expect(find.text('/ip4/203.0.113.7/tcp/4001'), findsOneWidget);

    // Interface rows carry the MAC and both address families.
    expect(find.text('eth0 — aa:bb:cc:dd:ee:ff'), findsOneWidget);
    expect(find.text('IPv4: 192.168.1.2'), findsOneWidget);
    expect(find.text('IPv6: fe80::1'), findsOneWidget);
    expect(find.text('lo — no MAC (loopback)'), findsOneWidget);
  });

  testWidgets('Network details repaint when the notifier updates', (
    tester,
  ) async {
    _useTallSurface(tester);
    final notifier = ValueNotifier<NetworkDetails?>(null);
    await tester.pumpWidget(_app(
      userId: 'user-abc',
      devices: const [],
      networkDetails: notifier,
    ));
    await tester.pump();
    expect(find.text('Network details'), findsNothing);

    notifier.value = _network;
    await tester.pump();
    expect(find.text('Network details'), findsOneWidget);
    expect(find.text('Public (directly reachable)'), findsOneWidget);
  });

  testWidgets('Bound devices listed and open PeerInfo on tap', (tester) async {
    final opened = <String>[];
    await tester.pumpWidget(_app(
      userId: 'user-abc',
      devices: [_device],
      peerInfoOpened: opened,
    ));
    await tester.pump();
    expect(find.text('Bound devices (1)'), findsOneWidget);
    expect(find.textContaining('12D3KooWpeer-1'), findsOneWidget);
    expect(find.textContaining('bound 2026-09-25 10:00:00'), findsOneWidget);

    await tester.tap(find.textContaining('12D3KooWpeer-1'));
    expect(opened, ['12D3KooWpeer-1']);
  });
}