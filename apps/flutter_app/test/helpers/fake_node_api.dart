// Shared test double for the global `nodeApi` (see
// lib/src/data/node_api.dart). Only the members a given test needs are
// configurable; everything else throws so an unexpected node call fails loudly
// instead of silently returning empty data.

import 'dart:typed_data';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/rust/reactions.dart';
import 'package:p2p_app_flutter/src/rust/network_config.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';

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
  Future<PeerQuality> getPeerQuality(String peerId) async {
    _record('getPeerQuality', peerId);
    final quality = peerQuality;
    if (quality == null) {
      throw UnimplementedError('set peerQuality in this test');
    }
    return quality;
  }

  /// Returned by [getPeerQuality] when set; otherwise the call throws.
  PeerQuality? peerQuality;

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
      throw UnimplementedError(
        'set loadBroadcastMessagesOverride in this test',
      );
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

  /// Called by [deleteMessage]. Tests may leave it unset and assert via [calls]
  /// instead; an override is useful for injecting a failure.
  Future<void> Function(int messageId, bool isGroup)? deleteMessageOverride;

  @override
  Future<void> deleteMessage({
    required int messageId,
    required bool isGroup,
  }) async {
    _record('deleteMessage', (messageId: messageId, isGroup: isGroup));
    final override = deleteMessageOverride;
    if (override != null) return override(messageId, isGroup);
  }

  /// Called by [editMessage]; an override can inject a failure.
  Future<void> Function(int messageId, bool isGroup, String content)?
  editMessageOverride;

  @override
  Future<void> editMessage({
    required int messageId,
    required bool isGroup,
    required String content,
  }) async {
    _record('editMessage', (
      messageId: messageId,
      isGroup: isGroup,
      content: content,
    ));
    final override = editMessageOverride;
    if (override != null) return override(messageId, isGroup, content);
  }

  /// Every typing signal emitted by the UI, in call order.
  final List<({String peerId, bool isTyping})> typingSends = [];

  @override
  Future<void> sendTyping({required String peerId, required bool isTyping}) async {
    _record('sendTyping', (peerId: peerId, isTyping: isTyping));
    typingSends.add((peerId: peerId, isTyping: isTyping));
  }

  /// Overrides reaction toggle/get; when unset, toggles are recorded and return
  /// an empty summary list, and gets return the last stored summaries.
  Future<List<ReactionSummary>> Function(String msgId, String emoji)?
  toggleReactionOverride;
  Future<List<ReactionSummary>> Function(String msgId)? getReactionsOverride;

  /// Every reaction toggle emitted by the UI, in call order.
  final List<({String msgId, String emoji})> reactionToggles = [];

  @override
  Future<List<ReactionSummary>> toggleReaction({
    required String msgId,
    required String emoji,
  }) async {
    _record('toggleReaction', (msgId: msgId, emoji: emoji));
    reactionToggles.add((msgId: msgId, emoji: emoji));
    final override = toggleReactionOverride;
    if (override != null) return override(msgId, emoji);
    return const [];
  }

  @override
  Future<List<ReactionSummary>> getReactions({required String msgId}) async {
    _record('getReactions', msgId);
    final override = getReactionsOverride;
    if (override != null) return override(msgId);
    return const [];
  }

  /// Recorded outgoing sends, in call order. Each includes the frozen reply
  /// draft (when one was active) so a test can assert the quote was threaded
  /// through to the transport.
  final List<({String content, ReplyRef? replyTo})> broadcastSends = [];
  final List<({String peerId, String content, ReplyRef? replyTo})> dmSends = [];
  final List<({String groupId, String content, ReplyRef? replyTo})> groupSends =
      [];

  /// Returned by the outgoing save methods when set; otherwise they throw.
  ChatMessage Function(String content, ReplyRef? replyTo)?
  saveOutgoingBroadcastOverride;
  ChatMessage Function(String peerId, String content, ReplyRef? replyTo)?
  saveOutgoingDmOverride;
  MobileGroupMessage Function(
    String groupId,
    String content,
    ReplyRef? replyTo,
  )?
  saveOutgoingGroupOverride;

  @override
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  }) async {
    broadcastSends.add((content: content, replyTo: replyTo));
    _record('saveOutgoingBroadcast', (content: content, replyTo: replyTo));
    final override = saveOutgoingBroadcastOverride;
    if (override == null) {
      throw UnimplementedError(
        'set saveOutgoingBroadcastOverride in this test',
      );
    }
    return override(content, replyTo);
  }

  @override
  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  }) async {
    dmSends.add((peerId: peerId, content: content, replyTo: replyTo));
    _record('saveOutgoingDm', (content: content, replyTo: replyTo));
    final override = saveOutgoingDmOverride;
    if (override == null) {
      throw UnimplementedError('set saveOutgoingDmOverride in this test');
    }
    return override(peerId, content, replyTo);
  }

  @override
  Future<ChatMessage> saveIncomingMessage({
    required String content,
    required String peerId,
    required bool isDirect,
    String? nickname,
    String? latency,
    String? msgId,
    ReplyRef? replyTo,
    required bool encrypted,
  }) async {
    _record('saveIncomingMessage', content);
    final override = saveIncomingMessageOverride;
    if (override == null) {
      throw UnimplementedError('set saveIncomingMessageOverride in this test');
    }
    return override(
      content,
      peerId,
      isDirect,
      nickname,
      latency,
      msgId,
      replyTo,
      encrypted,
    );
  }

  /// Builds the persisted incoming message for [saveIncomingMessage] when set;
  /// otherwise the call throws. The callback receives the raw arguments so a
  /// test can assert how the live event's `latency`, `msgId`, `replyTo` and
  /// `encrypted` flag were threaded through.
  ChatMessage Function(
    String content,
    String peerId,
    bool isDirect,
    String? nickname,
    String? latency,
    String? msgId,
    ReplyRef? replyTo,
    bool encrypted,
  )?
  saveIncomingMessageOverride;

  @override
  Future<void> dialPeer({required String address}) async {}

  // --- persisted network configuration ---

  /// Persisted network peers returned by [listNetworkPeers]; [addNetworkPeer]
  /// appends to it so a test can add + list without stubbing.
  List<NetworkPeer> networkPeers = [];

  /// Overrides the value [addNetworkPeer] returns (e.g. to simulate Rust-side
  /// validation/normalisation of the address).
  NetworkPeer Function(String kind, String multiaddr)? addNetworkPeerOverride;

  /// Error thrown by [addNetworkPeer] when set (validation failures).
  String? addNetworkPeerError;

  @override
  Future<List<NetworkPeer>> listNetworkPeers() async {
    _record('listNetworkPeers');
    return List.of(networkPeers);
  }

  @override
  Future<NetworkPeer> addNetworkPeer({
    required String kind,
    required String multiaddr,
  }) async {
    _record('addNetworkPeer', '$kind $multiaddr');
    final error = addNetworkPeerError;
    if (error != null) {
      throw error;
    }
    final override = addNetworkPeerOverride;
    if (override != null) {
      final row = override(kind, multiaddr);
      networkPeers.add(row);
      return row;
    }
    final row = NetworkPeer(
      id: networkPeers.length + 1,
      kind: kind,
      multiaddr: multiaddr,
      createdAt: '2026-11-08 12:00:00',
    );
    networkPeers.add(row);
    return row;
  }

  @override
  Future<bool> removeNetworkPeer({required int id}) async {
    _record('removeNetworkPeer', id);
    final before = networkPeers.length;
    networkPeers.removeWhere((peer) => peer.id == id);
    return networkPeers.length != before;
  }

  /// In-memory LAN-only flag; [setLanOnly] updates it so a test can toggle and
  /// read back without stubbing.
  bool lanOnly = false;

  @override
  Future<bool> getLanOnly() async {
    _record('getLanOnly');
    return lanOnly;
  }

  @override
  Future<void> setLanOnly({required bool enabled}) async {
    _record('setLanOnly', enabled);
    lanOnly = enabled;
  }

  // --- peer blocklist / mute ---

  /// The persisted blocklist the fake serves; edits apply to it, so a test can
  /// block then read back without stubbing.
  final Set<String> blockedPeers = {};

  /// Error thrown by [blockPeer] when set (an invalid peer id).
  String? blockPeerError;

  @override
  Future<bool> isPeerBlocked({required String peerId}) async {
    _record('isPeerBlocked', peerId);
    return blockedPeers.contains(peerId);
  }

  @override
  Future<bool> blockPeer({required String peerId}) async {
    _record('blockPeer', peerId);
    final error = blockPeerError;
    if (error != null) throw error;
    return blockedPeers.add(peerId);
  }

  @override
  Future<bool> unblockPeer({required String peerId}) async {
    _record('unblockPeer', peerId);
    return blockedPeers.remove(peerId);
  }

  @override
  Future<List<String>> listBlockedPeers() async {
    _record('listBlockedPeers');
    return List.of(blockedPeers);
  }

  @override
  Future<MobileGroup> createGroup({required String name}) =>
      throw UnimplementedError();

  @override
  Future<List<MobileGroup>> listGroups() => throw UnimplementedError();

  /// Returned by [loadGroupMessages] when set; otherwise the call throws.
  List<MobileGroupMessage> Function(String groupId, int limit)?
  loadGroupMessagesOverride;

  @override
  Future<List<MobileGroupMessage>> loadGroupMessages({
    required String groupId,
    required int limit,
  }) async {
    _record('loadGroupMessages', groupId);
    final override = loadGroupMessagesOverride;
    if (override == null) {
      throw UnimplementedError('set loadGroupMessagesOverride in this test');
    }
    return override(groupId, limit);
  }

  @override
  Future<MobileGroupMessage> saveOutgoingGroup({
    required String groupId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  }) async {
    groupSends.add((groupId: groupId, content: content, replyTo: replyTo));
    _record('saveOutgoingGroup', (content: content, replyTo: replyTo));
    final override = saveOutgoingGroupOverride;
    if (override == null) {
      throw UnimplementedError('set saveOutgoingGroupOverride in this test');
    }
    return override(groupId, content, replyTo);
  }

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
  Future<List<PeerWithStats>> getPeersWithStats() => throw UnimplementedError();

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
    return override == null
        ? contacts
        : override(contacts, sortColumn, ascending);
  }

  /// Replaces Rust's canonical contact ordering in [sortContacts] when set.
  List<ContactSortInput> Function(
    List<ContactSortInput> contacts,
    int sortColumn,
    bool ascending,
  )?
  sortContactsOverride;

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
