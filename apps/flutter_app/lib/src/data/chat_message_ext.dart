// Hand-written helpers over the FRB-generated chat models.

import '../rust/mobile_node.dart';

/// FRB 2.13 emits data classes without `copyWith`, but a live `receipt` event
/// must flip [ChatMessage.confirmed] without reloading history — so this
/// rebuilds the message with the new flag. Kept in a hand-written file (the
/// generated one is overwritten by every regen); a field change on `ChatMessage`
/// turns this into a compile error, which is the intent.
extension ChatMessageExt on ChatMessage {
  /// A copy of this message with [confirmed] set to [value].
  ChatMessage withConfirmed(bool value) => ChatMessage(
    id: id,
    content: content,
    peerId: peerId,
    isBroadcast: isBroadcast,
    targetPeer: targetPeer,
    sent: sent,
    msgId: msgId,
    sentAt: sentAt,
    createdAt: createdAt,
    senderNickname: senderNickname,
    attachment: attachment,
    encrypted: encrypted,
    confirmed: value,
  );
}
