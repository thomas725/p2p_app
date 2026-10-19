// Shared test double for the global `nodeApi` (see
// lib/src/data/node_api.dart). Only the members a given test needs are
// configurable; everything else throws so an unexpected node call fails loudly
// instead of silently returning empty data.

import 'dart:typed_data';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

class FakeNodeApi implements NodeApi {
  FakeNodeApi({this.isRemoteTransport = false});

  @override
  bool isRemoteTransport;

  // --- configurable behavior ---

  /// Returned by [resolveAttachmentLocation] verbatim.
  String Function(String fileId)? resolveAttachmentLocationOverride;

  /// Returned by [formatTimeHhmm] (and its sync variant when set).
  String Function(String dt)? formatTimeHhmmOverride;

  /// When set, [isAtBottom] returns this instead of the default `false`.
  bool Function(int scrollOffset, int total, int visible)? isAtBottomOverride;

  /// Recorded calls (method name + arguments), for assertions.
  final List<({String method, Object? argument})> calls = [];

  void _record(String method, [Object? argument]) =>
      calls.add((method: method, argument: argument));

  // --- event pump ---

  void Function(SwarmEventJson event)? onEvent;

  @override
  void startEventPump(void Function(SwarmEventJson event) onEvent) {
    _record('startEventPump');
    this.onEvent = onEvent;
  }

  @override
  void stopEventPump() {
    _record('stopEventPump');
    onEvent = null;
  }

  /// Deliver an event to whatever sink the pump was started with (mimics both
  /// transports' delivery into the shared event bus).
  void deliverEvent(SwarmEventJson event) => onEvent?.call(event);

  // --- pure UI helpers ---

  @override
  String? formatTimeHhmmSync({required String dt}) =>
      formatTimeHhmmOverride?.call(dt);

  @override
  Future<String> formatTimeHhmm({required String dt}) async {
    _record('formatTimeHhmm', dt);
    final override = formatTimeHhmmOverride;
    if (override == null) {
      throw UnimplementedError('set formatTimeHhmmOverride in this test');
    }
    return override(dt);
  }

  @override
  Future<bool> isAtBottom({
    required int scrollOffset,
    required int total,
    required int visible,
  }) async => isAtBottomOverride?.call(scrollOffset, total, visible) ?? false;

  // --- attachments ---

  @override
  Future<String> resolveAttachmentLocation(String fileId) async {
    _record('resolveAttachmentLocation', fileId);
    final override = resolveAttachmentLocationOverride;
    if (override != null) return override(fileId);
    return isRemoteTransport
        ? 'http://node.test/api/files/$fileId'
        : '/tmp/p2p-files/$fileId';
  }

  // --- everything else: not implemented (fail loudly) ---

  @override
  Future<MobileInitStatus> initMobileDatabase({required String dbPath}) =>
      throw UnimplementedError();

  @override
  Future<MobilePeerStatus> getMobilePeerStatus() => throw UnimplementedError();

  @override
  Future<String> startNode({required String dbPath}) =>
      throw UnimplementedError();

  @override
  Future<String> startNodeAuto() => throw UnimplementedError();

  @override
  Future<void> stopNode() => throw UnimplementedError();

  @override
  Future<List<String>> getLogs() => throw UnimplementedError();

  @override
  Future<void> clearLogs() => throw UnimplementedError();

  @override
  Future<List<ChatMessage>> loadBroadcastMessages({required int limit}) =>
      throw UnimplementedError();

  @override
  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  }) => throw UnimplementedError();

  @override
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) => throw UnimplementedError();

  @override
  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) => throw UnimplementedError();

  @override
  Future<ChatMessage> saveIncomingMessage({
    required String content,
    required String peerId,
    required bool isDirect,
    String? nickname,
  }) => throw UnimplementedError();

  @override
  Future<MobileGroup> createGroup({required String name}) =>
      throw UnimplementedError();

  @override
  Future<List<MobileGroup>> listGroups() => throw UnimplementedError();

  @override
  Future<List<MobileGroupMessage>> loadGroupMessages({
    required String groupId,
    required int limit,
  }) => throw UnimplementedError();

  @override
  Future<MobileGroupMessage> saveOutgoingGroup({
    required String groupId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) => throw UnimplementedError();

  @override
  Future<MobileGroup> acceptGroupInvite({required String groupId}) =>
      throw UnimplementedError();

  /// Group ids passed to [deleteGroup] (records each call).
  final List<String> deletedGroups = [];

  @override
  Future<void> deleteGroup({required String groupId}) async {
    _record('deleteGroup', groupId);
    deletedGroups.add(groupId);
  }

  @override
  Future<void> setSelfNickname({required String nickname}) =>
      throw UnimplementedError();

  /// `(peerId, nickname)` pairs passed to [setPeerLocalNickname].
  final List<(String, String)> peerLocalNicknames = [];

  @override
  Future<void> setPeerLocalNickname({
    required String peerId,
    required String nickname,
  }) async {
    _record('setPeerLocalNickname', peerId);
    peerLocalNicknames.add((peerId, nickname));
  }

  @override
  Future<bool> validateNickname({required String nickname}) =>
      throw UnimplementedError();

  @override
  Future<String> networkSizeLabel({required int peerCount}) =>
      throw UnimplementedError();

  @override
  Future<String?> getMyUserId() => throw UnimplementedError();

  @override
  Future<List<MobileDeviceInfo>> getUserDevices({required String userId}) =>
      throw UnimplementedError();

  @override
  Future<void> sendDeviceBindingIntro({required String peerId}) =>
      throw UnimplementedError();

  @override
  Future<void> acceptDeviceBinding({
    required String peerId,
    required Uint8List devicePubkey,
  }) => throw UnimplementedError();

  @override
  Future<List<PeerWithStats>> getPeersWithStats() =>
      throw UnimplementedError();

  @override
  Future<List<MobileContactRow>> getContactsWithStats() =>
      throw UnimplementedError();

  @override
  Future<List<ContactSortInput>> sortContacts({
    required List<ContactSortInput> contacts,
    required int sortColumn,
    required bool ascending,
  }) => throw UnimplementedError();

  @override
  Future<String> startCall({required String peerId}) =>
      throw UnimplementedError();

  @override
  Future<void> sendCallSignal({
    required String peerId,
    required String callId,
    required String kind,
  }) => throw UnimplementedError();
}
