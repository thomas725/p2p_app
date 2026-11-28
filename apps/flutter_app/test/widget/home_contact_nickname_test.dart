// The Contacts dialog only collects text; Home owns the NodeApi write and the
// roster refresh that makes Rust's resolved display name visible. This test
// drives that wiring with the real HomeScreen and FakeNodeApi.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/screens/home.dart';

import '../helpers/fake_node_api.dart';

MobileContactRow _userContact({String name = 'Old Name'}) => MobileContactRow(
  contactId: 'user-1',
  isUser: true,
  displayName: name,
  deviceCount: BigInt.from(2),
  deviceRows: [
    const MobileContactDevice(
      peerId: 'user-laptop',
      displayName: 'Laptop',
      firstSeen: '',
      lastSeen: '',
    ),
    const MobileContactDevice(
      peerId: 'user-phone',
      displayName: 'Phone',
      firstSeen: '',
      lastSeen: '',
    ),
  ],
  firstSeen: '',
  lastSeen: '',
  dmCount: 0,
  broadcastSentToPeer: 0,
);

void main() {
  testWidgets('a contact nickname save writes through NodeApi and refreshes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1000, 4000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    final previousApi = nodeApi;
    addTearDown(() => nodeApi = previousApi);

    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..getPeersWithStatsOverride = () async => const <PeerWithStats>[];
    fake.getContactsWithStatsOverride = () async {
      // Stand in for Rust re-resolving the roster from SQLite after the write.
      final saved = fake.contactNicknames.isEmpty
          ? null
          : fake.contactNicknames.last.$2;
      return [_userContact(name: saved ?? 'Old Name')];
    };
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();

    // Drop startup traffic so the assertions below only see the write path.
    fake.calls.clear();

    await tester.tap(find.byIcon(Icons.people));
    await tester.pumpAndSettle();
    await tester.ensureVisible(
      find.byKey(const Key('edit-contact-nickname-user-1')),
    );
    await tester.tap(find.byKey(const Key('edit-contact-nickname-user-1')));
    await tester.pumpAndSettle();

    await tester.enterText(
      find.byKey(const Key('contact-nickname-field')),
      'New-Nick',
    );
    await tester.tap(find.byKey(const Key('contact-nickname-save')));
    await tester.pumpAndSettle();

    expect(fake.contactNicknames, [('user-1', 'New-Nick')]);
    // The contact row carries Rust's ` ×N` device-count suffix as well.
    expect(find.text('New-Nick ×2'), findsOneWidget);
    expect(find.text('Old Name ×2'), findsNothing);

    final methods = fake.calls.map((call) => call.method).toList();
    final writeAt = methods.indexOf('setContactNickname');
    expect(writeAt, isNonNegative);
    expect(methods.indexOf('getPeersWithStats', writeAt + 1), isNonNegative);
    expect(methods.indexOf('getContactsWithStats', writeAt + 1), isNonNegative);
    expect(methods.indexOf('listBlockedPeers', writeAt + 1), isNonNegative);
    expect(find.text('Contact nickname updated.'), findsOneWidget);
  });
}
