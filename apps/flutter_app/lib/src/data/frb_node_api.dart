import 'dart:async';
import 'dart:typed_data';

// PlatformInt64Util is frb's int<->i64 adapter (int on IO, BigInt on web);
// it is internal-to-frb but the only portable converter the generated code
// exposes, hence the same ignore the generated files carry.
// ignore_for_file: invalid_use_of_internal_member

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64, PlatformInt64Util;

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart' as bridge;
import 'package:p2p_app_flutter/src/rust/location.dart' as location;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart' as mobile;
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart' as node;
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/rust/notifications.dart';
import 'package:p2p_app_flutter/src/rust/reactions.dart';
import 'package:p2p_app_flutter/src/rust/network_config.dart';
import 'package:p2p_app_flutter/src/rust/theme.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/rust/shares.dart' as shares;

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
  Future<PeerQuality> getPeerQuality(String peerId) =>
      bridge.getPeerQuality(peerId: peerId);

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
  Future<int> getBroadcastUnreadCount() => bridge.getBroadcastUnreadCount();

  @override
  Future<void> markBroadcastViewed() => bridge.markBroadcastViewed();

  @override
  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  }) => bridge.loadDmMessages(
    peerId: peerId,
    limit: PlatformInt64Util.from(limit),
  );

  @override
  Future<void> deleteMessage({required int messageId, required bool isGroup}) =>
      bridge.deleteMessage(messageId: messageId, isGroup: isGroup);

  @override
  Future<void> editMessage({
    required int messageId,
    required bool isGroup,
    required String content,
  }) => bridge.editMessage(
    messageId: messageId,
    isGroup: isGroup,
    content: content,
  );

  @override
  Future<void> sendTyping({required String peerId, required bool isTyping}) =>
      bridge.sendTyping(peerId: peerId, isTyping: isTyping);

  @override
  Future<List<ReactionSummary>> toggleReaction({
    required String msgId,
    required String emoji,
  }) => bridge.toggleReaction(msgId: msgId, emoji: emoji);

  @override
  Future<List<ReactionSummary>> getReactions({required String msgId}) =>
      bridge.getReactions(msgId: msgId);

  @override
  Future<bool> validateEmoji({required String emoji}) =>
      bridge.validateEmoji(emoji: emoji);

  @override
  Future<List<String>> quickReactions() => bridge.quickReactions();

  @override
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingBroadcast(
      content: content,
      attachmentPath: attachmentPath,
      replyTo: replyTo,
    );
  }

  @override
  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingDm(
      peerId: peerId,
      content: content,
      attachmentPath: attachmentPath,
      replyTo: replyTo,
    );
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
  }) => bridge.saveIncomingMessage(
    params: bridge.SaveIncomingMessageParams(
      content: content,
      peerId: peerId,
      isDirect: isDirect,
      nickname: nickname,
      latency: latency,
      msgId: msgId,
      replyTo: replyTo,
      attachment: attachment,
      encrypted: encrypted,
    ),
  );

  @override
  Future<void> dialPeer({required String address}) =>
      bridge.dialPeer(address: address);

  // ── Persisted network configuration ──────────────────────────────────────

  @override
  Future<List<NetworkPeer>> listNetworkPeers() => bridge.listNetworkPeers();

  @override
  Future<NetworkPeer> addNetworkPeer({
    required String kind,
    required String multiaddr,
  }) => bridge.addNetworkPeer(kind: kind, multiaddr: multiaddr);

  @override
  Future<bool> removeNetworkPeer({required int id}) =>
      bridge.removeNetworkPeer(id: id);

  @override
  Future<bool> getLanOnly() => bridge.getLanOnly();

  @override
  Future<void> setLanOnly({required bool enabled}) =>
      bridge.setLanOnly(enabled: enabled);

  @override
  Future<AppTheme> getTheme() => bridge.getTheme();

  @override
  Future<void> setTheme({required bool dark, required String accentName}) =>
      bridge.setTheme(dark: dark, accentName: accentName);

  @override
  Future<NotificationSettings> getNotificationSettings() =>
      bridge.getNotificationSettings();

  @override
  Future<void> setNotificationSettings({
    required bool sound,
    required bool desktop,
  }) => bridge.setNotificationSettings(sound: sound, desktop: desktop);

  // ── Peer blocklist / mute (US-054) ───────────────────────────────────────

  @override
  Future<bool> isPeerBlocked({required String peerId}) =>
      bridge.isPeerBlocked(peerId: peerId);

  @override
  Future<bool> blockPeer({required String peerId}) =>
      bridge.blockPeer(peerId: peerId);

  @override
  Future<bool> unblockPeer({required String peerId}) =>
      bridge.unblockPeer(peerId: peerId);

  @override
  Future<List<String>> listBlockedPeers() => bridge.listBlockedPeers();

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
    ReplyRef? replyTo,
  }) {
    _rejectBytesOnlyAttachments(attachmentBytes);
    return bridge.saveOutgoingGroup(
      groupId: groupId,
      content: content,
      attachmentPath: attachmentPath,
      replyTo: replyTo,
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
      bridge.networkSizeLabel(peerCount: PlatformInt64Util.from(peerCount));

  @override
  Future<String> locationMapUrl({
    required double latitude,
    required double longitude,
  }) => bridge.locationMapUrl(latitude: latitude, longitude: longitude);

  @override
  Future<String> buildLocationMessage({
    required location.LocationShareInput input,
  }) => bridge.buildLocationMessage(input: input);

  @override
  Future<bool> isLocationMessage({required String content}) =>
      bridge.isLocationMessage(content: content);

  @override
  Future<bool> locationSessionActive({
    required double startedAt,
    required String mode,
    int? durationSecs,
    required double now,
  }) => bridge.locationSessionActive(
    startedAt: startedAt,
    mode: mode,
    durationSecs: durationSecs,
    now: now,
  );

  @override
  Future<bool> locationSendDue({
    double? lastSentAt,
    required double now,
    required int intervalSecs,
  }) => bridge.locationSendDue(
    lastSentAt: lastSentAt,
    now: now,
    intervalSecs: intervalSecs,
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
  }) => bridge.acceptDeviceBinding(peerId: peerId, devicePubkey: devicePubkey);

  // ── Peers + contacts ──────────────────────────────────────────────────────

  @override
  Future<List<PeerWithStats>> getPeersWithStats() async =>
      mobile.getPeersWithStats();

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

  @override
  Future<void> startCallMedia({
    required String callId,
    required String peerId,
  }) => bridge.startCallMedia(callId: callId, peerId: peerId);

  @override
  Future<void> stopCallMedia() => bridge.stopCallMedia();

  @override
  Future<void> sendCallMedia({
    required String callId,
    required List<int> pcm,
  }) => bridge.sendCallMedia(callId: callId, pcm: pcm);

  @override
  Future<void> setCallMediaMute({required bool muted}) =>
      bridge.setCallMediaMute(muted: muted);

  // ── Versioned file shares ────────────────────────────────────────────────

  @override
  Future<List<bridge.MobileShare>> listShares() => bridge.listShares();

  @override
  Future<bridge.MobileShare?> getShare(String shareId) =>
      bridge.getShare(shareId: shareId);

  @override
  Future<bridge.MobileShare> createShare({
    required bridge.CreateShareParams params,
  }) => bridge.createShare(params: params);

  @override
  Future<void> bindShare({required bridge.BindShareParams params}) =>
      bridge.bindShare(params: params);

  @override
  Future<void> deleteShare(String shareId) =>
      bridge.deleteShare(shareId: shareId);

  @override
  Future<bridge.MobileShare> setShareVisibility({
    required String shareId,
    required String kind,
    required bool publicWrite,
  }) => bridge.setShareVisibility(
    shareId: shareId,
    kind: kind,
    publicWrite: publicWrite,
  );

  @override
  Future<List<bridge.MobileShareGrant>> listShareGrants(String shareId) =>
      bridge.listShareGrants(shareId: shareId);

  @override
  Future<void> setShareGrant({
    required String shareId,
    required String userId,
    required String mode,
  }) => bridge.setShareGrant(shareId: shareId, userId: userId, mode: mode);

  @override
  Future<void> removeShareGrant({
    required String shareId,
    required String userId,
  }) => bridge.removeShareGrant(shareId: shareId, userId: userId);

  @override
  Future<shares.ShareManifest?> loadShareManifest(String shareId) =>
      bridge.loadShareManifest(shareId: shareId);

  @override
  Future<void> saveShareManifest({
    required String shareId,
    required shares.ShareManifest manifest,
  }) => bridge.saveShareManifest(shareId: shareId, manifest: manifest);

  @override
  Future<bridge.MobileShare> acceptShareOffer({
    required shares.ShareOffer offer,
  }) => bridge.acceptShareOffer(offer: offer);

  @override
  Future<shares.ShareManifest> snapshotShare({
    required String shareId,
    required String root,
  }) => bridge.snapshotShare(shareId: shareId, root: root);

  @override
  Future<void> requestPeerShares(String peerId) =>
      bridge.requestPeerShares(peerId: peerId);

  @override
  Future<void> requestShareManifest({
    required String peerId,
    required String shareId,
  }) => bridge.requestShareManifest(peerId: peerId, shareId: shareId);

  @override
  Future<void> readvertiseShare({
    required String shareId,
    required String peerId,
  }) => bridge.readvertiseShare(shareId: shareId, peerId: peerId);

  @override
  Future<void> publishShareSnapshot({
    required String peerId,
    required PlatformInt64 baseRevision,
    required shares.ShareManifest manifest,
  }) => bridge.publishShareSnapshot(
    peerId: peerId,
    baseRevision: baseRevision,
    manifest: manifest,
  );

  // ── Pure UI helpers (Rust is the single source) ───────────────────────────

  @override
  String? formatTimeHhmmSync({required String dt}) =>
      mobile.formatTimeHhmm(dt: dt);

  @override
  Future<String> formatTimeHhmm({required String dt}) async =>
      mobile.formatTimeHhmm(dt: dt);

  @override
  Future<ReplyRef?> replyDraft({
    String? msgId,
    String? senderNickname,
    String? peerId,
    required String content,
    MobileAttachment? attachment,
  }) async => mobile.replyDraft(
    msgId: msgId,
    senderNickname: senderNickname,
    peerId: peerId,
    content: content,
    attachment: attachment,
  );

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
