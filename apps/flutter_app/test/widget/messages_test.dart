// Widget tests for the shared message UI: the delivery-receipt tick (a bubble
// shows `Icons.done_all` only for our own acknowledged messages, and both the
// broadcast tab and an open DM screen flip the flag on a live `receipt` event)
// and the per-message round-trip latency the TUI already renders inline.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_chat.dart';
import 'package:p2p_app_flutter/src/screens/home.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

import '../helpers/fake_node_api.dart';

ChatMessage chatMessage({
  int id = 1,
  String content = 'hello',
  String? peerId,
  String? msgId = 'm1',
  bool sent = true,
  bool confirmed = false,
  bool encrypted = false,
  String? senderNickname,
  String? latency,
}) => ChatMessage(
  id: id,
  content: content,
  peerId: peerId,
  isBroadcast: peerId == null,
  targetPeer: null,
  sent: sent,
  msgId: msgId,
  sentAt: '10:00',
  createdAt: '2026-01-01 10:00:00',
  senderNickname: senderNickname,
  attachment: null,
  encrypted: encrypted,
  confirmed: confirmed,
  latency: latency,
);

MobilePeerRecord peerRecord() => MobilePeerRecord(
  peerId: 'peer-abc',
  firstSeen: '',
  lastSeen: '',
  displayName: 'Alice',
  localNickname: null,
  nickname: null,
);

MobileGroup groupRecord() => MobileGroup(
  groupId: 'group-1',
  displayName: 'Test group',
  memberCount: 2,
  isPrivate: false,
);

/// Enter selection mode through the same mixin method the bubble's double-tap
/// calls. Driving the recognizer with synthetic pointer events is brittle in
/// widget tests, and the action under test is the delete flow, not the gesture.
void _selectMessage(WidgetTester tester, Type screenType, int index) {
  final state = tester.state(find.byType(screenType));
  (state as dynamic).onBubbleDoubleTap(index);
}

void main() {
  // The event bus is a global; leaving a sink installed would leak into the
  // next test (and into the pump of the next screen).
  tearDown(() => setEventSink(null));

  testWidgets('own confirmed bubbles show a delivery tick', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(message: chatMessage(confirmed: true), isOwn: true),
        ),
      ),
    );
    expect(find.byIcon(Icons.done_all), findsOneWidget);
  });

  testWidgets('unconfirmed and incoming bubbles show no tick', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              MessageBubble(message: chatMessage(confirmed: false), isOwn: true),
              // Even a *confirmed* incoming message never shows our tick.
              MessageBubble(
                message: chatMessage(id: 2, peerId: 'peer-abc', confirmed: true),
                isOwn: false,
              ),
            ],
          ),
        ),
      ),
    );
    expect(find.byIcon(Icons.done_all), findsNothing);
  });

  testWidgets('a receipt event ticks the acknowledged DM', (tester) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [chatMessage(msgId: 'dm-1')];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    // Resolve the async history load.
    await tester.pump();
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(eventType: 'receipt', peerId: 'peer-abc', msgId: 'dm-1'),
    );
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsOneWidget);
  });

  testWidgets('a receipt for an unknown id leaves the DM untouched', (
    tester,
  ) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [chatMessage(msgId: 'dm-1')];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pump();
    await tester.pump();

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'receipt',
        peerId: 'peer-abc',
        msgId: 'someone-elses-message',
      ),
    );
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsNothing);
  });

  testWidgets('a receipt event ticks the acknowledged broadcast', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..loadBroadcastMessagesOverride = (_) => [chatMessage(msgId: 'bc-1')];
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    // _init resolves status, history, roster and groups asynchronously.
    await tester.pump();
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(eventType: 'receipt', peerId: 'peer-abc', msgId: 'bc-1'),
    );
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsOneWidget);
  });

  testWidgets('an incoming bubble renders its live latency', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              MessageBubble(
                message: chatMessage(
                  id: 1,
                  peerId: 'peer-abc',
                  latency: '5ms',
                  senderNickname: 'Alice',
                ),
                isOwn: false,
              ),
              // A message with no measured latency shows none.
              MessageBubble(
                message: chatMessage(id: 2, peerId: 'peer-abc'),
                isOwn: false,
              ),
            ],
          ),
        ),
      ),
    );
    expect(find.text('5ms'), findsOneWidget);
  });

  testWidgets('a live broadcast carries its latency into the chat', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    final fake = FakeNodeApi(isRemoteTransport: true);
    fake.mobilePeerStatus = const MobilePeerStatus(
      databaseUrl: ':memory:',
      localPeerId: 'me',
    );
    // (Assignments are separate statements: a `=> []` cascade target would
    // swallow the following `..setter` into the list literal's own cascade.)
    fake.loadBroadcastMessagesOverride = (_) => [];
    fake.saveIncomingMessageOverride =
        (content, peerId, isDirect, nickname, latency) => chatMessage(
          id: 7,
          content: content,
          peerId: peerId,
          msgId: null,
          senderNickname: nickname,
          latency: latency,
        );
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();
    expect(find.text('12ms'), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'broadcast',
        peerId: 'peer-abc',
        content: 'hi all',
        latency: '12ms',
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(find.text('12ms'), findsOneWidget);
  });

  testWidgets('selection delete hides an own DM locally only', (tester) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(id: 7, content: 'mine', msgId: 'dm-7'),
        chatMessage(
          id: 8,
          content: 'theirs',
          peerId: 'peer-abc',
          sent: false,
          msgId: 'dm-8',
        ),
      ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();

    _selectMessage(tester, DmChatScreen, 0);
    await tester.pumpAndSettle();
    expect(find.byIcon(Icons.delete_outline), findsOneWidget);
    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();

    expect(find.textContaining('hides the message only on this device'), findsOneWidget);
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();

    final calls = fake.calls.where((c) => c.method == 'deleteMessage').toList();
    expect(calls, hasLength(1));
    expect(calls.single.argument, (messageId: 7, isGroup: false));
    expect(find.text('mine'), findsNothing);
    expect(find.text('theirs'), findsOneWidget);
  });

  testWidgets('cancelling local delete keeps the message', (tester) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(id: 9, content: 'keep me', msgId: 'dm-9'),
      ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    _selectMessage(tester, DmChatScreen, 0);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();

    expect(fake.calls.where((c) => c.method == 'deleteMessage'), isEmpty);
    expect(find.text('keep me'), findsOneWidget);
  });

  testWidgets('group delete marks the tombstone as a group row', (tester) async {
    final fake = FakeNodeApi()
      ..loadGroupMessagesOverride = (_, _) => [
        MobileGroupMessage(
          id: 11,
          content: 'group mine',
          sent: true,
          createdAt: '2026-01-01 10:00:00',
        ),
      ];
    nodeApi = fake;

    await tester.pumpWidget(
      MaterialApp(
        home: GroupChatScreen(group: groupRecord(), serviceRunning: true),
      ),
    );
    await tester.pumpAndSettle();
    _selectMessage(tester, GroupChatScreen, 0);
    await tester.pumpAndSettle();
    await tester.tap(find.byIcon(Icons.delete_outline));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Delete'));
    await tester.pumpAndSettle();

    final calls = fake.calls.where((c) => c.method == 'deleteMessage').toList();
    expect(calls, hasLength(1));
    expect(calls.single.argument, (messageId: 11, isGroup: true));
    expect(find.text('group mine'), findsNothing);
  });
}
