import 'package:p2p_app_flutter/src/data/node_api.dart';

/// Toggle this install's blocklist entry for [peerId], returning the new state.
///
/// Shared by every Peer Info entry point (Contacts and DM chat) so blocking has
/// exactly one behaviour. The persisted list is Rust's (`blockPeer` /
/// `unblockPeer`); enforcement lives in the swarm boundary, never here.
Future<bool> togglePeerBlock(String peerId) async {
  if (await nodeApi.isPeerBlocked(peerId: peerId)) {
    await nodeApi.unblockPeer(peerId: peerId);
    return false;
  }
  await nodeApi.blockPeer(peerId: peerId);
  return true;
}

/// Read-side companion for the Peer Info page's "Blocked" row.
Future<bool> peerIsBlocked(String peerId) =>
    nodeApi.isPeerBlocked(peerId: peerId);
