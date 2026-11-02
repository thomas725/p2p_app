// Unit tests for the attachment helpers (files lane §12) across both
// transports: native resolves a filesystem path and hands it to the system
// opener seam; the remote/web transport resolves a download URL and hands it
// to the browser-open seam.

import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/util/attachments.dart';

import '../helpers/fake_node_api.dart';

ChatMessage messageWithAttachment(String fileId) => ChatMessage(
  id: 1,
  content: '',
  peerId: 'peer-1',
  isBroadcast: false,
  sent: true,
  msgId: 'm1',
  sentAt: '2026-10-19 10:00:00',
  createdAt: '2026-10-19 10:00:00',
  senderNickname: null,
  attachment: MobileAttachment(
    fileId: fileId,
    name: 'notes.txt',
    size: 12,
    sizeLabel: '12 B',
    mime: 'text/plain',
  ),
  encrypted: false,
  confirmed: false,
);

void main() {
  late FakeNodeApi fake;
  late OpenFileOverride previousOpen;
  late OpenRemoteUrlOverride previousRemote;

  setUp(() {
    fake = FakeNodeApi();
    nodeApi = fake;
    previousOpen = openFileOverride;
    previousRemote = openRemoteUrlOverride;
  });

  tearDown(() {
    openFileOverride = previousOpen;
    openRemoteUrlOverride = previousRemote;
  });

  group('openAttachedFile', () {
    test('native transport opens the resolved filesystem path', () async {
      String? openedPath;
      openFileOverride = (path) async {
        openedPath = path;
        return null;
      };
      final error = await openAttachedFile(messageWithAttachment('abc'));

      expect(error, isNull);
      expect(openedPath, '/tmp/p2p-files/abc');
      expect(fake.calls.single.method, 'resolveAttachmentLocation');
    });

    test('remote transport opens the resolved download URL', () async {
      fake.isRemoteTransport = true;
      String? openedUrl;
      openRemoteUrlOverride = (url) async {
        openedUrl = url;
        return null;
      };
      final error = await openAttachedFile(messageWithAttachment('abc'));

      expect(error, isNull);
      expect(openedUrl, 'http://node.test/api/files/abc');
    });

    test('opener failures surface as error strings, not exceptions', () async {
      fake.isRemoteTransport = true;
      openRemoteUrlOverride = (url) async => 'the browser refused';
      final error = await openAttachedFile(messageWithAttachment('abc'));
      expect(error, 'the browser refused');
    });

    test('a message without an attachment is a no-op', () async {
      var resolverCalled = false;
      fake.resolveAttachmentLocationOverride = (fileId) {
        resolverCalled = true;
        return fileId;
      };
      final message = ChatMessage(
        id: 2,
        content: 'plain',
        peerId: 'peer-1',
        isBroadcast: false,
        sent: true,
        msgId: 'm2',
        sentAt: null,
        createdAt: '2026-10-19 10:00:00',
        senderNickname: null,
        attachment: null,
        encrypted: false,
        confirmed: false,
      );

      expect(await openAttachedFile(message), isNull);
      expect(resolverCalled, isFalse);
    });

    test('resolution failures surface as error strings', () async {
      fake.resolveAttachmentLocationOverride =
          (fileId) => throw StateError('blob missing');
      final error = await openAttachedFile(messageWithAttachment('abc'));
      expect(error, contains('blob missing'));
    });
  });

  group('PickedAttachment seam', () {
    test('carries either a path or bytes, never neither', () {
      expect(
        () => PickedAttachment(name: 'x'),
        throwsA(isA<AssertionError>()),
      );
      expect(PickedAttachment(path: '/a', name: 'x').bytes, isNull);
      expect(PickedAttachment(bytes: null, path: '/a', name: 'x').path, '/a');
    });
  });
}
