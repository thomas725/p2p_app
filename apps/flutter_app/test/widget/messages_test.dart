// Widget tests for the shared message UI: the delivery-receipt tick (a bubble
// shows `Icons.done_all` only for our own acknowledged messages, and both the
// broadcast tab and an open DM screen flip the flag on a live `receipt` event)
// and the per-message round-trip latency the TUI already renders inline.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reactions.dart';
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
  bool edited = false,
  bool encrypted = false,
  String? senderNickname,
  String? latency,
  ReplyRef? replyTo,
  MobileAttachment? attachment,
  List<ReactionSummary> reactions = const [],
}) => ChatMessage(
  id: id,
  content: content,
  peerId: peerId,
  origin: ChatMessageOrigin(isBroadcast: peerId == null, sent: sent),
  targetPeer: null,
  msgId: msgId,
  replyTo: replyTo,
  sentAt: '10:00',
  createdAt: '2026-01-01 10:00:00',
  senderNickname: senderNickname,
  attachment: attachment,
  encrypted: encrypted,
  confirmed: confirmed,
  edited: edited,
  latency: latency,
  reactions: reactions,
);

/// A broadcast authored by a peer (incoming), as `loadBroadcastMessages`
/// returns it: `isBroadcast: true` with the sender in `peerId`.
ChatMessage broadcastMessage({
  required int id,
  required String content,
  required String peerId,
  required String msgId,
  String? senderNickname,
}) => ChatMessage(
  id: id,
  content: content,
  peerId: peerId,
  origin: const ChatMessageOrigin(isBroadcast: true, sent: false),
  targetPeer: null,
  msgId: msgId,
  replyTo: null,
  sentAt: '10:00',
  createdAt: '2026-01-01 10:00:00',
  senderNickname: senderNickname,
  attachment: null,
  encrypted: false,
  confirmed: false,
  edited: false,
  latency: null,
  reactions: const [],
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

  testWidgets('an edited bubble shows the canonical local-edit marker', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Column(
            children: [
              MessageBubble(message: chatMessage(edited: true), isOwn: true),
              MessageBubble(message: chatMessage(id: 2), isOwn: true),
            ],
          ),
        ),
      ),
    );
    expect(find.byKey(const Key('edited-marker')), findsOneWidget);
    expect(find.text('(edited)'), findsOneWidget);
  });

  testWidgets('Edit updates one sent message locally and shows its marker', (
    tester,
  ) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(id: 7, content: 'original', msgId: 'dm-1'),
      ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    _selectMessage(tester, DmChatScreen, 0);
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const Key('edit-action')));
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('edit-message-field')), findsOneWidget);
    expect(
      tester
          .widget<TextField>(find.byKey(const Key('edit-message-field')))
          .controller
          ?.text,
      'original',
    );
    await tester.enterText(
      find.byKey(const Key('edit-message-field')),
      'fixed',
    );
    await tester.tap(find.text('Save'));
    await tester.pumpAndSettle();

    expect(fake.calls.where((c) => c.method == 'editMessage'), hasLength(1));
    expect(find.text('fixed'), findsOneWidget);
    expect(find.byKey(const Key('edited-marker')), findsOneWidget);
  });

  testWidgets('Edit is hidden for incoming messages', (tester) async {
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(
          id: 7,
          content: 'incoming',
          peerId: 'peer-abc',
          msgId: 'dm-1',
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
    expect(find.byKey(const Key('edit-action')), findsNothing);
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

  testWidgets('an incoming typing event shows and expires the indicator', (
    tester,
  ) async {
    final fake = FakeNodeApi();
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pump();
    await tester.pump();
    expect(find.byKey(const Key('peer-typing')), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'typing',
        peerId: 'peer-abc',
        content: 'start',
        encrypted: false,
      ),
    );
    await tester.pump();
    expect(find.byKey(const Key('peer-typing')), findsOneWidget);

    // The frontend-only TTL clears a stale indicator without a stop signal.
    await tester.pump(const Duration(seconds: 3));
    await tester.pump();
    expect(find.byKey(const Key('peer-typing')), findsNothing);
  });

  testWidgets('composer edits send throttled typing starts and stops', (
    tester,
  ) async {
    final fake = FakeNodeApi();
    fake.loadDmMessagesOverride = (_, _) => [];
    fake.saveOutgoingDmOverride = (peerId, content, replyTo) =>
        chatMessage(id: 9, content: content);
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pump();
    await tester.pump();

    final composer = find.byType(TextField);
    await tester.enterText(composer, 'h');
    await tester.pump();
    await tester.enterText(composer, 'hi');
    await tester.pump();
    expect(fake.typingSends.map((signal) => signal.isTyping).toList(), [
      true,
    ], reason: 'rapid edits are throttled');

    await tester.enterText(composer, '');
    await tester.pump();
    await tester.enterText(composer, 'hey');
    await tester.pump();
    await tester.tap(find.byIcon(Icons.send));
    await tester.pumpAndSettle();

    expect(fake.typingSends.map((signal) => signal.isTyping).toList(), [
      true,
      false,
      true,
      false,
    ]);
    expect(
      fake.typingSends.every((signal) => signal.peerId == 'peer-abc'),
      isTrue,
    );
  });

  testWidgets('multiline composer sends the whole body with line breaks', (
    tester,
  ) async {
    final fake = FakeNodeApi();
    fake.loadDmMessagesOverride = (_, _) => [];
    fake.saveOutgoingDmOverride = (peerId, content, replyTo) =>
        chatMessage(id: 10, content: content);
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pump();
    await tester.pump();

    final composer = find.byType(TextField);
    final field = tester.widget<TextField>(composer);
    expect(
      field.maxLines,
      isNull,
      reason: 'the field grows with the body instead of clipping at line one',
    );
    expect(field.minLines, 1, reason: 'starts compact at one line');
    expect(
      field.textInputAction,
      TextInputAction.newline,
      reason: 'return inserts a line break; the send button is the send action',
    );

    const body = 'first line\nsecond line\nthird line';
    await tester.enterText(composer, body);
    await tester.pump();
    await tester.tap(find.byIcon(Icons.send));
    await tester.pumpAndSettle();

    expect(fake.dmSends, hasLength(1));
    expect(fake.dmSends.single.content, body);
  });

  testWidgets('a bubble renders every line of a multiline body', (
    tester,
  ) async {
    const body = 'line one\nline two\nline three';
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(message: chatMessage(content: body), isOwn: true),
        ),
      ),
    );

    final bodyText = find.text(body);
    expect(bodyText, findsOneWidget);
    final text = tester.widget<Text>(bodyText);
    expect(
      text.maxLines,
      isNull,
      reason: 'no single-line clamp — the bubble grows vertically',
    );
    expect(text.overflow, isNull);
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

  testWidgets('cold start shows the DB-backed unread banner', (tester) async {
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..mobilePeerStatus = const MobilePeerStatus(
        databaseUrl: ':memory:',
        localPeerId: 'me',
      )
      ..broadcastUnreadCount = 2
      ..loadBroadcastMessagesOverride = (_) => [
        chatMessage(id: 1, msgId: 'old', content: 'old'),
        chatMessage(id: 2, msgId: 'new-1', content: 'new one'),
        chatMessage(id: 3, msgId: 'new-2', content: 'new two'),
      ];
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    // Init chains status -> history -> roster -> groups -> unread count.
    await tester.pump();
    await tester.pump();
    await tester.pump();

    expect(find.text('2 new messages'), findsOneWidget);
    expect(
      fake.calls.map((call) => call.method),
      contains('getBroadcastUnreadCount'),
    );
    // Showing unread rows must not silently clear the shared marker.
    expect(
      fake.calls.map((call) => call.method),
      isNot(contains('markBroadcastViewed')),
    );
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
    expect(mixed?.preview, 'see attached [file] notes.txt (2 KiB)');
  });

  test('a reply to our own message freezes the self nickname as sender', () {
    // Rust resolves our outgoing rows' senderNickname to the installed self
    // nickname, and the TUI captures that same `[nickname]` tag in its reply
    // snapshot. Flutter must not freeze the generic "Me" label instead.
    final draft = replyDraftFor(
      chatMessage(
        id: 1,
        content: 'my earlier note',
        peerId: null,
        msgId: 'own-1',
        senderNickname: 'Alice (abc)',
      ),
    );
    expect(draft?.sender, 'Alice (abc)');
    // Rows without a resolved nickname still fall back to "Me".
    expect(replyDraftFor(chatMessage(id: 2, msgId: 'own-2'))?.sender, 'Me');
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

  testWidgets('a DM screen shows only the open peer\'s broadcasts as context', (
    tester,
  ) async {
    final fake = FakeNodeApi();
    fake.loadDmMessagesOverride = (_, _) => [
      chatMessage(
        id: 1,
        content: 'private hello',
        peerId: 'peer-abc',
        sent: false,
      ),
    ];
    fake.loadBroadcastMessagesOverride = (_) => [
      broadcastMessage(
        id: 10,
        content: 'public from Alice',
        peerId: 'peer-abc',
        msgId: 'bc-alice',
        senderNickname: 'Alice',
      ),
      broadcastMessage(
        id: 11,
        content: 'public from someone else',
        peerId: 'peer-other',
        msgId: 'bc-other',
      ),
    ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('dm-broadcast-pane')), findsOneWidget);
    expect(find.text('Broadcast from Alice'), findsOneWidget);
    expect(find.text('public from Alice'), findsOneWidget);
    expect(find.text('public from someone else'), findsNothing);
    // The DM thread still renders below the context pane.
    expect(find.text('private hello'), findsOneWidget);
  });

  testWidgets(
    'a live broadcast from the open peer is mirrored without being persisted twice',
    (tester) async {
      final forwarded = <SwarmEventJson>[];
      setEventSink(forwarded.add);
      final fake = FakeNodeApi();
      fake.loadDmMessagesOverride = (_, _) => [];
      fake.loadBroadcastMessagesOverride = (_) => [];
      nodeApi = fake;
      startEventPolling();

      await tester.pumpWidget(
        MaterialApp(
          home: DmChatScreen(peer: peerRecord(), serviceRunning: true),
        ),
      );
      await tester.pumpAndSettle();

      fake.deliverEvent(
        const SwarmEventJson(
          eventType: 'broadcast',
          peerId: 'peer-abc',
          content: 'live public hello',
          msgId: 'bc-live',
          encrypted: false,
        ),
      );
      await tester.pumpAndSettle();

      expect(find.text('live public hello'), findsOneWidget);
      // The DM screen never persists broadcasts itself (home owns that via the
      // forwarded sink), so there is exactly one persistence path.
      expect(
        fake.calls.where((c) => c.method == 'saveIncomingMessage'),
        isEmpty,
      );
      // The event is still forwarded so the broadcast tab underneath stays live.
      expect(forwarded, hasLength(1));
      expect(forwarded.single.eventType, 'broadcast');
    },
  );

  testWidgets('a live reaction refreshes the DM broadcast-context chips', (
    tester,
  ) async {
    final fake = FakeNodeApi();
    fake.loadDmMessagesOverride = (_, _) => [];
    fake.loadBroadcastMessagesOverride = (_) => [
      broadcastMessage(
        id: 10,
        content: 'public from Alice',
        peerId: 'peer-abc',
        msgId: 'bc-rx',
        senderNickname: 'Alice',
      ),
    ];
    fake.getReactionsOverride = (msgId) async => const [
      ReactionSummary(emoji: '🎉', count: 3, mine: false),
    ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(home: DmChatScreen(peer: peerRecord(), serviceRunning: true)),
    );
    await tester.pumpAndSettle();
    expect(find.text('🎉 3'), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'reaction',
        peerId: 'peer-abc',
        encrypted: false,
        msgId: 'bc-rx',
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.text('🎉 3'), findsOneWidget);
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
          edited: false,
          createdAt: '2026-01-01 10:00:00',
          reactions: const [],
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

  testWidgets('reaction chips render canonical counts and mine marker', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(
              reactions: const [
                ReactionSummary(emoji: '👍', count: 2, mine: true),
                ReactionSummary(emoji: '🎉', count: 1, mine: false),
              ],
            ),
            isOwn: true,
          ),
        ),
      ),
    );
    expect(find.text('👍 2'), findsOneWidget);
    expect(find.text('🎉 1'), findsOneWidget);
    expect(find.byKey(const Key('reaction-👍')), findsOneWidget);
    expect(find.byKey(const Key('reaction-🎉')), findsOneWidget);
  });

  testWidgets('tapping a reaction chip asks the owning screen to toggle', (
    tester,
  ) async {
    String? toggled;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(
              reactions: const [
                ReactionSummary(emoji: '👍', count: 1, mine: false),
              ],
            ),
            isOwn: true,
            onReact: (emoji) => toggled = emoji,
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('reaction-👍')));
    await tester.pump();
    expect(toggled, '👍');
  });

  testWidgets('the reaction menu offers a Custom entry', (tester) async {
    nodeApi = FakeNodeApi(isRemoteTransport: true);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(),
            isOwn: true,
            onReact: (_) {},
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('add-reaction')));
    await tester.pumpAndSettle();
    expect(find.text('Custom…'), findsOneWidget);
    expect(find.text('👍'), findsOneWidget);
  });

  testWidgets('the quick-reaction palette comes from the Rust-backed API', (
    tester,
  ) async {
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..quickReactionsOverride = () async => const ['🔥', '🙌'];
    nodeApi = fake;
    String? toggled;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(),
            isOwn: true,
            onReact: (emoji) => toggled = emoji,
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('add-reaction')));
    await tester.pumpAndSettle();
    expect(find.text('🔥'), findsOneWidget);
    expect(find.text('🙌'), findsOneWidget);
    expect(find.text('👍'), findsNothing);
    expect(fake.calls.where((c) => c.method == 'quickReactions'), hasLength(1));

    await tester.tap(find.text('🔥'));
    await tester.pumpAndSettle();
    expect(toggled, '🔥');
  });

  testWidgets('a valid custom reaction token is forwarded trimmed', (
    tester,
  ) async {
    final fake = FakeNodeApi(isRemoteTransport: true);
    nodeApi = fake;
    String? toggled;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(),
            isOwn: true,
            onReact: (emoji) => toggled = emoji,
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('add-reaction')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Custom…'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '  🚀  ');
    await tester.tap(find.text('Add'));
    await tester.pumpAndSettle();
    expect(toggled, '🚀');
    // The dialog trims before validating so the forwarded token is canonical.
    expect(
      fake.calls.where((c) => c.method == 'validateEmoji').single.argument,
      '🚀',
    );
  });

  testWidgets('an invalid custom reaction shows an error and does not react', (
    tester,
  ) async {
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..validateEmojiOverride = (_) => Future.value(false);
    nodeApi = fake;
    String? toggled;
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: MessageBubble(
            message: chatMessage(),
            isOwn: true,
            onReact: (emoji) => toggled = emoji,
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const Key('add-reaction')));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Custom…'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), 'nope');
    await tester.tap(find.text('Add'));
    await tester.pumpAndSettle();
    expect(find.text('Enter 1–32 characters'), findsOneWidget);
    expect(toggled, isNull);
  });

  testWidgets('a live reaction event refreshes the broadcast chips', (
    tester,
  ) async {
    final fake = FakeNodeApi(isRemoteTransport: true);
    fake.mobilePeerStatus = const MobilePeerStatus(
      databaseUrl: ':memory:',
      localPeerId: 'me',
    );
    fake.loadBroadcastMessagesOverride = (_) => [chatMessage(msgId: 'bc-rx')];
    fake.getReactionsOverride = (msgId) async => const [
      ReactionSummary(emoji: '🎉', count: 3, mine: false),
    ];
    nodeApi = fake;

    await tester.pumpWidget(const MaterialApp(home: HomeScreen()));
    await tester.pump();
    await tester.pump();
    expect(find.text('🎉 3'), findsNothing);

    fake.deliverEvent(
      const SwarmEventJson(
        eventType: 'reaction',
        peerId: 'peer-abc',
        encrypted: false,
        msgId: 'bc-rx',
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(find.text('🎉 3'), findsOneWidget);
  });

  test('toggleReactionAt applies canonical summaries to the row', () async {
    final fake = FakeNodeApi(isRemoteTransport: true)
      ..toggleReactionOverride = (msgId, emoji) async => const [
        ReactionSummary(emoji: '❤️', count: 1, mine: true),
      ];
    nodeApi = fake;
    final messages = [chatMessage(msgId: 'm-tgl')];

    await toggleReactionAt(messages, 0, '❤️');

    expect(fake.reactionToggles, [(msgId: 'm-tgl', emoji: '❤️')]);
    expect(messages[0].reactions, [
      const ReactionSummary(emoji: '❤️', count: 1, mine: true),
    ]);
    expect(messages[0].content, 'hello', reason: 'only chips change');
  });

  testWidgets('tapping a DM sender name opens that sender\'s Peer Info', (
    tester,
  ) async {
    final opened = <String>[];
    final fake = FakeNodeApi()
      ..loadDmMessagesOverride = (_, _) => [
        chatMessage(
          id: 3,
          content: 'from peer',
          peerId: 'peer-abc',
          sent: false,
          senderNickname: 'Peer Sender',
        ),
      ];
    nodeApi = fake;
    startEventPolling();

    await tester.pumpWidget(
      MaterialApp(
        home: DmChatScreen(
          peer: peerRecord(),
          serviceRunning: true,
          onOpenPeerInfo: opened.add,
        ),
      ),
    );
    await tester.pumpAndSettle();
    // Drive the bubble's callback directly: the bubbles live inside a
    // SelectionArea, whose recognizers swallow synthetic taps in widget tests.
    // This still verifies the screen → body → bubble wiring under test; the
    // gesture-to-callback path itself is covered by MessageBubble's own tests.
    tester
        .widget<MessageBubble>(find.byType(MessageBubble).first)
        .onOpenPeerInfo!('peer-abc');

    expect(opened, ['peer-abc']);
  });

  testWidgets('tapping a group sender name opens that sender\'s Peer Info', (
    tester,
  ) async {
    final opened = <String>[];
    final fake = FakeNodeApi()
      ..loadGroupMessagesOverride = (_, _) => [
        MobileGroupMessage(
          id: 4,
          content: 'from group peer',
          peerId: 'group-sender',
          sent: false,
          edited: false,
          senderNickname: 'Group Sender',
          createdAt: '2026-01-01 10:00:00',
          reactions: const [],
        ),
      ];
    nodeApi = fake;

    await tester.pumpWidget(
      MaterialApp(
        home: GroupChatScreen(
          group: groupRecord(),
          serviceRunning: true,
          onOpenPeerInfo: opened.add,
        ),
      ),
    );
    await tester.pumpAndSettle();
    // See the DM test above: drive the bubble callback directly because the
    // SelectionArea swallows synthetic taps in widget tests.
    tester
        .widget<MessageBubble>(find.byType(MessageBubble).first)
        .onOpenPeerInfo!('group-sender');

    expect(opened, ['group-sender']);
  });
}
