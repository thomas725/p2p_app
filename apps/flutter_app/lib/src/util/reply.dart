// Hand-written helpers for the US-006 reply/quote UI.
//
// The Rust core owns the wire snapshot (`ReplyRef`) and the canonical quoted
// header; Dart only builds the snapshot when a user picks Reply and renders the
// frozen strings later. Keeping both rules here guarantees the three chat
// surfaces (broadcast, DM, group) capture and display the same thing.

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';

/// Maximum number of characters kept in a quoted preview (same as Rust's
/// `reply::PREVIEW_CHARS`).
const int replyPreviewChars = 80;

/// Sender label of [message], matching the name `MessageBubble` prints above
/// the body: `Me` for our own rows, otherwise the received nickname or the
/// short peer id.
String replySenderLabel(ChatMessage message) {
  if (message.peerId == null) return 'Me';
  final nickname = message.senderNickname;
  if (nickname != null) return nickname;
  final peerId = message.peerId!;
  return peerId.length >= 12 ? peerId.substring(0, 12) : peerId;
}

/// Collapse whitespace to single spaces and bound a preview to
/// [replyPreviewChars] code points plus an ellipsis. Mirrors Rust's
/// `reply::preview_line` (it uses `char`s, hence `runes` here, not `length`).
String replyPreviewLine(String value) {
  final flat = value
      .split(RegExp(r'\s+'))
      .where((part) => part.isNotEmpty)
      .join(' ');
  final runes = flat.runes.toList();
  if (runes.length <= replyPreviewChars) return flat;
  return '${String.fromCharCodes(runes.take(replyPreviewChars))}…';
}

/// The canonical quoted header Rust prepends to a transcript line.
String quotedHeader(ReplyRef reply) => '↳ ${reply.sender}: ${reply.preview}';

/// Text a quoted preview is built from, mirroring Rust's
/// `files::render_message_body`: an attachment-only row shows the canonical
/// Rust chip (`MobileAttachment.chip`), and a mixed row appends it after the
/// body. No frontend assembles the chip or formats bytes itself.
String replyPreviewBody(ChatMessage message) {
  final attachment = message.attachment;
  if (attachment == null) return message.content;
  final text = message.content.trim();
  if (text.isEmpty) return attachment.chip;
  return '$text ${attachment.chip}';
}

/// Freeze [message] into an immutable reply draft. Returns null when the row
/// has no stable wire id, so it can never be answered (the Reply action is
/// disabled for those rows in every chat surface).
ReplyRef? replyDraftFor(ChatMessage message) {
  final msgId = message.msgId;
  if (msgId == null || msgId.isEmpty) return null;
  return ReplyRef(
    msgId: msgId,
    sender: replySenderLabel(message),
    preview: replyPreviewLine(replyPreviewBody(message)),
  );
}
