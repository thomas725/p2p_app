// Shared test double for the global `nodeApi` (see
// lib/src/data/node_api.dart). Only the members a given test needs are
// configurable; everything else throws so an unexpected node call fails loudly
// instead of silently returning empty data.

import 'dart:typed_data';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';

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
  Future<MobilePeerStatus> getMobilePeerStatus() async {
    _record('getMobilePeerStatus');
    final status = mobilePeerStatus;
    if (status == null) {
      throw UnimplementedError('set mobilePeerStatus in this test');
    }
    return status;
  }

  /// Returned by [getMobilePeerStatus] when set; otherwise the call throws.
  MobilePeerStatus? mobilePeerStatus;

  @override
  Future<NetworkDetails> getNetworkDetails() async {
    _record('getNetworkDetails');
    final details = networkDetails;
    if (details == null) {
      throw UnimplementedError('set networkDetails in this test');
    }
    return details;
  }

  /// Returned by [getNetworkDetails] when set; otherwise the call throws.
  NetworkDetails? networkDetails;

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
  Future<List<ChatMessage>> loadBroadcastMessages({required int limit}) async {
    _record('loadBroadcastMessages', limit);
    final override = loadBroadcastMessagesOverride;
    if (override == null) {
      throw UnimplementedError('set loadBroadcastMessagesOverride in this test');
    }
    return override(limit);
  }

  /// Returned by [loadBroadcastMessages] when set; otherwise the call throws.
  List<ChatMessage> Function(int limit)? loadBroadcastMessagesOverride;

  @override
  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  }) async {
    _record('loadDmMessages', peerId);
    final override = loadDmMessagesOverride;
    if (override == null) {
      throw UnimplementedError('set loadDmMessagesOverride in this test');
    }
    return override(peerId, limit);
  }

  /// Returned by [loadDmMessages] when set; otherwise the call throws.
  List<ChatMessage> Function(String peerId, int limit)? loadDmMessagesOverride;

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
    String? latency,
  }) async {
    _record('saveIncomingMessage', content);
    final override = saveIncomingMessageOverride;
    if (override == null) {
      throw UnimplementedError('set saveIncomingMessageOverride in this test');
    }
    return override(content, peerId, isDirect, nickname, latency);
  }

  /// Builds the persisted incoming message for [saveIncomingMessage] when set;
  /// otherwise the call throws. The callback receives the raw arguments so a
  /// test can assert how the live event's `latency` was threaded through.
  ChatMessage Function(
    String content,
    String peerId,
    bool isDirect,
    String? nickname,
    String? latency,
  )? saveIncomingMessageOverride;

  @override
  Future<void> dialPeer({required String address}) async {}

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
  }) async {
    _record('sortContacts', contacts.length);
    final override = sortContactsOverride;
    // Rust owns contact ordering. The fake returns the rows unchanged unless an
    // override mirrors the real semantics (as `contact_list_test.dart` does).
    return override == null ? contacts : override(contacts, sortColumn, ascending);
  }

  /// Replaces Rust's canonical contact ordering in [sortContacts] when set.
  List<ContactSortInput> Function(
    List<ContactSortInput> contacts,
    int sortColumn,
    bool ascending,
  )? sortContactsOverride;

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
