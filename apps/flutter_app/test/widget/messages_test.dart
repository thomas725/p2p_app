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
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/group_chat.dart';
import 'package:p2p_app_flutter/src/screens/home.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';
import 'package:p2p_app_flutter/src/util/reply.dart';

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
  ReplyRef? replyTo,
  MobileAttachment? attachment,
}) => ChatMessage(
  id: id,
  content: content,
  peerId: peerId,
  isBroadcast: peerId == null,
  targetPeer: null,
  sent: sent,
  msgId: msgId,
  replyTo: replyTo,
  sentAt: '10:00',
  createdAt: '2026-01-01 10:00:00',
  senderNickname: senderNickname,
  attachment: attachment,
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
          body: MessageBubble(
            message: chatMessage(confirmed: true),
            isOwn: true,
          ),
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
              MessageBubble(
                message: chatMessage(confirmed: false),
                isOwn: true,
              ),
              // Even a *confirmed* incoming message never shows our tick.
              MessageBubble(
                message: chatMessage(
                  id: 2,
                  peerId: 'peer-abc',
                  confirmed: true,
                ),
                isOwn: false,
              ),
            ],
          ),
        ),
      ),
    );
    expect(find.byIcon(Icons.done_all), findsNothing);
  });

  testWidgets('only sealed DMs show the end-to-end lock marker', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              MessageBubble(
                message: chatMessage(encrypted: true),
                isOwn: false,
              ),
              MessageBubble(
                message: chatMessage(id: 2, encrypted: false),
                isOwn: false,
              ),
            ],
          ),
        ),
      ),
    );
    expect(find.byIcon(Icons.lock), findsOneWidget);
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
      const SwarmEventJson(
        eventType: 'receipt',
        peerId: 'peer-abc',
        encrypted: false,
        msgId: 'dm-1',
      ),
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
        encrypted: false,
        msgId: 'someone-elses-message',
      ),
    );
    await tester.pump();
    expect(find.byIcon(Icons.done_all), findsNothing);
  });

  test('a receipt match skips inbound rows echoing an outgoing wire id', () {
    final messages = [
      chatMessage(
        id: 1,
        peerId: 'peer-abc',
        msgId: 'dm-1',
        sent: false,
        senderNickname: 'Alice',
      ),
      chatMessage(id: 2, msgId: 'dm-1'),
    ];
    expect(confirmedMessageIndex(messages, 'dm-1'), 1);
    expect(confirmedMessageIndex(messages, 'missing'), -1);
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
      const SwarmEventJson(
        eventType: 'receipt',
        peerId: 'peer-abc',
        encrypted: false,
        msgId: 'bc-1',
      ),
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
        (
          content,
          peerId,
          isDirect,
          nickname,
          latency,
          msgId,
          replyTo,
          encrypted,
        ) => chatMessage(
          id: 7,
          content: content,
          peerId: peerId,
          msgId: null,
          senderNickname: nickname,
          latency: latency,
          encrypted: encrypted,
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
        encrypted: false,
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(find.text('12ms'), findsOneWidget);
  });

  test('an attachment-only reply quotes the canonical Rust chip', () {
    const attachment = MobileAttachment(
      fileId: 'abc123',
      name: 'notes.txt',
      size: 2048,
      sizeLabel: '2 KiB',
      chip: '[file] notes.txt (2 KiB)',
    );
    final fileOnly = replyDraftFor(
      chatMessage(
        id: 1,
        content: '',
        peerId: 'peer-abc',
        msgId: 'file-1',
        sent: false,
        senderNickname: 'Alice',
        attachment: attachment,
      ),
    );
    expect(
      fileOnly,
      const ReplyRef(
        msgId: 'file-1',
        sender: 'Alice',
        preview: '[file] notes.txt (2 KiB)',
      ),
    );
    final mixed = replyDraftFor(
      chatMessage(
        id: 2,
        content: 'see attached',
        peerId: 'peer-abc',
        msgId: 'file-2',
        sent: false,
        senderNickname: 'Alice',
        attachment: attachment,
      ),
    );
    expect(
      mixed?.preview,
      'see attached [file] notes.txt (2 KiB)',
    );
  });

  testWidgets('a reply bubble renders the canonical quoted header', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              MessageBubble(
                message: chatMessage(
                  id: 1,
                  peerId: 'peer-abc',
                  content: 'the answer',
                  senderNickname: 'Alice',
                  replyTo: const ReplyRef(
                    msgId: 'm0',
                    sender: 'Bob',
                    preview: 'the original question',
                  ),
                ),
                isOwn: false,
              ),
              MessageBubble(
                message: chatMessage(id: 2, content: 'no quote'),
                isOwn: true,
              ),
            ],
          ),
        ),
      ),
    );
    expect(find.text('↳ Bob: the original question'), findsOneWidget);
    expect(find.byKey(const Key('quote-header')), findsOneWidget);
  });

  testWidgets('a live broadcast carries its reply snapshot into the chat', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({});
    const quote = ReplyRef(
      msgId: 'quoted-wire-id',
      sender: 'Bob',
      preview: 'an earlier broadcast',
    );
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..loadBroadcastMessagesOverride = (_) => [];
    String? seenMsgId;
    ReplyRef? seenReply;
    fake.saveIncomingMessageOverride =
        (
          content,
          peerId,
          isDirect,
          nickname,
          latency,
          msgId,
          replyTo,
          encrypted,
        ) {
          seenMsgId = msgId;
          seenReply = replyTo;
          return chatMessage(
            id: 7,
            content: content,
            peerId: peerId,
            msgId: msgId,
            replyTo: replyTo,
            latency: latency,
            encrypted: encrypted,
          );
        };
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();

    fake.deliverEvent(
      SwarmEventJson(
        eventType: 'broadcast',
        peerId: 'peer-abc',
        content: 'this answers Bob',
        msgId: 'new-wire-id',
        replyTo: quote,
        encrypted: false,
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(seenMsgId, 'new-wire-id');
    expect(seenReply, quote);
    expect(find.text('↳ Bob: an earlier broadcast'), findsOneWidget);
  });

  testWidgets('the reply action builds a draft and a DM send forwards it', (
    tester,
  ) async {
    const expected = ReplyRef(
      msgId: 'dm-1',
      sender: 'Alice',
      preview: 'hi there',
    );
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(
          id: 1,
          content: 'hi there',
          peerId: 'peer-abc',
          msgId: 'dm-1',
          senderNickname: 'Alice',
          sent: false,
        ),
      ];
    fake.saveOutgoingDmOverride = (peerId, content, replyTo) =>
        chatMessage(id: 2, content: content, msgId: 'dm-2', replyTo: replyTo);
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('reply-action')), findsNothing);

    _selectMessage(tester, DmChatScreen, 0);
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('reply-action')), findsOneWidget);
    await tester.tap(find.byKey(const Key('reply-action')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('reply-draft')), findsOneWidget);
    expect(find.text('↳ Alice: hi there'), findsOneWidget);

    await tester.enterText(find.byType(TextField), 'answering');
    await tester.tap(find.byIcon(Icons.send));
    await tester.pumpAndSettle();

    expect(fake.dmSends, hasLength(1));
    expect(fake.dmSends.single.content, 'answering');
    expect(fake.dmSends.single.replyTo, expected);
    // The quote stays on the sent bubble, but the compose affordance is gone.
    expect(find.byKey(const Key('reply-draft')), findsNothing);
  });

  testWidgets('Reply is hidden for a selected row with an empty wire id', (
    tester,
  ) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(
          id: 1,
          content: 'no stable id',
          peerId: 'peer-abc',
          msgId: '',
          sent: false,
          senderNickname: 'Alice',
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

    expect(find.byKey(const Key('reply-action')), findsNothing);
  });

  testWidgets('cancelling selection keeps the draft; cancel reply drops it', (
    tester,
  ) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(id: 1, msgId: 'dm-1', senderNickname: 'Alice'),
      ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    _selectMessage(tester, DmChatScreen, 0);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('reply-action')));
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('cancel-selection')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('reply-draft')), findsOneWidget);

    await tester.tap(find.byKey(const Key('reply-cancel')));
    await tester.pumpAndSettle();
    expect(find.byKey(const Key('reply-draft')), findsNothing);
    expect(fake.dmSends, isEmpty);
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

    expect(
      find.textContaining('hides the message only on this device'),
      findsOneWidget,
    );
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

  testWidgets('group delete marks the tombstone as a group row', (
    tester,
  ) async {
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

  testWidgets('a sealed live DM keeps its lock flag when persisted', (
    tester,
  ) async {
    bool? persistedEncrypted;
    final fake = FakeNodeApi()..loadDmMessagesOverride = (_, _) => [];
    fake.saveIncomingMessageOverride =
        (
          content,
          peerId,
          isDirect,
          nickname,
          latency,
          msgId,
          replyTo,
          encrypted,
        ) {
          persistedEncrypted = encrypted;
          return chatMessage(
            id: 9,
            content: content,
            peerId: peerId,
            msgId: msgId,
            replyTo: replyTo,
            sent: false,
            encrypted: encrypted,
          );
        };
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'dm',
        peerId: 'peer-abc',
        content: 'sealed hello',
        msgId: 'dm-e2e-1',
        encrypted: true,
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(persistedEncrypted, isTrue);
    expect(find.text('sealed hello'), findsOneWidget);
    expect(find.byIcon(Icons.lock), findsOneWidget);
  });
}
