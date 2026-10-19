// Widget tests for the peer-info page's call and local-nickname affordances.
// The screen never touches the node itself: calling goes through `onCall` and
// nickname edits through `onEditLocalNickname`, so both can be exercised
// without a loaded Rust library.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

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
}