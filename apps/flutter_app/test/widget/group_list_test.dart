// Widget tests for the Groups tab `GroupList`.
//
// The widget is pure UI: it never calls the Rust FFI directly — group
// creation goes through the injected `onCreateGroup` callback and opening a
// chat through `onOpenGroup`, so the create/join dialog and list rendering can
// be exercised without a loaded Rust library.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/group_list.dart';

MobileGroup group(String id, {String? name, int members = 0}) => MobileGroup(
  groupId: id,
  displayName: name ?? id,
  memberCount: members,
  isPrivate: false,
);

Widget _app({
  required List<MobileGroup> groups,
  required Future<void> Function(String) onCreateGroup,
  void Function(MobileGroup)? onOpenGroup,
}) => MaterialApp(
  home: Scaffold(
    body: GroupList(
      groups: groups,
      onOpenGroup: onOpenGroup ?? (_) {},
      onCreateGroup: onCreateGroup,
    ),
  ),
);

void main() {
  testWidgets('empty state prompts to create or join a group', (tester) async {
    await tester.pumpWidget(_app(groups: const [], onCreateGroup: (_) async {}));
    await tester.pump();

    expect(find.text('Groups (0)'), findsOneWidget);
    expect(find.text('No groups yet. Create or join one!'), findsOneWidget);
  });

  testWidgets('renders groups with member counts and opens chat on tap', (
    tester,
  ) async {
    final opened = <MobileGroup>[];
    await tester.pumpWidget(
      _app(
        groups: [
          group('aaa', name: 'Rust Enthusiasts', members: 1),
          group('bbb', name: 'Flutter Devs', members: 3),
        ],
        onCreateGroup: (_) async {},
        onOpenGroup: opened.add,
      ),
    );
    await tester.pump();

    expect(find.text('Groups (2)'), findsOneWidget);
    expect(find.text('Rust Enthusiasts'), findsOneWidget);
    expect(find.text('Flutter Devs'), findsOneWidget);
    expect(find.text('1 member'), findsOneWidget);
    expect(find.text('3 members'), findsOneWidget);

    await tester.tap(find.text('Flutter Devs'));
    await tester.pump();
    expect(opened.single.groupId, 'bbb');
  });

  testWidgets('create dialog trims the name and submits to onCreateGroup', (
    tester,
  ) async {
    final created = <String>[];
    await tester.pumpWidget(
      _app(groups: const [], onCreateGroup: (name) async => created.add(name)),
    );
    await tester.pump();

    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), '  rust-lang  ');
    await tester.tap(find.text('Create'));
    await tester.pumpAndSettle();

    expect(created, ['rust-lang']);
    // Dialog closed and a confirmation snackbar is shown.
    expect(find.text('Create or join a group'), findsNothing);
    expect(find.text('Group "rust-lang" ready.'), findsOneWidget);
  });

  testWidgets('create dialog surfaces errors and stays open on failure', (
    tester,
  ) async {
    await tester.pumpWidget(
      _app(groups: const [], onCreateGroup: (_) async => throw Exception('boom')),
    );
    await tester.pump();

    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), 'rust');
    await tester.tap(find.text('Create'));
    await tester.pumpAndSettle();

    // The error snackbar is shown and the dialog remains for a retry.
    expect(find.text('Failed to create group: Exception: boom'), findsOneWidget);
    expect(find.text('Create or join a group'), findsOneWidget);
  });

  testWidgets('create dialog ignores empty names', (tester) async {
    var calls = 0;
    await tester.pumpWidget(
      _app(groups: const [], onCreateGroup: (name) async => calls++),
    );
    await tester.pump();

    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();

    await tester.enterText(find.byType(TextField), '   ');
    await tester.tap(find.text('Create'));
    await tester.pump();

    expect(calls, 0);
    expect(find.text('Create or join a group'), findsOneWidget);
  });
}