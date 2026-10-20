// Widget tests for the peer-info page's call, local-nickname and message-stats
// affordances. The screen never touches the node itself: calling goes through
// `onCall`, nickname edits through `onEditLocalNickname` and the stats come
// from the supplied contact row, so all can be exercised without a loaded Rust
// library.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';

MobilePeerRecord peer({
  String id = 'peer-abc',
  String? localNickname,
  String? nickname,
}) => MobilePeerRecord(
  peerId: id,
  firstSeen: '',
  lastSeen: '',
  displayName: localNickname ?? nickname ?? 'Alice',
  localNickname: localNickname,
  nickname: nickname,
);

/// The contact row that owns the peer's device, carrying the aggregate stats
/// the Contacts tab shows (and the Peer Info page mirrors).
MobileContactRow contact({int dm = 3, int broadcast = 7}) => MobileContactRow(
  contactId: 'peer-abc',
  isUser: false,
  displayName: 'Alice',
  deviceCount: BigInt.one,
  deviceRows: [
    MobileContactDevice(
      peerId: 'peer-abc',
      displayName: 'Alice',
      firstSeen: '',
      lastSeen: '',
    ),
  ],
  firstSeen: '',
  lastSeen: '',
  dmCount: dm,
  broadcastSentToPeer: broadcast,
);

void main() {
  testWidgets('call button renders and invokes onCall with the peer',
      (tester) async {
    MobilePeerRecord? called;
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          onCall: (p) => called = p,
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('call-peer-info')));
    expect(called?.peerId, 'peer-abc');
  });

  testWidgets('no call button renders without onCall', (tester) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    expect(find.byKey(const Key('call-peer-info')), findsNothing);
  });

  testWidgets('no edit button renders without onEditLocalNickname', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    expect(find.byKey(const Key('edit-nickname-peer-info')), findsNothing);
  });

  testWidgets('editing the local nickname persists and updates the page', (
    tester,
  ) async {
    final saved = <(String, String)>[];
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          onEditLocalNickname: (p, name) async {
            saved.add((p.peerId, name));
            return null;
          },
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('edit-nickname-peer-info')));
    await tester.pumpAndSettle();
    expect(find.text('Set local nickname'), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'bob');
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();

    expect(saved.single, ('peer-abc', 'bob'));
    // The page reflects the edit without being reopened: title + display name.
    expect(find.text('Local nickname set to "bob".'), findsOneWidget);
    expect(find.text('bob'), findsWidgets);
    // Origin card title + the nickname info row both now say "Local nickname".
    expect(find.text('Local nickname'), findsNWidgets(2));
  });

  testWidgets('a rejected nickname keeps the dialog open with an error', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          onEditLocalNickname: (_, _) async => throw Exception('bad!'),
        ),
      ),
    );

    await tester.tap(find.byKey(const Key('edit-nickname-peer-info')));
    await tester.pumpAndSettle();
    // Prefilled with the current (empty) local nickname.
    await tester.enterText(find.byType(TextField), 'bad!');
    await tester.tap(find.widgetWithText(FilledButton, 'Save'));
    await tester.pumpAndSettle();

    expect(find.text('Invalid nickname: Exception: bad!'), findsOneWidget);
    expect(find.text('Set local nickname'), findsOneWidget);
    // Nothing was applied to the page title.
    expect(find.text('Alice'), findsWidgets);
  });

  testWidgets('renders the contact stats rows when a contact is supplied', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          contact: contact(),
        ),
      ),
    );
    expect(find.text('Direct messages'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('Broadcasts sent'), findsOneWidget);
    expect(find.text('7'), findsOneWidget);
  });

  testWidgets('renders no stats rows without a contact', (tester) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    expect(find.text('Direct messages'), findsNothing);
    expect(find.text('Broadcasts sent'), findsNothing);
  });
}