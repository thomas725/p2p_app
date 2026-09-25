import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
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
    ),
  );
}

final _device = MobileDeviceInfo(
  claimId: 'c1',
  userId: 'user-abc',
  peerId: '12D3KooWpeer-1',
  devicePubkey: Uint8List.fromList([1, 2, 3]),
  boundAt: '2026-09-25 10:00:00',
);

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