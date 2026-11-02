import 'dart:async';
import 'dart:typed_data';

// PlatformInt64Util is frb's int<->i64 adapter (int on IO, BigInt on web);
// it is internal-to-frb but the only portable converter the generated code
// exposes, hence the same ignore the generated files carry.
// ignore_for_file: invalid_use_of_internal_member

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64Util;

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart' as bridge;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart' as mobile;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart' as node;
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';

/// In-process [NodeApi]: delegates to the generated flutter_rust_bridge
/// bindings (the historical mobile behavior). This is the default transport
/// on non-web builds; the web build swaps in `HttpNodeApi` in `main()`.
class FrbNodeApi implements NodeApi {
  Timer? _pollTimer;

  @override
  bool get isRemoteTransport => false;

  // ── Node lifecycle ────────────────────────────────────────────────────────

  @override
  Future<MobileInitStatus> initMobileDatabase({required String dbPath}) =>
      bridge.initMobileDatabase(dbPath: dbPath);

  @override
  Future<MobilePeerStatus> getMobilePeerStatus() =>
      bridge.getMobilePeerStatus();

  @override
  Future<NetworkDetails> getNetworkDetails() => bridge.getNetworkDetails();

  @override
  Future<String> startNode({required String dbPath}) =>
      bridge.startNode(dbPath: dbPath);

  @override
  Future<String> startNodeAuto() => bridge.startNodeAuto();

  @override
  Future<void> stopNode() => bridge.stopNode();

  // ── Event pump ────────────────────────────────────────────────────────────

