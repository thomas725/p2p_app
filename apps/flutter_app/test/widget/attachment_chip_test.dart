import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';

void main() {
  final attachment = MobileAttachment(
    fileId: 'abc123',
    name: 'notes.txt',
    size: 2048,
    mime: 'text/plain',
  );

  Widget harness({bool selectionMode = false, VoidCallback? onOpen}) =>
      MaterialApp(
        home: Scaffold(
          body: ListView(
            children: [
              MessageBubble(
                message: ChatMessage(
                  id: 1,
                  content: '',
                  peerId: 'peer-1',
                  isBroadcast: false,
                  sent: true,
                  msgId: 'msg-1',
                  sentAt: '2026-10-04 12:00:00',
                  createdAt: '2026-10-04 12:00:00',
                  senderNickname: 'Alice',
                  attachment: attachment,
                  confirmed: false,
                ),
                isOwn: false,
                selectionMode: selectionMode,
                onOpenAttachment: onOpen,
              ),
            ],
          ),
        ),
      );

  testWidgets('attachment-only bubble renders the chip, not a blank row',
      (tester) async {
    await tester.pumpWidget(harness());
    expect(find.text('[file] notes.txt (2048)'), findsOneWidget);
    expect(find.text(''), findsNothing);
  });

  testWidgets('tapping the chip invokes onOpenAttachment', (tester) async {
    var opened = 0;
    await tester.pumpWidget(harness(onOpen: () => opened++));
    await tester.tap(find.text('[file] notes.txt (2048)'));
    expect(opened, 1);
  });

  testWidgets('chip is not tappable in selection mode', (tester) async {
    var opened = 0;
    await tester.pumpWidget(
      harness(selectionMode: true, onOpen: () => opened++),
    );
    await tester.tap(find.text('[file] notes.txt (2048)'));
    expect(opened, 0);
  });
}