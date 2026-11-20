// Hand-written helpers over the FRB-generated chat models.

import '../rust/mobile_node.dart';

/// FRB 2.13 emits data classes without `copyWith`, but a live `receipt` event
/// must flip [ChatMessage.confirmed] without reloading history — so this
/// rebuilds the message with the new flag. Kept in a hand-written file (the
/// generated one is overwritten by every regen); a field change on `ChatMessage`
/// turns this into a compile error, which is the intent (it caught both
/// `replyTo` and `latency` when they were added).
extension ChatMessageExt on ChatMessage {
  /// A copy of this message with [confirmed] set to [value]. Every other field
  /// — including the immutable quote snapshot and the live latency decoration —
  /// is preserved verbatim.
  ChatMessage withConfirmed(bool value) => ChatMessage(
    id: id,
    content: content,
    peerId: peerId,
    isBroadcast: isBroadcast,
    targetPeer: targetPeer,
    sent: sent,
    msgId: msgId,
    replyTo: replyTo,
    sentAt: sentAt,
    createdAt: createdAt,
    senderNickname: senderNickname,
    attachment: attachment,
    encrypted: encrypted,
    confirmed: value,
    edited: edited,
    latency: latency,
  );

  /// A copy with the body replaced and the local-edit marker set (US-005).
  /// Everything else — quote snapshot, attachment, receipts, latency — stays
  /// verbatim, mirroring the TUI's in-place line replacement.
  ChatMessage withEditedBody(String newContent) => ChatMessage(
    id: id,
    content: newContent,
    peerId: peerId,
    isBroadcast: isBroadcast,
    targetPeer: targetPeer,
    sent: sent,
    msgId: msgId,
    replyTo: replyTo,
    sentAt: sentAt,
    createdAt: createdAt,
    senderNickname: senderNickname,
    attachment: attachment,
    encrypted: encrypted,
    confirmed: confirmed,
    edited: true,
    latency: latency,
  );
}
