// Hand-written helpers for the US-006 reply/quote UI.
//
// Rust owns the wire snapshot (`ReplyRef`), the canonical quoted header and all
// the rules that freeze a draft (sender label, attachment body and bounded
// preview). Dart asks the node for a draft via [replyDraftFor] when the user
// picks Reply and renders the frozen strings later. The two remaining helpers
// here are display-only mirrors used by live bubbles and the copy transcript.

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';

/// Canonical sender label for a message row: the Rust-resolved nickname (our
/// own rows carry the installed self nickname; others the announced/petname
/// label) or the short peer id, with `Me` only as a last-resort own-row label.
/// Shared by the bubble header and the copy-to-clipboard transcript, and
/// mirrored by `mobile_api::reply_sender_label` for the frozen quote.
///
/// [isOwn] defaults to the wire rule (`peerId == null`) but can be supplied by
/// widgets that track ownership separately (e.g. `MessageBubble`).
String messageSenderLabel(ChatMessage message, {bool? isOwn}) {
  final own = isOwn ?? message.peerId == null;
  final nickname = message.senderNickname;
  if (nickname != null) return nickname;
  if (own) return 'Me';
  final peerId = message.peerId;
  if (peerId == null) return 'Unknown';
  return peerId.length >= 12 ? peerId.substring(0, 12) : peerId;
}

/// The canonical quoted header Rust prepends to a transcript line.
String quotedHeader(ReplyRef reply) => '↳ ${reply.sender}: ${reply.preview}';

/// Freeze [message] into an immutable reply draft. The construction rules live
/// in Rust and run through [NodeApi] on both transports, so no frontend can
/// drift from the TUI's quote capture. Returns null when the row has no stable
/// wire id (the Reply action is disabled for those rows in every chat surface).
Future<ReplyRef?> replyDraftFor(ChatMessage message) => nodeApi.replyDraft(
      msgId: message.msgId,
      senderNickname: message.senderNickname,
      peerId: message.peerId,
      content: message.content,
      attachment: message.attachment,
    );
