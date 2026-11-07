// Transport abstraction over the p2p node (docs/web_frontend.md).
//
// Every screen talks to the node through [NodeApi]. Two transports exist:
//  - [FrbNodeApi]: in-process flutter_rust_bridge FFI (Android/mobile), the
//    historical behavior;
//  - [HttpNodeApi]: JSON over WebSocket/HTTP to a `p2p_chat_server` process
//    (web build) — the node stays native on the server; the browser is a
//    pure display client.
//
// The interface mirrors the FRB surface 1:1 and is fully async so both
// transports fit; the pure UI helpers (time format, at-bottom, sorting,
// nickname validation) still execute in Rust on both paths — Dart never
// re-implements them.
library;

import 'dart:typed_data';

import 'package:p2p_app_flutter/src/data/frb_node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';

/// The active transport. Defaults to the in-process FFI bridge; `main()`
/// swaps in [HttpNodeApi] for the web build. Tests may replace it with a
/// fake implementing this interface.
NodeApi nodeApi = FrbNodeApi();

/// Node access surface shared by every screen (see library doc).
abstract class NodeApi {
  /// Whether calls round-trip to a remote node server (web) or execute
  /// in-process (mobile). Lets platform-specific helpers (attachment
  /// opening, Android services) branch on where the node lives.
  bool get isRemoteTransport;

  // --- Node lifecycle / status ---

  /// Initialize the database at [dbPath] and return peer info. On the remote
  /// transport the server owns the database; this returns the server's current
  /// status and ignores [dbPath].
  Future<MobileInitStatus> initMobileDatabase({required String dbPath});

  Future<MobilePeerStatus> getMobilePeerStatus();

  /// Live network diagnostics: OS interfaces (name/MAC/IPv4/IPv6), AutoNAT/
  /// DCUtR reachability, relay reservations, and DHT connectivity/announcement
  /// state. One polled snapshot shared by the Settings views.
  Future<NetworkDetails> getNetworkDetails();

  /// Live ping-based connection quality for [peerId] (RTT, ping counts, loss),
  /// with the Rust-formatted display labels. Zeroed/`n/a` when the peer is not
  /// connected or no ping has completed yet.
  Future<PeerQuality> getPeerQuality(String peerId);

  Future<String> startNode({required String dbPath});

  Future<String> startNodeAuto();

  Future<void> stopNode();

  // --- Event pump ---

  /// Deliver every swarm event to [onEvent] until [stopEventPump]. FFI: a
  /// 200 ms poll loop over `pollEvent`; remote: server-pushed frames.
  void startEventPump(void Function(SwarmEventJson event) onEvent);

  void stopEventPump();

  // --- Logs ---

  Future<List<String>> getLogs();

  Future<void> clearLogs();

  // --- Broadcast / DM messages ---

  Future<List<ChatMessage>> loadBroadcastMessages({required int limit});

  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  });

  /// Send a broadcast. Attachments arrive either as a server-local path (FFI)
  /// or as raw [attachmentBytes] + [attachmentName] that the remote transport
  /// uploads to the node first (files lane §12).
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  });

  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  });

  Future<ChatMessage> saveIncomingMessage({
    required String content,
    required String peerId,
    required bool isDirect,
    String? nickname,
    String? latency,
  });

  Future<void> dialPeer({required String address});

  // --- Groups ---

  Future<MobileGroup> createGroup({required String name});

  Future<List<MobileGroup>> listGroups();

  Future<List<MobileGroupMessage>> loadGroupMessages({
    required String groupId,
    required int limit,
  });

  Future<MobileGroupMessage> saveOutgoingGroup({
    required String groupId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  });

  Future<MobileGroup> acceptGroupInvite({required String groupId});

  /// Leave a group: delete its local rows and unsubscribe from its topic.
  /// Local-only — other members keep their copy of the history.
  Future<void> deleteGroup({required String groupId});

  // --- Nicknames / labels ---

  Future<void> setSelfNickname({required String nickname});

  /// Set the local nickname shown for a peer (this install only).
  Future<void> setPeerLocalNickname({
    required String peerId,
    required String nickname,
  });

  Future<bool> validateNickname({required String nickname});

  Future<String> networkSizeLabel({required int peerCount});

  // --- Identity / device binding ---

  Future<String?> getMyUserId();

  Future<List<MobileDeviceInfo>> getUserDevices({required String userId});

  Future<void> sendDeviceBindingIntro({required String peerId});

  Future<void> acceptDeviceBinding({
    required String peerId,
    required Uint8List devicePubkey,
  });

  // --- Peers + contacts ---

  /// Peers table rows with message counts in one round trip (replaces the
  /// former per-peer stat N+1).
  Future<List<PeerWithStats>> getPeersWithStats();

  Future<List<MobileContactRow>> getContactsWithStats();

  Future<List<ContactSortInput>> sortContacts({
    required List<ContactSortInput> contacts,
    required int sortColumn,
    required bool ascending,
  });

  // --- Calls (signaling only) ---

  Future<String> startCall({required String peerId});

  Future<void> sendCallSignal({
    required String peerId,
    required String callId,
    required String kind,
  });

  // --- Pure UI helpers (executed in Rust on every transport) ---

  Future<String> formatTimeHhmm({required String dt});

  /// Synchronous variant of [formatTimeHhmm] for hot render paths. The
  /// in-process transport always answers (its Rust call is sync); the remote
  /// transport returns its memoized Rust answer, or `null` on a cold miss —
  /// callers must then fall back to [formatTimeHhmm] (results are pure, so a
  /// memo hit can never diverge from Rust).
  String? formatTimeHhmmSync({required String dt});

  Future<bool> isAtBottom({
    required int scrollOffset,
    required int total,
    required int visible,
  });

  // --- Attachments ---

  /// Resolve an attachment blob for opening. The FFI transport returns the
  /// node's on-disk path (open with the system opener); the remote transport
  /// returns a download URL served by the node.
  Future<String> resolveAttachmentLocation(String fileId);
}
