// Widget tests for the peer-info page's call, local-nickname and message-stats
// affordances. The screen never touches the node itself: calling goes through
// `onCall`, nickname edits through `onEditLocalNickname` and the stats come
// from the supplied contact row, so all can be exercised without a loaded Rust
// library.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';

MobilePeerRecord peer({
  String id = 'peer-abc',
  String? localNickname,
  String? nickname,
  String firstSeen = '',
  String lastSeen = '',
}) => MobilePeerRecord(
  peerId: id,
  firstSeen: firstSeen,
  lastSeen: lastSeen,
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
  testWidgets('call button renders and invokes onCall with the peer', (
    tester,
  ) async {
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

  testWidgets('link-device button renders and invokes onLinkDevice', (
    tester,
  ) async {
    MobilePeerRecord? linked;
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          onLinkDevice: (p) => linked = p,
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('link-device-peer-info')));
    expect(linked?.peerId, 'peer-abc');
  });

  testWidgets('no link-device button renders without onLinkDevice', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    expect(find.byKey(const Key('link-device-peer-info')), findsNothing);
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

  testWidgets('seen timestamps render verbatim from Rust', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(
            firstSeen: '2026-11-20 10:00:00',
            lastSeen: '2026-11-21 09:00:00',
          ),
          serviceRunning: true,
        ),
      ),
    );
    expect(find.text('First seen'), findsOneWidget);
    expect(find.text('2026-11-20 10:00:00'), findsOneWidget);
    expect(find.text('Last seen'), findsOneWidget);
    expect(find.text('2026-11-21 09:00:00'), findsOneWidget);
  });

  testWidgets('unrecognized seen strings are passed through untouched', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(
            firstSeen: 'raw-first',
            lastSeen: '2026-11-21T09:00:00.123Z',
          ),
          serviceRunning: true,
        ),
      ),
    );
    expect(find.text('raw-first'), findsOneWidget);
    expect(find.text('2026-11-21T09:00:00.123Z'), findsOneWidget);
  });

  testWidgets('renders the connection-quality card with Rust labels', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          loadQuality: (peerId) async => PeerQuality(
            peerId: peerId,
            rttMs: 12,
            pingSuccesses: 12,
            pingFailures: 0,
            pingLoss: 0,
            rttLabel: '12 ms',
            lossLabel: '0% (0/12)',
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('Connection quality'), findsOneWidget);
    expect(find.text('12 ms'), findsOneWidget);
    expect(find.text('0% (0/12)'), findsOneWidget);
  });

  testWidgets('renders no connection-quality card without loadQuality', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    expect(find.text('Connection quality'), findsNothing);
  });

  testWidgets('renders no block affordance without block callbacks', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(home: PeerInfoScreen(peer: peer(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('block-peer-info')), findsNothing);
    expect(find.text('Blocked'), findsNothing);
  });

  testWidgets('shows the blocked state and toggles it', (tester) async {
    final calls = <(String, String)>[];
    var blocked = false;
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          loadBlocked: (peerId) async {
            calls.add(('load', peerId));
            return blocked;
          },
          toggleBlock: (peerId) async {
            calls.add(('toggle', peerId));
            blocked = !blocked;
            return blocked;
          },
        ),
      ),
    );
    await tester.pumpAndSettle();

    expect(calls.single, ('load', 'peer-abc'));
    expect(find.text('Blocked'), findsOneWidget);
    expect(find.text('no'), findsOneWidget);
    expect(find.byTooltip('Block peer'), findsOneWidget);

    await tester.tap(find.byKey(const Key('block-peer-info')));
    await tester.pumpAndSettle();

    expect(calls.last, ('toggle', 'peer-abc'));
    expect(find.text('yes'), findsOneWidget);
    expect(find.byTooltip('Unblock peer'), findsOneWidget);
    expect(
      find.text('Peer blocked: its traffic is now dropped.'),
      findsOneWidget,
    );

    await tester.tap(find.byKey(const Key('block-peer-info')));
    await tester.pumpAndSettle();
    expect(find.text('no'), findsOneWidget);
    expect(find.byTooltip('Block peer'), findsOneWidget);
  });

  testWidgets('a failed block keeps the state and shows the error', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: PeerInfoScreen(
          peer: peer(),
          serviceRunning: true,
          loadBlocked: (_) async => false,
          toggleBlock: (_) async => throw Exception('bad id'),
        ),
      ),
    );
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('block-peer-info')));
    await tester.pumpAndSettle();

    expect(find.text('no'), findsOneWidget);
    expect(find.text('Blocklist error: Exception: bad id'), findsOneWidget);
  });
}
