import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/screens/settings.dart';

import '../helpers/fake_node_api.dart';

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
  LiveStatus liveStatus = _live,
  ValueNotifier<LiveStatus>? liveStatusNotifier,
  List<MobilePeerRecord> peers = const [],
}) {
  return MaterialApp(
    home: Settings(
      liveStatus: liveStatusNotifier ?? ValueNotifier(liveStatus),
      status: ValueNotifier<MobilePeerStatus?>(null),
      serviceRunning: false,
      onToggleService: () {},
      onShareApk: () {},
      isAndroid: false,
      peers: peers,
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

MobileDeviceInfo _device({
  String peerId = '12D3KooWpeer-1',
  String boundAt = '2026-09-25 10:00:00',
}) {
  return MobileDeviceInfo(
    claimId: 'c1',
    userId: 'user-abc',
    peerId: peerId,
    devicePubkey: Uint8List.fromList([1, 2, 3]),
    boundAt: boundAt,
  );
}

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
    await tester.pumpWidget(
      _app(
        userId: 'user-abc',
        devices: const [],
        networkDetails: ValueNotifier(_network),
      ),
    );
    await tester.pump();

    expect(find.text('Network details'), findsOneWidget);
    expect(find.text('Public (directly reachable)'), findsOneWidget);
    expect(find.text('Public — probes 2 ok / 1 failed'), findsOneWidget);
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
    await tester.pumpWidget(
      _app(userId: 'user-abc', devices: const [], networkDetails: notifier),
    );
    await tester.pump();
    expect(find.text('Network details'), findsNothing);

    notifier.value = _network;
    await tester.pump();
    expect(find.text('Network details'), findsOneWidget);
    expect(find.text('Public (directly reachable)'), findsOneWidget);
  });

  testWidgets('Bound devices listed and open PeerInfo on tap', (tester) async {
    final opened = <String>[];
    await tester.pumpWidget(
      _app(userId: 'user-abc', devices: [_device()], peerInfoOpened: opened),
    );
    await tester.pump();
    expect(find.text('Bound devices (1)'), findsOneWidget);
    expect(find.textContaining('12D3KooWpeer-1'), findsOneWidget);
    expect(find.textContaining('bound 2026-09-25 10:00:00'), findsOneWidget);

    await tester.tap(find.textContaining('12D3KooWpeer-1'));
    expect(opened, ['12D3KooWpeer-1']);
  });

  testWidgets('Settings orders bound devices by peer ID', (tester) async {
    await tester.pumpWidget(
      _app(
        userId: 'user-abc',
        devices: [
          _device(peerId: 'peer-z', boundAt: '2026-01-03 00:00:00'),
          _device(peerId: 'peer-a', boundAt: '2026-01-01 00:00:00'),
          _device(peerId: 'peer-m', boundAt: '2026-01-02 00:00:00'),
        ],
      ),
    );
    await tester.pump();

    final rows = tester
        .widgetList<Text>(find.textContaining(' · bound '))
        .map((text) => text.data ?? '')
        .toList();
    expect(rows, [
      'peer-a · bound 2026-01-01 00:00:00',
      'peer-m · bound 2026-01-02 00:00:00',
      'peer-z · bound 2026-01-03 00:00:00',
    ]);
  });

  testWidgets('Settings orders connected peers by peer ID', (tester) async {
    _useTallSurface(tester);
    final opened = <String>[];
    await tester.pumpWidget(
      _app(
        userId: null,
        devices: const [],
        peerInfoOpened: opened,
        liveStatus: const LiveStatus(
          connectedCount: 3,
          connectedPeerIds: ['peer-z', 'peer-a', 'peer-m'],
          listenAddresses: [],
          lastConnectionAt: null,
        ),
        peers: const [
          MobilePeerRecord(
            peerId: 'peer-a',
            firstSeen: '',
            lastSeen: '',
            displayName: 'Zulu peer',
          ),
          MobilePeerRecord(
            peerId: 'peer-m',
            firstSeen: '',
            lastSeen: '',
            displayName: 'Alpha peer',
          ),
          MobilePeerRecord(
            peerId: 'peer-z',
            firstSeen: '',
            lastSeen: '',
            displayName: 'Middle peer',
          ),
        ],
      ),
    );
    await tester.pump();

    final labels = {'Zulu peer', 'Alpha peer', 'Middle peer'};
    final rows = tester
        .widgetList<Text>(
          find.byWidgetPredicate((widget) {
            return widget is Text && labels.contains(widget.data);
          }),
        )
        .map((text) => text.data)
        .toList();
    expect(rows, ['Zulu peer', 'Alpha peer', 'Middle peer']);

    await tester.tap(find.text('Zulu peer'));
    expect(opened, ['peer-a']);
  });

  testWidgets('Last connection lost names the peer from the event', (
    tester,
  ) async {
    _useTallSurface(tester);
    await tester.pumpWidget(
      _app(
        userId: 'user-abc',
        devices: const [],
        liveStatus: const LiveStatus(
          connectedCount: 0,
          connectedPeerIds: [],
          listenAddresses: [],
          lastConnectionAt: null,
          lastConnectionPeerId: '12D3KooWLastPeer',
        ),
        peers: const [
          MobilePeerRecord(
            peerId: '12D3KooWLastPeer',
            firstSeen: '2026-11-20 10:00:00',
            lastSeen: '2026-11-21 09:00:00',
            displayName: 'Brave Otter (er9)',
          ),
        ],
      ),
    );
    await tester.pump();

    expect(find.text('Last connection lost'), findsOneWidget);
    expect(find.text('Never · Brave Otter (er9)'), findsOneWidget);
  });

  testWidgets('Help card opens and closes the controls dialog', (tester) async {
    _useTallSurface(tester);
    await tester.pumpWidget(_app(userId: 'user-abc', devices: const []));
    await tester.pump();

    await tester.tap(find.byKey(const Key('settings-help')));
    await tester.pumpAndSettle();

    expect(find.byType(AlertDialog), findsOneWidget);
    final dialog = find.byType(AlertDialog);
    expect(
      find.descendant(of: dialog, matching: find.text('Navigate')),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: dialog,
        matching: find.textContaining('Double-tap a message'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: dialog,
        matching: find.text(
          'No custom letter or Ctrl shortcuts are installed here.',
        ),
      ),
      findsOneWidget,
    );

    await tester.tap(find.descendant(of: dialog, matching: find.text('Close')));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsNothing);
  });

  testWidgets(
    'Notification toggles persist both channels through the transport',
    (tester) async {
      _useTallSurface(tester);
      final fake = FakeNodeApi();
      nodeApi = fake;
      await tester.pumpWidget(_app(userId: 'user-abc', devices: const []));
      await tester.pumpAndSettle();

      // Defaults from the persisted settings (sound on, desktop off).
      final sound = find.byKey(const Key('notification-sound-toggle'));
      final desktop = find.byKey(const Key('notification-desktop-toggle'));
      expect(sound, findsOneWidget);
      expect(desktop, findsOneWidget);
      expect(tester.widget<SwitchListTile>(sound).value, isTrue);
      expect(tester.widget<SwitchListTile>(desktop).value, isFalse);

      // Toggle the desktop channel on; only that flag changes.
      await tester.tap(desktop);
      await tester.pumpAndSettle();
      expect(fake.notificationSettings.sound, isTrue);
      expect(fake.notificationSettings.desktop, isTrue);

      // Toggle sound off; both writes go through the shared transport.
      await tester.tap(sound);
      await tester.pumpAndSettle();
      expect(fake.notificationSettings.sound, isFalse);
      expect(fake.notificationSettings.desktop, isTrue);

      expect(
        fake.calls.where((c) => c.method == 'setNotificationSettings').length,
        2,
      );
    },
  );

  testWidgets('Network size follows the live connected peer count', (
    tester,
  ) async {
    _useTallSurface(tester);
    final previousApi = nodeApi;
    addTearDown(() => nodeApi = previousApi);
    final fake = FakeNodeApi();
    final requestedCounts = <int>[];
    fake.networkSizeLabelOverride = (peerCount) async {
      requestedCounts.add(peerCount);
      if (peerCount < 4) return 'Small';
      if (peerCount < 16) return 'Medium';
      return 'Large';
    };
    nodeApi = fake;

    final notifier = ValueNotifier(
      const LiveStatus(
        connectedCount: 4,
        connectedPeerIds: [],
        listenAddresses: [],
        lastConnectionAt: null,
      ),
    );
    addTearDown(notifier.dispose);
    await tester.pumpWidget(
      _app(userId: 'user-abc', devices: const [], liveStatusNotifier: notifier),
    );
    await tester.pumpAndSettle();

    expect(find.text('Network size'), findsOneWidget);
    expect(find.text('Medium'), findsOneWidget);
    expect(requestedCounts, [4]);

    notifier.value = const LiveStatus(
      connectedCount: 16,
      connectedPeerIds: [],
      listenAddresses: [],
      lastConnectionAt: null,
    );
    await tester.pumpAndSettle();
    expect(find.text('Large'), findsOneWidget);
    expect(requestedCounts, [4, 16]);

    notifier.value = const LiveStatus(
      connectedCount: 0,
      connectedPeerIds: [],
      listenAddresses: [],
      lastConnectionAt: null,
    );
    await tester.pumpAndSettle();
    expect(find.text('Small'), findsOneWidget);
    expect(requestedCounts, [4, 16, 0]);
  });
}