  @override
  void startEventPump(void Function(SwarmEventJson event) onEvent) {
    stopEventPump();
    _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
      try {
        // Drain the queue each tick: same batch bound as the web server's
        // poller so a busy node cannot starve the UI loop.
        for (var i = 0; i < 100; i++) {
          final event = await bridge.pollEvent();
          if (event == null) break;
          onEvent(event);
        }
      } catch (_) {
        // Rust call failure mid-tick: keep the timer alive for the next one.
      }
    });
  }

  @override
  void stopEventPump() {
    _pollTimer?.cancel();
    _pollTimer = null;
  }

  // ── Logs ──────────────────────────────────────────────────────────────────

  @override
  Future<List<String>> getLogs() => bridge.getLogs();

  @override
  Future<void> clearLogs() => bridge.clearLogs();

  // ── Messages ──────────────────────────────────────────────────────────────

  @override
  Future<List<ChatMessage>> loadBroadcastMessages({required int limit}) =>
      bridge.loadBroadcastMessages(limit: PlatformInt64Util.from(limit));

  @override
  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  }) => bridge.loadDmMessages(
    peerId: peerId,
    limit: PlatformInt64Util.from(limit),
  );

  @override
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingBroadcast(
      content: content,
      attachmentPath: attachmentPath,
    );
  }

  @override
  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingDm(
      peerId: peerId,
      content: content,
      attachmentPath: attachmentPath,
    );
  }

  @override
  Future<ChatMessage> saveIncomingMessage({
    required String content,
    required String peerId,
    required bool isDirect,
    String? nickname,
    String? latency,
  }) => bridge.saveIncomingMessage(
    content: content,
    peerId: peerId,
    isDirect: isDirect,
    nickname: nickname,
    latency: latency,
  );

  @override
  Future<void> dialPeer({required String address}) =>
      bridge.dialPeer(address: address);

  // ── Groups ────────────────────────────────────────────────────────────────

  @override
  Future<MobileGroup> createGroup({required String name}) =>
      bridge.createGroup(name: name);

  @override
  Future<List<MobileGroup>> listGroups() => bridge.listGroups();

  @override
  Future<List<MobileGroupMessage>> loadGroupMessages({
    required String groupId,
    required int limit,
  }) => bridge.loadGroupMessages(
    groupId: groupId,
    limit: PlatformInt64Util.from(limit),
  );

  @override
  Future<MobileGroupMessage> saveOutgoingGroup({
    required String groupId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingGroup(
      groupId: groupId,
      content: content,
      attachmentPath: attachmentPath,
    );
  }

  @override
  Future<MobileGroup> acceptGroupInvite({required String groupId}) =>
      bridge.acceptGroupInvite(groupId: groupId);

  @override
  Future<void> deleteGroup({required String groupId}) =>
      bridge.deleteGroup(groupId: groupId);

  // ── Nickname / self ───────────────────────────────────────────────────────

  @override
  Future<void> setSelfNickname({required String nickname}) =>
      bridge.setSelfNickname(nickname: nickname);

  @override
  Future<void> setPeerLocalNickname({
    required String peerId,
    required String nickname,
  }) => bridge.setPeerLocalNickname(peerId: peerId, nickname: nickname);

  @override
  Future<bool> validateNickname({required String nickname}) =>
      bridge.validateNickname(nickname: nickname);

  @override
  Future<String> networkSizeLabel({required int peerCount}) =>
      bridge.networkSizeLabel(
        peerCount: PlatformInt64Util.from(peerCount),
      );

  // ── Identity / device binding ─────────────────────────────────────────────

  @override
  Future<String?> getMyUserId() => bridge.getMyUserId();

  @override
  Future<List<MobileDeviceInfo>> getUserDevices({required String userId}) =>
      bridge.getUserDevices(userId: userId);

  @override
  Future<void> sendDeviceBindingIntro({required String peerId}) =>
      bridge.sendDeviceBindingIntro(peerId: peerId);

  @override
  Future<void> acceptDeviceBinding({
    required String peerId,
    required Uint8List devicePubkey,
  }) => bridge.acceptDeviceBinding(
    peerId: peerId,
    devicePubkey: devicePubkey,
  );

  // ── Peers + contacts ──────────────────────────────────────────────────────

  @override
  Future<List<PeerWithStats>> getPeersWithStats() async => mobile
      .getPeersWithStats();

  @override
  Future<List<MobileContactRow>> getContactsWithStats() =>
      bridge.getContactsWithStats();

  @override
  Future<List<ContactSortInput>> sortContacts({
    required List<ContactSortInput> contacts,
    required int sortColumn,
    required bool ascending,
  }) async => mobile.sortContacts(
    contacts: contacts,
    sortColumn: sortColumn,
    ascending: ascending,
  );

  // ── Calls ─────────────────────────────────────────────────────────────────

  @override
  Future<String> startCall({required String peerId}) =>
      bridge.startCall(peerId: peerId);

  @override
  Future<void> sendCallSignal({
    required String peerId,
    required String callId,
    required String kind,
  }) => bridge.sendCallSignal(peerId: peerId, callId: callId, kind: kind);

  // ── Pure UI helpers (Rust is the single source) ───────────────────────────

  @override
  String? formatTimeHhmmSync({required String dt}) =>
      mobile.formatTimeHhmm(dt: dt);

  @override
  Future<String> formatTimeHhmm({required String dt}) async =>
      mobile.formatTimeHhmm(dt: dt);

  @override
  Future<bool> isAtBottom({
    required int scrollOffset,
    required int total,
    required int visible,
  }) async => mobile.isAtBottom(
    scrollOffset: scrollOffset,
    total: total,
    visible: visible,
  );

  // ── Attachments ───────────────────────────────────────────────────────────

  @override
  Future<String> resolveAttachmentLocation(String fileId) =>
      node.attachmentFilePath(fileId: fileId);

  /// The in-process transport sends attachments by filesystem path (pickers
  /// hand us one there); raw bytes are the remote/web path only.
  static void _rejectBytesOnlyAttachments(Uint8List? attachmentBytes) {
    if (attachmentBytes != null) {
      throw UnsupportedError(
        'byte-array attachments are only supported by the remote transport; '
        'the in-process transport sends by file path',
      );
    }
  }
}
