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

// PlatformInt64Util is frb's int<->i64 adapter; same internal import the
// generated bindings and the other transports use.
// ignore_for_file: invalid_use_of_internal_member
import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64;

import 'package:p2p_app_flutter/src/data/frb_node_api.dart';
import 'package:p2p_app_flutter/src/rust/api.dart'
    show BindShareParams, CreateShareParams, MobileShare, MobileShareGrant;
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

  /// Broadcasts newer than the persisted viewed marker, for the "N new" badge.
  /// Rust owns the watermark so every transport agrees after a restart.
  Future<int> getBroadcastUnreadCount();

  /// Persist that every broadcast currently in history has been viewed.
  Future<void> markBroadcastViewed();

  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  });

  /// Locally hide one sent message (broadcast/DM when [isGroup] is false,
  /// group message when it is true). This is a tombstone on this device only:
  /// no retraction is sent and other peers keep their copy.
  Future<void> deleteMessage({required int messageId, required bool isGroup});

  /// Locally replace the body of one sent message (US-005). Device-only: no
  /// edit is sent, so peers keep the original body.
  Future<void> editMessage({
    required int messageId,
    required bool isGroup,
    required String content,
  });

  /// Send a transient DM typing state (`true` = started, `false` = stopped)
  /// over the dedicated fire-and-forget lane (US-009).
  Future<void> sendTyping({required String peerId, required bool isTyping});

  /// Toggle this install's reaction on a message and return the fresh
  /// aggregated chips (US-008). The canonical summaries come from Rust on every
  /// transport; Dart never assembles counts or "mine" flags itself.
  Future<List<ReactionSummary>> toggleReaction({
    required String msgId,
    required String emoji,
  });

  /// Load the aggregated reaction chips for one message (US-008).
  Future<List<ReactionSummary>> getReactions({required String msgId});

  /// Validate a custom reaction token against the canonical Rust rule before
  /// sending it through [toggleReaction] (empty/overlong rejected, max 32 chars).
  Future<bool> validateEmoji({required String emoji});

  /// Default quick-reaction tokens, owned by Rust and shared with the TUI.
  Future<List<String>> quickReactions();

  /// Send a broadcast. Attachments arrive either as a server-local path (FFI)
  /// or as raw [attachmentBytes] + [attachmentName] that the remote transport
  /// uploads to the node first (files lane §12). [replyTo] is an immutable
  /// quote snapshot captured when the user picked Reply (US-006).
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  });

  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
    ReplyRef? replyTo,
  });

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
  });

  Future<void> dialPeer({required String address});

  // --- Persisted network configuration (bootstrap + relay multiaddrs) ---

  /// Every persisted network peer, oldest first. The node reads this list once
  /// at start, so edits apply on the next start.
  Future<List<NetworkPeer>> listNetworkPeers();

  /// Persist one peer multiaddr (`kind` = `bootstrap` or `relay`). The address
  /// is validated in Rust; relay entries need a `/p2p/<peer-id>` suffix.
  Future<NetworkPeer> addNetworkPeer({
    required String kind,
    required String multiaddr,
  });

  /// Remove the persisted entry with [id]; true when a row was deleted.
  Future<bool> removeNetworkPeer({required int id});

  /// Whether LAN-only mode is enabled (US-051). Disables DHT, relays and
  /// internet bootstrap; mDNS and direct dials still work.
  Future<bool> getLanOnly();

  /// Persist the LAN-only toggle (US-051). Applies on next node start.
  Future<void> setLanOnly({required bool enabled});

  /// Resolved theme (US-025/US-026), including the available accent swatches.
  Future<AppTheme> getTheme();

  /// Persist the dark/light + accent choice. An unknown accent is rejected in
  /// Rust, so the palette cannot drift between frontends.
  Future<void> setTheme({required bool dark, required String accentName});

  /// Persisted notification delivery channels (US-027/US-028). Text and policy
  /// are Rust-owned; the frontends only deliver the payload on the event.
  Future<NotificationSettings> getNotificationSettings();

  /// Persist both notification channels in one round trip.
  Future<void> setNotificationSettings({
    required bool sound,
    required bool desktop,
  });

  // --- Peer blocklist / mute (US-054) ---

  /// Whether [peerId] is muted. Blocked peers are ignored at the swarm
  /// boundary: their traffic is dropped and they are never dialed.
  Future<bool> isPeerBlocked({required String peerId});

  /// Block one peer; true when it was not already blocked.
  Future<bool> blockPeer({required String peerId});

  /// Unblock one peer; true when a row was removed.
  Future<bool> unblockPeer({required String peerId});

  /// Every blocked peer id, oldest first.
  Future<List<String>> listBlockedPeers();

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
    ReplyRef? replyTo,
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

  // --- Calls (signaling + Rust-owned media transport) ---

  Future<String> startCall({required String peerId});

  Future<void> sendCallSignal({
    required String peerId,
    required String callId,
    required String kind,
  });

  /// Open the media session for an accepted call; peer audio then arrives as a
  /// `call_media` event with decoded PCM in `audio` (calls lane, US-044).
  Future<void> startCallMedia({required String callId, required String peerId});

  Future<void> stopCallMedia();

  /// Encode and send one 16 kHz mono PCM block (i16 samples) on the session.
  Future<void> sendCallMedia({required String callId, required List<int> pcm});

  /// Mute or unmute outgoing audio; inbound audio keeps playing.
  Future<void> setCallMediaMute({required bool muted});

  // --- Versioned file shares / folder sync ---

  Future<List<MobileShare>> listShares();

  Future<MobileShare?> getShare(String shareId);

  Future<MobileShare> createShare({required CreateShareParams params});

  Future<void> bindShare({required BindShareParams params});

  Future<void> deleteShare(String shareId);

  /// Change visibility/public-write on a share owned by the local user.
  /// Rust resolves the caller and refuses peer-owned shares.
  Future<MobileShare> setShareVisibility({
    required String shareId,
    required String kind,
    required bool publicWrite,
  });

  Future<List<MobileShareGrant>> listShareGrants(String shareId);

  Future<void> setShareGrant({
    required String shareId,
    required String userId,
    required String mode,
  });

  Future<void> removeShareGrant({
    required String shareId,
    required String userId,
  });

  Future<ShareManifest?> loadShareManifest(String shareId);

  /// Native-only raw store write. Web clients must use
  /// [publishShareSnapshot], which enforces ACLs and `base_revision`; the web
  /// transport rejects this method.
  Future<void> saveShareManifest({
    required String shareId,
    required ShareManifest manifest,
  });

  Future<MobileShare> acceptShareOffer({required ShareOffer offer});

  /// Native-only: the web transport rejects a server-local [root].
  Future<ShareManifest> snapshotShare({
    required String shareId,
    required String root,
  });

  Future<void> requestPeerShares(String peerId);

  Future<void> requestShareManifest({
    required String peerId,
    required String shareId,
  });

  /// Re-advertise a locally known share using Rust's canonical offer builder.
  /// Rust rebuilds the offer, so callers pass only ids and cannot leak the
  /// local unknown-owner marker onto the wire.
  Future<void> readvertiseShare({
    required String shareId,
    required String peerId,
  });

  Future<void> publishShareSnapshot({
    required String peerId,
    required PlatformInt64 baseRevision,
    required ShareManifest manifest,
  });

  // --- Pure UI helpers (executed in Rust on every transport) ---

  Future<String> formatTimeHhmm({required String dt});

  /// Synchronous variant of [formatTimeHhmm] for hot render paths. The
  /// in-process transport always answers (its Rust call is sync); the remote
  /// transport returns its memoized Rust answer, or `null` on a cold miss —
  /// callers must then fall back to [formatTimeHhmm] (results are pure, so a
  /// memo hit can never diverge from Rust).
  String? formatTimeHhmmSync({required String dt});

  /// Freeze a chat row into the immutable reply snapshot sent on the wire.
  ///
  /// Rust owns the sender-label, attachment-chip and bounded-preview rules, so
  /// both transports call the same helper and a quote captured in the browser
  /// matches one captured in process.
  Future<ReplyRef?> replyDraft({
    String? msgId,
    String? senderNickname,
    String? peerId,
    required String content,
    MobileAttachment? attachment,
  });

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
