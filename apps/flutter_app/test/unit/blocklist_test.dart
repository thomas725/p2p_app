// Unit tests for the shared blocklist toggle helper: the Rust list is the only
// authority, so the Dart side must only flip the persisted entry and report the
// new state.

import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/util/blocklist.dart';

import '../helpers/fake_node_api.dart';

void main() {
  test(
    'togglePeerBlock blocks, then unblocks, through the transport',
    () async {
      final fake = FakeNodeApi();
      nodeApi = fake;

      expect(await peerIsBlocked('peer-1'), isFalse);

      expect(await togglePeerBlock('peer-1'), isTrue);
      expect(fake.blockedPeers, contains('peer-1'));
      expect(await peerIsBlocked('peer-1'), isTrue);

      expect(await togglePeerBlock('peer-1'), isFalse);
      expect(fake.blockedPeers, isNot(contains('peer-1')));
      expect(await peerIsBlocked('peer-1'), isFalse);

      expect(fake.calls.map((call) => call.method).toList(), [
        'isPeerBlocked',
        'isPeerBlocked',
        'blockPeer',
        'isPeerBlocked',
        'isPeerBlocked',
        'unblockPeer',
        'isPeerBlocked',
      ]);
    },
  );

  test('a rejected peer id surfaces the transport error', () async {
    final fake = FakeNodeApi()..blockPeerError = 'invalid peer id';
    nodeApi = fake;

    await expectLater(togglePeerBlock('bad'), throwsA('invalid peer id'));
  });
}
