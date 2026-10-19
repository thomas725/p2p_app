// Unit tests for the web transport ([HttpNodeApi]) — the browser client half
// of docs/2026-10-18_web_frontend.md. A fake [RemoteTransport] stands in for
// the WebSocket + HTTP routes, so these tests pin the wire contract without a
// network: request frames ({id, method, params}), response handling
// ({id, ok|err}), pushed event frames, attachment upload, and reconnect.

import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/http_node_api.dart';
import 'package:p2p_app_flutter/src/data/node_transport.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// In-memory stand-in for `WsRemoteTransport`: records outbound frames and
/// uploads, and lets tests inject response/event frames.
class FakeRemoteTransport implements RemoteTransport {
  final List<String> sentFrames = [];
  final List<(String, Uint8List)> uploads = [];
  final List<String> closedFileIds = [];

  /// Called with every outbound frame (after recording); tests use it to
  /// answer requests synchronously.
  void Function(String frame)? onSend;

  final StreamController<String> _controller = StreamController<String>();

  @override
  Stream<String> get frames => _controller.stream;

  @override
  Future<void> get ready async {}

  @override
  Future<void> send(String frame) async {
    sentFrames.add(frame);
    onSend?.call(frame);
  }

  @override
  Future<Map<String, dynamic>> upload({
    required String name,
    required Uint8List bytes,
  }) async {
    uploads.add((name, bytes));
    return {
      'attachment': {
        'file_id': 'sha-$name',
        'name': name,
        'size': bytes.length,
        'mime': null,
      },
    };
  }

  @override
  String downloadUrl(String fileId) {
    closedFileIds.add(fileId);
    return 'http://node.test/api/files/$fileId';
  }

  @override
  void close() {
    unawaited(_controller.close());
  }

  // --- test injection helpers ---

  /// Decode the frame at [index] of the recorded outbound frames.
  Map<String, dynamic> frame(int index) =>
      jsonDecode(sentFrames[index]) as Map<String, dynamic>;

  /// Push a `{id, ok}` response for a previously captured request frame.
  void respond(Map<String, dynamic> request, {required Object? ok}) {
    _controller.add(jsonEncode({'id': request['id'], 'ok': ok}));
  }

  /// Push a `{id, err}` response for a previously captured request frame.
  void respondError(Map<String, dynamic> request, String error) {
    _controller.add(jsonEncode({'id': request['id'], 'err': error}));
  }

  /// Push a server-side swarm event.
  void pushEvent(Map<String, dynamic> event) {
    _controller.add(jsonEncode({'event': event}));
  }

  /// Push arbitrary raw text (e.g. a non-protocol frame).
  void pushRaw(String raw) => _controller.add(raw);

  /// Simulate a dropped connection (all streams end; pending calls fail).
  void drop() => unawaited(_controller.close());

  /// Let queued frames + completers settle.
  static Future<void> pump() => Future<void>.delayed(Duration.zero);
}

/// Wire fixture: a camelCase-serialized `ChatMessage` with a descriptor.
Map<String, dynamic> chatMessageJson({bool withAttachment = false}) => {
  'id': 7,
  'content': 'hello',
  'peerId': null,
  'isBroadcast': true,
  'targetPeer': null,
  'sent': true,
  'msgId': 'm1',
  'sentAt': '2026-10-19 10:00:00',
  'createdAt': '2026-10-19 10:00:00',
  'senderNickname': null,
  'attachment': withAttachment
      ? {
          'fileId': 'abc123',
          'name': 'notes.txt',
          'size': 2048,
          'mime': 'text/plain',
        }
      : null,
};

