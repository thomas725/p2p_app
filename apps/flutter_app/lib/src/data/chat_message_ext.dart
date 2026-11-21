// Hand-written helpers over the FRB-generated chat models.

import '../rust/mobile_node.dart';
import '../rust/reactions.dart';

/// Keep the pre-refactor property names for channel and direction: FRB now
/// generates a nested [ChatMessageOrigin], but call sites still read the two
/// flags directly (and the web JSON remains flat).
extension ChatMessageOriginX on ChatMessage {
  bool get isBroadcast => origin.isBroadcast;
  bool get sent => origin.sent;
}

/// FRB 2.13 emits data classes without `copyWith`, but live events must flip a
/// flag or swap a field without reloading history — so we rebuild the message.
/// Kept in a hand-written file (the generated one is overwritten by every
/// regen); a field change on `ChatMessage` turns the single [_copy] call into a
/// compile error, which is the intent (it caught `replyTo` and `latency` when
/// they were added).
extension ChatMessageExt on ChatMessage {
  /// Rebuild this row, overriding only the supplied fields. Centralising the
  /// constructor here means the full field list lives in exactly one place.
  /// `??` is safe for the nullable overrides: an explicit `false` is not null,
  /// so `confirmed: false` overrides correctly.
  ChatMessage _copy({
    String? content,
    bool? confirmed,
    bool? edited,
    List<ReactionSummary>? reactions,
  }) => ChatMessage(
    id: id,
    content: content ?? this.content,
    peerId: peerId,
    origin: origin,
    targetPeer: targetPeer,
    msgId: msgId,
    replyTo: replyTo,
    sentAt: sentAt,
    createdAt: createdAt,
    senderNickname: senderNickname,
    attachment: attachment,
    encrypted: encrypted,
    confirmed: confirmed ?? this.confirmed,
    edited: edited ?? this.edited,
    latency: latency,
    reactions: reactions ?? this.reactions,
  );

  /// A copy with [confirmed] set to [value]. Every other field — including the
  /// immutable quote snapshot and live latency — is preserved verbatim.
  ChatMessage withConfirmed(bool value) => _copy(confirmed: value);

  /// A copy with the body replaced and the local-edit marker set (US-005).
  ChatMessage withEditedBody(String newContent) =>
      _copy(content: newContent, edited: true);

  /// A copy with only the reaction chips replaced (US-008).
  ChatMessage withReactions(List<ReactionSummary> next) =>
      _copy(reactions: next);
}
