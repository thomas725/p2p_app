// Shared test double for the global `nodeApi` (see
// lib/src/data/node_api.dart). Only the members a given test needs are
// configurable; everything else throws so an unexpected node call fails loudly
// instead of silently returning empty data.

import 'dart:typed_data';

// ignore_for_file: invalid_use_of_internal_member
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64;

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart'
    show BindShareParams, CreateShareParams, MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/location.dart' show LocationShareInput;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/rust/notifications.dart';
import 'package:p2p_app_flutter/src/rust/reactions.dart';
import 'package:p2p_app_flutter/src/rust/network_config.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareManifest, ShareOffer;
import 'package:p2p_app_flutter/src/rust/theme.dart';

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

  /// Mirrors the Rust-owned draft rule for tests that exercise reply wiring.
  ReplyRef? Function(
    String? msgId,
    String? senderNickname,
    String? peerId,
    String content,
    MobileAttachment? attachment,
  )?
  replyDraftOverride;

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
  Future<ReplyRef?> replyDraft({
    String? msgId,
    String? senderNickname,
    String? peerId,
    required String content,
    MobileAttachment? attachment,
  }) async {
    _record('replyDraft', msgId);
    final override = replyDraftOverride;
    if (override == null) {
      throw UnimplementedError('set replyDraftOverride in this test');
    }
    return override(msgId, senderNickname, peerId, content, attachment);
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
    // Every DM screen loads this feed for its broadcast-context pane, so an
    // empty default is the sensible baseline; tests that need rows override it.
    return loadBroadcastMessagesOverride?.call(limit) ?? const [];
  }

  /// Returned by [loadBroadcastMessages] when set; otherwise it yields [].
  List<ChatMessage> Function(int limit)? loadBroadcastMessagesOverride;

  /// In-memory broadcast unread watermark. Tests can seed it before pumping
  /// `HomeScreen`; [markBroadcastViewed] clears it just like the Rust marker.
  int broadcastUnreadCount = 0;

  @override
  Future<int> getBroadcastUnreadCount() async {
    _record('getBroadcastUnreadCount');
    return broadcastUnreadCount;
  }

  @override
  Future<void> markBroadcastViewed() async {
    _record('markBroadcastViewed');
    broadcastUnreadCount = 0;
  }

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
  Future<void> sendTyping({
    required String peerId,
    required bool isTyping,
  }) async {
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
    MobileAttachment? attachment,
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
      attachment,
    );
  }

  /// Builds the persisted incoming message for [saveIncomingMessage] when set;
  /// otherwise the call throws. The callback receives the raw arguments so a
  /// test can assert how the live event's `latency`, `msgId`, `replyTo`,
  /// `attachment` and `encrypted` flag were threaded through.
  ChatMessage Function(
    String content,
    String peerId,
    bool isDirect,
    String? nickname,
    String? latency,
    String? msgId,
    ReplyRef? replyTo,
    bool encrypted,
    MobileAttachment? attachment,
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

  /// In-memory theme snapshot; [setTheme] updates it so widget tests can
  /// toggle dark/accent and read the result back without FFI.
  AppTheme theme = const AppTheme(
    dark: true,
    accentName: 'teal',
    accentArgb: 0xff2f5d50,
    accents: [
      AccentOption(name: 'teal', argb: 0xff2f5d50),
      AccentOption(name: 'blue', argb: 0xff2f5d9e),
    ],
  );

  @override
  Future<AppTheme> getTheme() async {
    _record('getTheme');
    return theme;
  }

  @override
  Future<void> setTheme({
    required bool dark,
    required String accentName,
  }) async {
    _record('setTheme', {'dark': dark, 'accentName': accentName});
    final swatch = theme.accents
        .where((a) => a.name == accentName)
        .cast<AccentOption>()
        .toList();
    theme = AppTheme(
      dark: dark,
      accentName: accentName,
      accentArgb: swatch.isNotEmpty ? swatch.first.argb : theme.accentArgb,
      accents: theme.accents,
    );
  }

  /// In-memory notification settings; [setNotificationSettings] updates them
  /// so widget tests can toggle both channels without FFI.
  NotificationSettings notificationSettings = const NotificationSettings(
    sound: true,
    desktop: false,
  );

  @override
  Future<NotificationSettings> getNotificationSettings() async {
    _record('getNotificationSettings');
    return notificationSettings;
  }

  @override
  Future<void> setNotificationSettings({
    required bool sound,
    required bool desktop,
  }) async {
    _record('setNotificationSettings', {'sound': sound, 'desktop': desktop});
    notificationSettings = NotificationSettings(sound: sound, desktop: desktop);
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

  /// Override the custom-reaction validation result. Defaults to accepting
  /// any non-empty token; tests can force a rejection to exercise the dialog.
  Future<bool> Function(String emoji)? validateEmojiOverride;

  @override
  Future<bool> validateEmoji({required String emoji}) {
    _record('validateEmoji', emoji);
    final override = validateEmojiOverride;
    if (override != null) return override(emoji);
    return Future.value(emoji.trim().isNotEmpty && emoji.trim().length <= 32);
  }

  /// Override the Rust-owned palette so a widget test can prove the menu reads
  /// it from the API rather than a Dart constant.
  Future<List<String>> Function()? quickReactionsOverride;

  @override
  Future<List<String>> quickReactions() {
    _record('quickReactions');
    final override = quickReactionsOverride;
    if (override != null) return override();
    return Future.value(const ['👍', '❤️', '😂', '🎉', '😮', '😢']);
  }

  /// Override the network-size label returned by the Rust boundary helper.
  /// Defaults to throwing so an unconfigured test fails loudly.
  Future<String> Function(int peerCount)? networkSizeLabelOverride;

  @override
  Future<String> networkSizeLabel({required int peerCount}) {
    _record('networkSizeLabel', peerCount);
    final override = networkSizeLabelOverride;
    if (override != null) return override(peerCount);
    throw UnimplementedError('set networkSizeLabelOverride in this test');
  }

  /// Rust-owned location helpers. Tests override these rather than formatting
  /// coordinates or URLs in Dart.
  Future<String> Function(double latitude, double longitude)?
  locationMapUrlOverride;
  Future<String> Function(LocationShareInput input)?
  buildLocationMessageOverride;
  Future<bool> Function(String content)? isLocationMessageOverride;

  @override
  Future<String> locationMapUrl({
    required double latitude,
    required double longitude,
  }) {
    _record('locationMapUrl', {'latitude': latitude, 'longitude': longitude});
    final override = locationMapUrlOverride;
    if (override != null) return override(latitude, longitude);
    throw UnimplementedError('set locationMapUrlOverride in this test');
  }

  @override
  Future<String> buildLocationMessage({required LocationShareInput input}) {
    _record('buildLocationMessage', input);
    final override = buildLocationMessageOverride;
    if (override != null) return override(input);
    throw UnimplementedError('set buildLocationMessageOverride in this test');
  }

  @override
  Future<bool> isLocationMessage({required String content}) {
    _record('isLocationMessage', content);
    final override = isLocationMessageOverride;
    if (override != null) return override(content);
    throw UnimplementedError('set isLocationMessageOverride in this test');
  }

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

  // --- Calls: Rust-owned media transport (recorded, no FFI) ---

  @override
  Future<void> startCallMedia({
    required String callId,
    required String peerId,
  }) async {
    _record('startCallMedia', (callId: callId, peerId: peerId));
  }

  @override
  Future<void> stopCallMedia() async {
    _record('stopCallMedia');
  }

  @override
  Future<void> sendCallMedia({
    required String callId,
    required List<int> pcm,
  }) async {
    _record('sendCallMedia', (callId: callId, pcm: pcm));
  }

  @override
  Future<void> setCallMediaMute({required bool muted}) async {
    _record('setCallMediaMute', muted);
  }

  // --- Versioned file shares (recorded; no FFI) ---

  @override
  Future<List<MobileShare>> listShares() async {
    _record('listShares');
    return const [];
  }

  @override
  Future<MobileShare?> getShare(String shareId) => throw UnimplementedError();

  @override
  Future<MobileShare> createShare({required CreateShareParams params}) =>
      throw UnimplementedError();

  @override
  Future<void> bindShare({required BindShareParams params}) =>
      throw UnimplementedError();

  @override
  Future<void> deleteShare(String shareId) => throw UnimplementedError();

  @override
  Future<MobileShare> setShareVisibility({
    required String shareId,
    required String kind,
    required bool publicWrite,
  }) => throw UnimplementedError();

  @override
  Future<List<MobileShareGrant>> listShareGrants(String shareId) =>
      throw UnimplementedError();

  @override
  Future<void> setShareGrant({
    required String shareId,
    required String userId,
    required String mode,
  }) => throw UnimplementedError();

  @override
  Future<void> removeShareGrant({
    required String shareId,
    required String userId,
  }) => throw UnimplementedError();

  @override
  Future<ShareManifest?> loadShareManifest(String shareId) =>
      throw UnimplementedError();

  @override
  Future<void> saveShareManifest({
    required String shareId,
    required ShareManifest manifest,
  }) => throw UnimplementedError();

  @override
  Future<MobileShare> acceptShareOffer({required ShareOffer offer}) =>
      throw UnimplementedError();

  @override
  Future<ShareManifest> snapshotShare({
    required String shareId,
    required String root,
  }) => throw UnimplementedError();

  @override
  Future<void> requestPeerShares(String peerId) => throw UnimplementedError();

  @override
  Future<void> requestShareManifest({
    required String peerId,
    required String shareId,
  }) => throw UnimplementedError();

  @override
  Future<void> readvertiseShare({
    required String shareId,
    required String peerId,
  }) => throw UnimplementedError();

  @override
  Future<void> publishShareSnapshot({
    required String peerId,
    required PlatformInt64 baseRevision,
    required ShareManifest manifest,
  }) => throw UnimplementedError();
}