void main() {
  group('HttpNodeApi request frames', () {
    test('loadBroadcastMessages sends method + params and parses messages',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(0);
        fake.respond(request, ok: [
          chatMessageJson(),
          chatMessageJson(withAttachment: true),
        ]);
      };
      final api = HttpNodeApi(connect: () async => fake);

      final messages = await api.loadBroadcastMessages(limit: 200);

      final request = fake.frame(0);
      expect(request['method'], 'loadBroadcastMessages');
      expect(request['params'], {'limit': 200});
      expect(request['id'], isA<int>());
      expect(messages, hasLength(2));
      final first = messages.first;
      expect(first.id, 7);
      expect(first.content, 'hello');
      expect(first.isBroadcast, isTrue);
      expect(first.peerId, isNull);
      final attachment = messages.last.attachment!;
      expect(attachment.fileId, 'abc123');
      expect(attachment.size.toInt(), 2048);
      expect(attachment.mime, 'text/plain');
    });

    test('loadDmMessages and saveOutgoingDm route peer-specific params',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(fake.sentFrames.length - 1);
        // loadDmMessages returns a list; saveOutgoingDm returns one message.
        fake.respond(
          request,
          ok: request['method'] == 'loadDmMessages'
              ? [chatMessageJson()]
              : chatMessageJson(),
        );
      };
      final api = HttpNodeApi(connect: () async => fake);

      await api.loadDmMessages(peerId: 'peer-x', limit: 50);
      await api.saveOutgoingDm(peerId: 'peer-x', content: 'yo');

      expect(fake.frame(0)['method'], 'loadDmMessages');
      expect(fake.frame(0)['params'], {'peerId': 'peer-x', 'limit': 50});
      expect(fake.frame(1)['method'], 'saveOutgoingDm');
      expect(fake.frame(1)['params'], {'peerId': 'peer-x', 'content': 'yo'});
    });

    test('error frames surface as exceptions with the server message',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respondError(fake.frame(0), 'boom');
      };
      final api = HttpNodeApi(connect: () async => fake);

      await expectLater(
        api.getLogs(),
        throwsA(
          isA<Exception>().having(
            (e) => e.toString(),
            'message',
            contains('boom'),
          ),
        ),
      );
    });

    test('null results (void methods) complete with null', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respond(fake.frame(fake.sentFrames.length - 1), ok: null);
      };
      final api = HttpNodeApi(connect: () async => fake);

      await api.clearLogs();
      await api.stopNode();
      expect(fake.frame(0)['method'], 'clearLogs');
      expect(fake.frame(1)['method'], 'stopNode');
    });

    test('deleteGroup and setPeerLocalNickname route camelCase params',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respond(fake.frame(fake.sentFrames.length - 1), ok: null);
      };
      final api = HttpNodeApi(connect: () async => fake);

      await api.deleteGroup(groupId: 'g-1');
      await api.setPeerLocalNickname(peerId: 'peer-x', nickname: 'bob');

      expect(fake.frame(0)['method'], 'deleteGroup');
      expect(fake.frame(0)['params'], {'groupId': 'g-1'});
      expect(fake.frame(1)['method'], 'setPeerLocalNickname');
      expect(fake.frame(1)['params'], {
        'peerId': 'peer-x',
        'nickname': 'bob',
      });
    });

    test('non-protocol frames are ignored, not fatal', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.pushRaw('this is not json');
        fake.pushRaw('[1,2,3]');
        fake.respond(fake.frame(0), ok: <String>[]);
      };
      final api = HttpNodeApi(connect: () async => fake);

      expect(await api.getLogs(), isEmpty);
    });
  });

  group('HttpNodeApi pushed events', () {
    test('event frames reach the pump callback as SwarmEventJson', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respond(fake.frame(0), ok: null);
      };
      final api = HttpNodeApi(connect: () async => fake);

      SwarmEventJson? received;
      api.startEventPump((event) => received = event);

      fake.pushEvent({
        'eventType': 'device_binding_intro',
        'peerId': 'peer-9',
        'devicePubkey': [4, 5, 6],
      });
      await FakeRemoteTransport.pump();

      expect(received, isNotNull);
      expect(received!.eventType, 'device_binding_intro');
      expect(received!.peerId, 'peer-9');
      expect(received!.devicePubkey, [4, 5, 6]);

      api.stopEventPump();
      fake.pushEvent({'eventType': 'peer_found', 'peerId': 'peer-10'});
      await FakeRemoteTransport.pump();
      expect(received!.eventType, 'device_binding_intro'); // unchanged
    });

    test('response frames are not delivered as events', () async {
      final fake = FakeRemoteTransport();
      final delivered = <SwarmEventJson>[];
      fake.onSend = (frame) {
        fake.respond(fake.frame(0), ok: <String>[]);
      };
      final api = HttpNodeApi(connect: () async => fake)
        ..startEventPump(delivered.add);

      await api.getLogs();
      await FakeRemoteTransport.pump();
      expect(delivered, isEmpty);
    });
  });

  group('HttpNodeApi attachments', () {
    test('byte sends upload first, then pass the descriptor to the save call',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(fake.sentFrames.length - 1);
        if (request['method'] == 'saveOutgoingBroadcast') {
          fake.respond(request, ok: chatMessageJson(withAttachment: true));
        }
      };
      final api = HttpNodeApi(connect: () async => fake);
      final bytes = Uint8List.fromList([1, 2, 3, 4]);

      final message = await api.saveOutgoingBroadcast(
        content: '',
        attachmentBytes: bytes,
        attachmentName: 'photo.png',
      );

      expect(fake.uploads.single.$1, 'photo.png');
      expect(fake.uploads.single.$2, bytes);
      // Uploads travel over HTTP, not the WS protocol — the save call is the
      // only frame.
      final request = fake.frame(0);
      expect(request['method'], 'saveOutgoingBroadcast');
      final params = request['params']! as Map<String, dynamic>;
      expect(params['content'], '');
      // The descriptor rides in the persisted snake_case shape.
      expect(params['attachment'], {
        'file_id': 'sha-photo.png',
        'name': 'photo.png',
        'size': 4,
        'mime': null,
      });
      expect(message.attachment!.name, 'notes.txt');
    });

    test('resolveAttachmentLocation returns the transport download URL',
        () async {
      final fake = FakeRemoteTransport();
      final api = HttpNodeApi(connect: () async => fake);

      final url = await api.resolveAttachmentLocation('abc123');
      expect(url, 'http://node.test/api/files/abc123');
      // Pure URL resolution — no frame should have been sent.
      expect(fake.sentFrames, isEmpty);
    });
  });

  group('HttpNodeApi model conversion', () {
    test('getContactsWithStats parses rows, devices, and stat counts',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respond(fake.frame(0), ok: [
          {
            'contactId': 'user-1',
            'isUser': true,
            'displayName': 'Alice',
            'deviceCount': 2,
            'deviceRows': [
              {
                'peerId': 'peer-a',
                'displayName': 'Alice phone',
                'firstSeen': '2026-10-01 00:00:00',
                'lastSeen': '2026-10-02 00:00:00',
                'receivedNickname': null,
                'localNickname': 'Al',
              },
            ],
            'firstSeen': '2026-10-01 00:00:00',
            'lastSeen': '2026-10-02 00:00:00',
            'dmCount': 3,
            'broadcastSentToPeer': 1,
          },
        ]);
      };
      final api = HttpNodeApi(connect: () async => fake);

      final rows = await api.getContactsWithStats();
      final row = rows.single;
      expect(row.contactId, 'user-1');
      expect(row.deviceCount, BigInt.two);
      expect(row.dmCount.toInt(), 3);
      expect(row.broadcastSentToPeer.toInt(), 1);
      expect(row.deviceRows.single.localNickname, 'Al');
    });

    test('sortContacts round-trips rows with integer counts', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(0);
        // Answer with the same rows the client sent (server echoes order).
        fake.respond(request, ok: (request['params']! as Map<String, dynamic>)['contacts']);
      };
      final api = HttpNodeApi(connect: () async => fake);
      final row = ContactSortInput(
        contactId: 'user-1',
        displayName: 'Alice',
        lastSeen: '2026-10-02 00:00:00',
        firstSeen: '2026-10-01 00:00:00',
        dmCount: 5,
        broadcastSentToPeer: 2,
      );

      final sorted = await api.sortContacts(
        contacts: [row],
        sortColumn: 1,
        ascending: false,
      );

      final params = fake.frame(0)['params']! as Map<String, dynamic>;
      expect(params['sortColumn'], 1);
      expect(params['ascending'], isFalse);
      // i64 fields travel as plain JSON numbers.
      expect((params['contacts'] as List).single['dmCount'], 5);
      final parsed = sorted.single;
      expect(parsed.contactId, 'user-1');
      expect(parsed.dmCount.toInt(), 5);
      expect(parsed.broadcastSentToPeer.toInt(), 2);
    });

    test('getPeersWithStats parses nullable nicknames and counts', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        fake.respond(fake.frame(0), ok: [
          {
            'peerId': 'peer-1',
            'displayName': 'Alice',
            'firstSeen': '2026-10-01 00:00:00',
            'lastSeen': '2026-10-02 00:00:00',
            'nickname': 'received',
            'localNickname': null,
            'dmCount': 4,
            'broadcastSentToPeer': 0,
          },
        ]);
      };
      final api = HttpNodeApi(connect: () async => fake);

      final rows = await api.getPeersWithStats();
      expect(rows.single.nickname, 'received');
      expect(rows.single.localNickname, isNull);
      expect(rows.single.dmCount.toInt(), 4);
    });

    test('status, identity, and pure-helper shapes parse', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(fake.sentFrames.length - 1);
        switch (request['method']) {
          case 'getMobilePeerStatus':
            fake.respond(request, ok: {
              'databaseUrl': 'sqlite:///tmp/x.db',
              'localPeerId': '12D3KooWTEST',
              'selfNickname': 'Me',
            });
          case 'getMyUserId':
            fake.respond(request, ok: 'user-42');
          case 'validateNickname':
            fake.respond(request, ok: true);
          case 'networkSizeLabel':
            fake.respond(request, ok: '3 peers connected');
          default:
            fail('unexpected method ${request['method']}');
        }
      };
      final api = HttpNodeApi(connect: () async => fake);

      final status = await api.getMobilePeerStatus();
      expect(status.databaseUrl, 'sqlite:///tmp/x.db');
      expect(status.localPeerId, '12D3KooWTEST');
      expect(status.selfNickname, 'Me');

      // initMobileDatabase mirrors the server status (server owns the DB).
      final init = await api.initMobileDatabase(dbPath: 'ignored');
      expect(init.localPeerId, '12D3KooWTEST');
      expect(init.databaseUrl, 'sqlite:///tmp/x.db');

      expect(await api.getMyUserId(), 'user-42');
      expect(await api.validateNickname(nickname: 'ok'), isTrue);
      expect(
        await api.networkSizeLabel(peerCount: 3),
        '3 peers connected',
      );
    });
  });

  group('HttpNodeApi pure-helper memo', () {
    test('formatTimeHhmm caches Rust answers for synchronous reads',
        () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(0);
        final dt = (request['params']! as Map<String, dynamic>)['dt']! as String;
        // Mirror the Rust rule: HH:MM when the string is long enough.
        fake.respond(
          request,
          ok: dt.length >= 16 ? dt.substring(11, 16) : dt,
        );
      };
      final api = HttpNodeApi(connect: () async => fake);
      const dt = '2026-10-19 08:42:00';

      expect(api.formatTimeHhmmSync(dt: dt), isNull); // cold miss
      expect(await api.formatTimeHhmm(dt: dt), '08:42');
      // Memo hit is synchronous and identical to Rust's answer.
      expect(api.formatTimeHhmmSync(dt: dt), '08:42');
      expect(fake.sentFrames, hasLength(1)); // memo prevents a second round trip
    });

    test('isAtBottom forwards the geometry as integers', () async {
      final fake = FakeRemoteTransport();
      fake.onSend = (frame) {
        final request = fake.frame(0);
        fake.respond(request, ok: true);
      };
      final api = HttpNodeApi(connect: () async => fake);

      expect(
        await api.isAtBottom(scrollOffset: 500, total: 580, visible: 80),
        isTrue,
      );
      expect(fake.frame(0)['params'], {
        'scrollOffset': 500,
        'total': 580,
        'visible': 80,
      });
    });
  });

  group('HttpNodeApi connection lifecycle', () {
    test('pending calls fail on drop; the next call reconnects', () async {
      final fakes = <FakeRemoteTransport>[];
      final api = HttpNodeApi(connect: () async {
        final fake = FakeRemoteTransport();
        fakes.add(fake);
        return fake;
      });

      // First call hangs (no response), then the connection drops.
      final hanging = api.getLogs();
      await FakeRemoteTransport.pump();
      fakes.single.drop();
      await expectLater(hanging, throwsA(isA<Exception>()));

      // A fresh connection is established transparently for the next call.
      // (The factory runs synchronously inside the call's first step, so the
      // second fake exists before its request frame is sent.)
      final second = api.getLogs();
      fakes[1].onSend = (frame) {
        fakes[1].respond(fakes[1].frame(0), ok: ['log line']);
      };
      expect(await second, ['log line']);
      expect(fakes, hasLength(2));
    });

    test('repeated calls reuse one connection', () async {
      var connects = 0;
      late FakeRemoteTransport fake;
      final api = HttpNodeApi(connect: () async {
        connects++;
        fake = FakeRemoteTransport();
        fake.onSend = (frame) {
          fake.respond(fake.frame(fake.sentFrames.length - 1), ok: []);
        };
        return fake;
      });

      await api.clearLogs();
      await api.stopNode();
      api.startEventPump((_) {}); // pump registration is transport-local

      expect(connects, 1);
      expect(fake.sentFrames, hasLength(2));
      expect(fake.frame(1)['method'], 'stopNode');
    });
  });
}
