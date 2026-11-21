import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/data/chat_message_ext.dart';
import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reactions.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/util/reply.dart';

// ────────────────────────────────────────────────────────────────────────────
// Shared chat-list behavior (at-bottom tracking, unread counting, selection)
// ────────────────────────────────────────────────────────────────────────────

/// Return true when the scroll position is at (or past) the bottom, using the
/// same Rust `is_at_bottom` definition every chat view shares.
Future<bool> chatScrollAtBottom(ScrollPosition pos) => nodeApi.isAtBottom(
  scrollOffset: pos.pixels.round(),
  total: pos.maxScrollExtent.round(),
  visible: 80,
);

/// Selection + at-bottom + unread state shared by the full-screen DM and group
/// chat tabs. The broadcast tab (_HomeScreenState) owns equivalent state so it
/// can survive IndexedStack tab switches and does not use this mixin.
mixin MessageChatMixin<T extends StatefulWidget> on State<T> {
  bool atBottom = true;
  int unreadCount = 0;
  bool selectionMode = false;
  final Set<int> selectedIndices = {};

  Future<void> onListScroll(ScrollPosition pos) async {
    final reachedBottom = await chatScrollAtBottom(pos);
    if (!mounted || reachedBottom == atBottom) return;
    setState(() {
      atBottom = reachedBottom;
      if (atBottom) unreadCount = 0;
    });
  }

  void onBubbleDoubleTap(int index) {
    setState(() {
      selectionMode = true;
      selectedIndices.add(index);
    });
  }

  void onBubbleTap(int index) {
    if (!selectionMode) return;
    setState(() {
      if (selectedIndices.contains(index)) {
        selectedIndices.remove(index);
        if (selectedIndices.isEmpty) selectionMode = false;
      } else {
        selectedIndices.add(index);
      }
    });
  }

  void cancelSelection() {
    setState(() {
      selectionMode = false;
      selectedIndices.clear();
    });
  }

  /// Follow the tail on the next frame when the user is at the bottom.
  void scrollToBottom(ScrollController controller) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (controller.hasClients && atBottom) {
        controller.jumpTo(controller.position.maxScrollExtent);
      }
    });
  }

  /// Jump to the bottom unconditionally, marking everything read.
  void forceJumpToBottom(ScrollController controller) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (controller.hasClients) {
        controller.jumpTo(controller.position.maxScrollExtent);
        setState(() {
          atBottom = true;
          unreadCount = 0;
        });
      }
    });
  }

  /// Build the clipboard transcript and show the copied snackbar; resolves
  /// with the number of messages copied. Side effects: clipboard + snackbar.
  Future<int> copySelection(List<ChatMessage> messages) {
    return _copyMessages(context, messages, selectedIndices);
  }

  /// Delete the selected outgoing rows locally, then drop the same ids from the
  /// screen's in-memory history. [messages] is the screen-owned mutable list.
  Future<void> deleteSelection(
    List<ChatMessage> messages, {
    required bool isGroup,
  }) async {
    final deleted = await deleteSelectedMessages(
      context,
      messages,
      selectedIndices,
      isGroup: isGroup,
    );
    if (!mounted) return;
    setState(() {
      messages.removeWhere((m) => deleted.contains(m.id));
      selectionMode = false;
      selectedIndices.clear();
    });
  }

  /// Replace the body of the single selected sent message locally, then update
  /// the screen-owned list so the new text and edit marker appear immediately.
  Future<void> editSelection(
    List<ChatMessage> messages, {
    required bool isGroup,
  }) async {
    final edited = await editSelectedMessage(
      context,
      messages,
      selectedIndices,
      isGroup: isGroup,
    );
    if (!mounted || edited == null) return;
    setState(() {
      final idx = messages.indexWhere((m) => m.id == edited.id);
      if (idx >= 0) messages[idx] = edited;
      selectionMode = false;
      selectedIndices.clear();
    });
  }
}

/// Copy a set of selected messages to the clipboard and confirm with a
/// snackbar. Shared by the broadcast tab (which owns its selection state) and
/// the DM/group screens.
Future<int> copySelectedMessages(
  BuildContext context,
  List<ChatMessage> messages,
  Set<int> selectedIndices,
) {
  return _copyMessages(context, messages, selectedIndices);
}

/// Locally delete the selected messages we sent and return their DB ids so the
/// calling screen can drop the same rows from its in-memory history.
///
/// Only rows with [ChatMessage.sent] are eligible: incoming messages stay on
/// this device. The confirmation makes the local-only scope explicit — no
/// retraction is sent, so other peers keep their copies.
Future<List<int>> deleteSelectedMessages(
  BuildContext context,
  List<ChatMessage> messages,
  Set<int> selectedIndices, {
  required bool isGroup,
}) async {
  final sorted = selectedIndices.toList()..sort();
  final own = [
    for (final i in sorted)
      if (i >= 0 && i < messages.length && messages[i].sent) messages[i],
  ];
  if (own.isEmpty) return const <int>[];

  // Capture the messenger before the async confirmation. The dialog itself is
  // opened synchronously, so no BuildContext crosses an await.
  final messenger = ScaffoldMessenger.of(context);
  final count = own.length;
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (dialogContext) {
      final colors = Theme.of(dialogContext).colorScheme;
      return AlertDialog(
        title: Text(
          count == 1 ? 'Delete sent message?' : 'Delete $count sent messages?',
        ),
        content: const Text(
          'This hides the message only on this device. Other people keep '
          'their copies; no retraction is sent.',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(backgroundColor: colors.error),
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('Delete'),
          ),
        ],
      );
    },
  );
  if (confirmed != true) return const <int>[];

  final deleted = <int>[];
  var failed = 0;
  for (final msg in own) {
    try {
      await nodeApi.deleteMessage(messageId: msg.id, isGroup: isGroup);
      deleted.add(msg.id);
    } catch (_) {
      failed++;
    }
  }

  final noun = deleted.length == 1 ? 'message' : 'messages';
  final summary = 'Deleted ${deleted.length} sent $noun on this device';
  messenger.showSnackBar(
    SnackBar(
      content: Text(failed == 0 ? summary : '$summary; $failed failed'),
      duration: const Duration(seconds: 3),
    ),
  );
  return deleted;
}

/// Locally replace the body of the one selected message we sent, returning the
/// rebuilt row so the calling screen can update its in-memory history. Only a
/// single sent row is eligible. The dialog makes the device-only scope plain:
/// no edit is sent, so other peers keep the original body.
Future<ChatMessage?> editSelectedMessage(
  BuildContext context,
  List<ChatMessage> messages,
  Set<int> selectedIndices, {
  required bool isGroup,
}) async {
  if (selectedIndices.length != 1) return null;
  final index = selectedIndices.first;
  if (index < 0 || index >= messages.length) return null;
  final original = messages[index];
  if (!original.sent) return null;

  final messenger = ScaffoldMessenger.of(context);
  final controller = TextEditingController(text: original.content);
  final saved = await showDialog<bool>(
    context: context,
    builder: (dialogContext) {
      return AlertDialog(
        title: const Text('Edit sent message?'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'This changes the message only on this device. Other people '
              'keep the original; no edit is sent.',
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('edit-message-field'),
              controller: controller,
              autofocus: true,
              maxLines: 4,
              minLines: 1,
              decoration: const InputDecoration(
                labelText: 'Message',
                border: OutlineInputBorder(),
              ),
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialogContext, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialogContext, true),
            child: const Text('Save'),
          ),
        ],
      );
    },
  );
  if (saved != true) return null;
  final content = controller.text.trim();
  if (content.isEmpty) {
    messenger.showSnackBar(
      const SnackBar(content: Text('Message body cannot be empty')),
    );
    return null;
  }
  try {
    await nodeApi.editMessage(
      messageId: original.id,
      isGroup: isGroup,
      content: content,
    );
    return original.withEditedBody(content);
  } catch (_) {
    messenger.showSnackBar(
      const SnackBar(content: Text('Could not edit message')),
    );
    return null;
  }
}

Future<int> _copyMessages(
  BuildContext context,
  List<ChatMessage> messages,
  Set<int> selectedIndices,
) async {
  // Capture before the awaits below (context must not cross async gaps).
  final messenger = ScaffoldMessenger.of(context);
  final sorted = selectedIndices.toList()..sort();
  final buf = StringBuffer();
  for (var i = 0; i < sorted.length; i++) {
    final msg = messages[sorted[i]];
    final isOwn = msg.peerId == null;
    final peerName = isOwn
        ? 'Me'
        : (msg.senderNickname ??
              (msg.peerId!.length >= 12
                  ? msg.peerId!.substring(0, 12)
                  : msg.peerId!));
    final time = msg.sentAt ?? await nodeApi.formatTimeHhmm(dt: msg.createdAt);
    buf.writeln(peerName);
    buf.writeln(msg.content);
    buf.writeln(time);
    if (i < sorted.length - 1) buf.writeln();
  }
  Clipboard.setData(ClipboardData(text: buf.toString()));
  messenger.showSnackBar(
    SnackBar(
      content: Text(
        '${sorted.length} message${sorted.length > 1 ? 's' : ''} copied',
      ),
      duration: const Duration(seconds: 2),
    ),
  );
  return sorted.length;
}

// ────────────────────────────────────────────────────────────────────────────
// Shared chat UI: scrollable body (with overlays) + input bar
// ────────────────────────────────────────────────────────────────────────────

/// Index of the outgoing message a `receipt` acknowledges, or -1 when the id
/// is unknown here. Incoming rows persist wire ids too, so an echoed id must
/// never flip the delivery tick of an inbound message (mirrors Rust's
/// `mark_confirmed`).
int confirmedMessageIndex(List<ChatMessage> messages, String msgId) =>
    messages.indexWhere((m) => m.sent && m.msgId == msgId);

/// The small palette offered when a user adds a reaction. Kept as a plain
/// constant in the UI: the canonical *counts* and `mine` flags always come from
/// Rust, so a frontend palette is purely an affordance and can never diverge
/// from the stored data.
const List<String> quickReactions = ['👍', '❤️', '😂', '🎉', '😮', '😢'];

/// Toggle [emoji] on the message at [index] through Rust and replace just that
/// row with the returned canonical chips. The owning [messages] list is mutated
/// in place so the caller can `setState`; a reply that arrives after the row was
/// removed is ignored.
Future<void> toggleReactionAt(
  List<ChatMessage> messages,
  int index,
  String emoji,
) async {
  final msgId = messages[index].msgId;
  if (msgId == null || msgId.isEmpty) return;
  final summaries = await nodeApi.toggleReaction(msgId: msgId, emoji: emoji);
  if (index >= 0 && index < messages.length && messages[index].msgId == msgId) {
    messages[index] = messages[index].withReactions(summaries);
  }
}

/// Apply fresh chips from a live `reaction` event (US-008) to every row with the
/// matching wire id. Mirrors the TUI's `msg_id`-keyed in-memory chip map: the
/// event carries only the id, and Rust is the single source of the summary.
Future<void> applyReactionEvent(List<ChatMessage> messages, String msgId) async {
  final summaries = await nodeApi.getReactions(msgId: msgId);
  for (var i = 0; i < messages.length; i++) {
    if (messages[i].msgId == msgId) {
      messages[i] = messages[i].withReactions(summaries);
    }
  }
}

/// The scrollable message list with its overlays — tap-selection toolbar or the
/// unread "jump to latest" banner — shared by the broadcast tab, DM chat, and
/// group chat.
class MessageChatBody extends StatelessWidget {
  const MessageChatBody({
    super.key,
    required this.messages,
    required this.scrollController,
    required this.loading,
    required this.emptyText,
    required this.selectionMode,
    required this.selectedIndices,
    required this.onBubbleDoubleTap,
    required this.onBubbleTap,
    required this.onCancelSelection,
    required this.onCopySelected,
    required this.onDeleteSelected,
    this.onReply,
    this.onEdit,
    required this.unreadCount,
    required this.unreadLabel,
    required this.onJumpToBottom,
    this.onOpenPeerInfo,
    this.onOpenAttachment,
    this.onReact,
  });

  final List<ChatMessage> messages;
  final ScrollController scrollController;
  final bool loading;
  final String emptyText;
  final bool selectionMode;
  final Set<int> selectedIndices;
  final void Function(int) onBubbleDoubleTap;
  final void Function(int) onBubbleTap;
  final VoidCallback onCancelSelection;
  final VoidCallback onCopySelected;
  final VoidCallback onDeleteSelected;

  /// Called with the selected message's index when Reply is tapped. The action
  /// is hidden unless exactly one message with a wire id is selected.
  final void Function(int)? onReply;

  /// Called when Edit is tapped. Hidden unless exactly one sent message is
  /// selected (a local-only edit); the owning screen already holds that list.
  final VoidCallback? onEdit;
  final int unreadCount;
  final String unreadLabel;
  final VoidCallback onJumpToBottom;
  final void Function(String)? onOpenPeerInfo;
  final void Function(ChatMessage)? onOpenAttachment;

  /// Called with `(message index, emoji)` when a reaction chip is tapped. The
  /// owning screen applies the change through Rust and replaces just that row.
  final void Function(int, String)? onReact;

  @override
  Widget build(BuildContext context) {
    final canDelete = selectedIndices.any(
      (i) => i >= 0 && i < messages.length && messages[i].sent,
    );
    final bool singleSelected =
        selectedIndices.length == 1 &&
        selectedIndices.first >= 0 &&
        selectedIndices.first < messages.length;
    final int? replyIndex =
        singleSelected &&
            onReply != null &&
            messages[selectedIndices.first].msgId?.isNotEmpty == true
        ? selectedIndices.first
        : null;
    final int? editIndex =
        singleSelected && onEdit != null && messages[selectedIndices.first].sent
        ? selectedIndices.first
        : null;
    return Expanded(
      child: Stack(
        alignment: Alignment.bottomCenter,
        children: [
          if (loading)
            const Center(child: CircularProgressIndicator())
          else if (messages.isEmpty)
            Center(
              child: Text(emptyText, style: TextStyle(color: Colors.grey[500])),
            )
          else
            SelectionArea(
              child: SingleChildScrollView(
                controller: scrollController,
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                child: Column(
                  children: [
                    for (int i = 0; i < messages.length; i++)
                      MessageBubble(
                        message: messages[i],
                        isOwn: messages[i].peerId == null,
                        selected: selectedIndices.contains(i),
                        selectionMode: selectionMode,
                        onDoubleTap: () => onBubbleDoubleTap(i),
                        onTap: () => onBubbleTap(i),
                        onOpenPeerInfo: onOpenPeerInfo,
                        onOpenAttachment: onOpenAttachment == null
                            ? null
                            : () => onOpenAttachment!(messages[i]),
                        onReact: onReact == null
                            ? null
                            : (emoji) => onReact!(i, emoji),
                      ),
                  ],
                ),
              ),
            ),
          if (selectionMode)
            SelectionToolbar(
              count: selectedIndices.length,
              onCancel: onCancelSelection,
              onCopy: onCopySelected,
              canDelete: canDelete,
              onDelete: onDeleteSelected,
              onReply: replyIndex == null ? null : () => onReply!(replyIndex),
              onEdit: editIndex == null ? null : onEdit,
            )
          else if (unreadCount > 0)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: FilledButton.icon(
                onPressed: onJumpToBottom,
                icon: const Icon(Icons.arrow_downward, size: 16),
                label: Text(unreadLabel, style: const TextStyle(fontSize: 12)),
                style: FilledButton.styleFrom(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 6,
                  ),
                  visualDensity: VisualDensity.compact,
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Bottom overlays shown while selection mode is active.
class SelectionToolbar extends StatelessWidget {
  const SelectionToolbar({
    super.key,
    required this.count,
    required this.onCancel,
    required this.onCopy,
    required this.canDelete,
    required this.onDelete,
    this.onReply,
    this.onEdit,
  });

  final int count;
  final VoidCallback onCancel;
  final VoidCallback onCopy;
  final bool canDelete;
  final VoidCallback onDelete;

  /// Null hides the Reply action (multi-select or a row without a wire id).
  final VoidCallback? onReply;

  /// Null hides the local Edit action (multi-select or an incoming row).
  final VoidCallback? onEdit;

  @override
  Widget build(BuildContext context) {
    return Positioned(
      bottom: 8,
      child: Card(
        elevation: 4,
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              key: const Key('cancel-selection'),
              onPressed: onCancel,
              icon: const Icon(Icons.close),
              tooltip: 'Cancel',
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 4),
              child: Text(
                '$count selected',
                style: const TextStyle(fontSize: 13),
              ),
            ),
            IconButton(
              onPressed: count == 0 ? null : onCopy,
              icon: const Icon(Icons.copy),
              tooltip: 'Copy',
            ),
            IconButton(
              onPressed: canDelete ? onDelete : null,
              icon: const Icon(Icons.delete_outline),
              tooltip: 'Delete sent messages on this device',
            ),
            if (onReply != null)
              IconButton(
                key: const Key('reply-action'),
                onPressed: onReply,
                icon: const Icon(Icons.reply),
                tooltip: 'Reply',
              ),
            if (onEdit != null)
              IconButton(
                key: const Key('edit-action'),
                onPressed: onEdit,
                icon: const Icon(Icons.edit_outlined),
                tooltip: 'Edit sent message on this device',
              ),
          ],
        ),
      ),
    );
  }
}

/// The bottom message-input row shared by every chat view.
class MessageInputBar extends StatelessWidget {
  const MessageInputBar({
    super.key,
    required this.controller,
    required this.enabled,
    required this.hintText,
    required this.onSend,
    this.onPickFile,
    this.onChanged,
    this.replyTo,
    this.onCancelReply,
  });

  final TextEditingController controller;
  final bool enabled;
  final String hintText;
  final VoidCallback onSend;
  final VoidCallback? onPickFile;

  /// Notified on every composer edit; DM screens use it to send transient
  /// typing signals (US-009).
  final ValueChanged<String>? onChanged;

  /// Frozen quote shown above the field until the user sends or cancels it.
  final ReplyRef? replyTo;
  final VoidCallback? onCancelReply;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            if (replyTo != null)
              Container(
                key: const Key('reply-draft'),
                margin: const EdgeInsets.only(bottom: 6),
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: cs.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(8),
                  border: Border.all(color: cs.outlineVariant),
                ),
                child: Row(
                  children: [
                    Icon(Icons.reply, size: 16, color: cs.onSurfaceVariant),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        quotedHeader(replyTo!),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(
                          fontSize: 12,
                          color: cs.onSurfaceVariant,
                        ),
                      ),
                    ),
                    IconButton(
                      key: const Key('reply-cancel'),
                      onPressed: onCancelReply,
                      icon: const Icon(Icons.close, size: 16),
                      tooltip: 'Cancel reply',
                      visualDensity: VisualDensity.compact,
                    ),
                  ],
                ),
              ),
            Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: controller,
                    decoration: InputDecoration(
                      hintText: hintText,
                      border: const OutlineInputBorder(),
                      isDense: true,
                      enabled: enabled,
                    ),
                    onSubmitted: (_) => onSend(),
                    onChanged: onChanged,
                  ),
                ),
                const SizedBox(width: 8),
                if (onPickFile != null)
                  IconButton(
                    onPressed: enabled ? onPickFile : null,
                    icon: const Icon(Icons.attach_file),
                    tooltip: 'Attach a file',
                  ),
                IconButton.filled(
                  onPressed: enabled ? onSend : null,
                  icon: const Icon(Icons.send),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ────────────────────────────────────────────────────────────────────────────
// Broadcast tab
// ────────────────────────────────────────────────────────────────────────────

/// Broadcast chat tab (peer-to-peer public messages), with selection/copy and
/// an unread "jump to latest" banner. Lives in the home IndexedStack, so the
/// shell owns the message/selection state and provides it through the widget.
class BroadcastChat extends StatefulWidget {
  const BroadcastChat({
    super.key,
    required this.messages,
    required this.scrollController,
    required this.serviceRunning,
    required this.onSend,
    required this.unreadCount,
    required this.onJumpToBottom,
    required this.selectionMode,
    required this.selectedIndices,
    required this.onBubbleDoubleTap,
    required this.onBubbleTap,
    required this.onCancelSelection,
    required this.onCopySelected,
    required this.onDeleteSelected,
    required this.onOpenPeerInfo,
    this.onReply,
    this.onEdit,
    this.replyTo,
    this.onCancelReply,
    this.onPickFile,
    this.onOpenAttachment,
    this.onReact,
  });

  final List<ChatMessage> messages;
  final ScrollController scrollController;
  final bool serviceRunning;
  final Future<void> Function(String) onSend;
  final int unreadCount;
  final VoidCallback onJumpToBottom;
  final bool selectionMode;
  final Set<int> selectedIndices;
  final void Function(int) onBubbleDoubleTap;
  final void Function(int) onBubbleTap;
  final VoidCallback onCancelSelection;
  final VoidCallback onCopySelected;
  final VoidCallback onDeleteSelected;
  final void Function(String) onOpenPeerInfo;
  final void Function(int)? onReply;
  final VoidCallback? onEdit;
  final ReplyRef? replyTo;
  final VoidCallback? onCancelReply;
  final VoidCallback? onPickFile;
  final void Function(ChatMessage)? onOpenAttachment;
  final void Function(int, String)? onReact;

  @override
  State<BroadcastChat> createState() => _BroadcastChatState();
}

class _BroadcastChatState extends State<BroadcastChat> {
  final _controller = TextEditingController();

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _send() {
    final text = _controller.text.trim();
    if (text.isEmpty) return;
    widget.onSend(text);
    _controller.clear();
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          color: Theme.of(context).colorScheme.primaryContainer,
          child: Row(
            children: [
              const Icon(Icons.public, size: 20),
              const SizedBox(width: 8),
              Text(
                'Broadcast',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                  color: Theme.of(context).colorScheme.onPrimaryContainer,
                ),
              ),
              const Spacer(),
              if (!widget.serviceRunning)
                const Text(
                  'Offline',
                  style: TextStyle(fontSize: 12, color: Colors.orange),
                ),
            ],
          ),
        ),
        MessageChatBody(
          messages: widget.messages,
          scrollController: widget.scrollController,
          loading: false,
          emptyText: widget.serviceRunning
              ? 'No messages yet. Send one!'
              : 'Start the service to chat.',
          selectionMode: widget.selectionMode,
          selectedIndices: widget.selectedIndices,
          onBubbleDoubleTap: widget.onBubbleDoubleTap,
          onBubbleTap: widget.onBubbleTap,
          onCancelSelection: widget.onCancelSelection,
          onCopySelected: widget.onCopySelected,
          onDeleteSelected: widget.onDeleteSelected,
          onReply: widget.onReply,
          onEdit: widget.onEdit,
          unreadCount: widget.unreadCount,
          unreadLabel:
              '${widget.unreadCount} new message${widget.unreadCount > 1 ? 's' : ''}',
          onJumpToBottom: widget.onJumpToBottom,
          onOpenPeerInfo: widget.onOpenPeerInfo,
          onOpenAttachment: widget.onOpenAttachment,
          onReact: widget.onReact,
        ),
        MessageInputBar(
          controller: _controller,
          enabled: widget.serviceRunning,
          hintText: widget.serviceRunning ? 'Broadcast...' : 'Offline',
          onSend: _send,
          onPickFile: widget.onPickFile,
          replyTo: widget.replyTo,
          onCancelReply: widget.onCancelReply,
        ),
      ],
    );
  }
}

/// A single message bubble: peer name (tappable to open peer info), content,
/// a live round-trip latency (incoming messages only), and a timestamp.
/// Double-tap enters selection mode; tapping toggles the bubble's selection.
class MessageBubble extends StatelessWidget {
  const MessageBubble({
    super.key,
    required this.message,
    required this.isOwn,
    this.selected = false,
    this.selectionMode = false,
    this.onDoubleTap,
    this.onTap,
    this.onOpenPeerInfo,
    this.onOpenAttachment,
    this.onReact,
  });
  final ChatMessage message;
  final bool isOwn;
  final bool selected;
  final bool selectionMode;
  final VoidCallback? onDoubleTap;
  final VoidCallback? onTap;
  final void Function(String)? onOpenPeerInfo;
  final VoidCallback? onOpenAttachment;
  final void Function(String)? onReact;

  /// Show the small quick-reaction palette and forward the chosen token to the
  /// owning screen. Kept here so every bubble gets the same affordance without
  /// the three chat screens duplicating a menu.
  Future<void> _pickReaction(BuildContext context, void Function(String) react) async {
    final emoji = await showMenu<String>(
      context: context,
      position: const RelativeRect.fromLTRB(0, 0, 0, 0),
      items: [
        for (final token in quickReactions)
          PopupMenuItem<String>(value: token, child: Text(token, style: const TextStyle(fontSize: 18))),
      ],
    );
    if (emoji != null) react(emoji);
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final bg = isOwn ? cs.primaryContainer : cs.surfaceContainerHighest;
    final fg = isOwn ? cs.onPrimaryContainer : cs.onSurface;

    final senderName = isOwn
        ? 'Me'
        : (message.senderNickname ??
              (message.peerId != null
                  ? (message.peerId!.length >= 12
                        ? message.peerId!.substring(0, 12)
                        : message.peerId!)
                  : 'Unknown'));
    // Tapping the sender name opens the peer info page (unless selecting).
    final onOpenPeerInfo =
        (!isOwn && !selectionMode && this.onOpenPeerInfo != null)
        ? () => this.onOpenPeerInfo!(message.peerId!)
        : null;

    return Align(
      alignment: isOwn ? Alignment.centerRight : Alignment.centerLeft,
      child: GestureDetector(
        onDoubleTap: onDoubleTap,
        onTap: onTap,
        child: Container(
          margin: const EdgeInsets.symmetric(vertical: 3),
          constraints: BoxConstraints(
            maxWidth: MediaQuery.of(context).size.width * 0.75,
          ),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: selected ? cs.primary.withAlpha(40) : bg,
            borderRadius: BorderRadius.circular(12),
            border: selected ? Border.all(color: cs.primary, width: 2) : null,
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              if (!isOwn)
                GestureDetector(
                  onTap: onOpenPeerInfo,
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: 2),
                    child: Text(
                      senderName,
                      style: TextStyle(
                        fontSize: 11,
                        fontWeight: FontWeight.bold,
                        color: onOpenPeerInfo != null
                            ? cs.primary
                            : fg.withAlpha(180),
                        decoration: onOpenPeerInfo != null
                            ? TextDecoration.underline
                            : TextDecoration.none,
                      ),
                    ),
                  ),
                ),
              if (message.replyTo != null)
                Padding(
                  key: const Key('quote-header'),
                  padding: const EdgeInsets.only(bottom: 4),
                  child: Text(
                    quotedHeader(message.replyTo!),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 11, color: fg.withAlpha(150)),
                  ),
                ),
              if (message.content.isNotEmpty)
                Text(
                  message.content,
                  style: TextStyle(fontSize: 14, color: fg),
                ),
              if (message.attachment != null)
                _AttachmentChip(
                  attachment: message.attachment!,
                  color: fg.withAlpha(200),
                  onOpen: (selectionMode || onOpenAttachment == null)
                      ? null
                      : onOpenAttachment,
                ),
              if ((message.reactions.isNotEmpty || onReact != null) &&
                  !selectionMode &&
                  message.msgId != null)
                Padding(
                  key: const Key('reaction-chips'),
                  padding: const EdgeInsets.only(top: 4),
                  child: Wrap(
                    spacing: 4,
                    runSpacing: 2,
                    crossAxisAlignment: WrapCrossAlignment.center,
                    children: [
                      for (final reaction in message.reactions)
                        _ReactionChip(
                          reaction: reaction,
                          foreground: fg,
                          onTap: onReact == null
                              ? null
                              : () => onReact!(reaction.emoji),
                        ),
                      if (onReact != null)
                        InkWell(
                          key: const Key('add-reaction'),
                          onTap: () => _pickReaction(context, onReact!),
                          borderRadius: BorderRadius.circular(10),
                          child: Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 6,
                              vertical: 2,
                            ),
                            decoration: BoxDecoration(
                              color: fg.withAlpha(18),
                              borderRadius: BorderRadius.circular(10),
                              border: Border.all(color: fg.withAlpha(70)),
                            ),
                            child: Icon(
                              Icons.add_reaction_outlined,
                              size: 14,
                              color: fg.withAlpha(160),
                              semanticLabel: 'Add reaction',
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    // Lock marker for a DM that arrived sealed end to end
                    // (US-042). It is in the metadata row so the body stays
                    // identical to the TUI's `[e2e] `-prefixed line.
                    if (message.encrypted) ...[
                      Icon(
                        Icons.lock,
                        size: 12,
                        color: cs.primary,
                        semanticLabel: 'End-to-end encrypted',
                      ),
                      const SizedBox(width: 6),
                    ],
                    // Local-edit marker (US-005). The TUI appends the same
                    // canonical text to the transcript line.
                    if (message.edited) ...[
                      const Text(
                        '(edited)',
                        key: Key('edited-marker'),
                        style: TextStyle(fontSize: 11),
                      ),
                      const SizedBox(width: 6),
                    ],
                    // Round-trip latency of a live incoming message, printed
                    // before the time just as the TUI prefixes it onto the line.
                    if (message.latency != null) ...[
                      Text(
                        message.latency!,
                        style: TextStyle(
                          fontSize: 11,
                          color: fg.withAlpha(130),
                        ),
                      ),
                      const SizedBox(width: 6),
                    ],
                    _MessageTime(
                      sentAt: message.sentAt,
                      createdAt: message.createdAt,
                      color: fg.withAlpha(130),
                    ),
                    // Delivery tick: a recipient acknowledged this outgoing
                    // message (mirrors the TUI's confirmed prefix).
                    if (isOwn && message.confirmed) ...[
                      const SizedBox(width: 4),
                      Icon(
                        Icons.done_all,
                        size: 12,
                        color: fg.withAlpha(150),
                        semanticLabel: 'Delivered',
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The file-attachment chip (files lane §12) shown inside a bubble: a
/// tappable `[file] name (size)` summary that opens the stored blob. The size
/// text is Rust's [MobileAttachment.sizeLabel], so both frontends format bytes
/// identically.
class _AttachmentChip extends StatelessWidget {
  const _AttachmentChip({
    required this.attachment,
    required this.color,
    this.onOpen,
  });

  final MobileAttachment attachment;
  final Color color;
  final VoidCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onOpen,
      child: Container(
        margin: const EdgeInsets.only(top: 4),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          color: color.withAlpha(25),
          borderRadius: BorderRadius.circular(6),
          border: Border.all(color: color.withAlpha(90)),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.insert_drive_file,
              size: 14,
              color: onOpen != null
                  ? Theme.of(context).colorScheme.primary
                  : color,
            ),
            const SizedBox(width: 6),
            Flexible(
              child: Text(
                '[file] ${attachment.name} (${attachment.sizeLabel})',
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 12,
                  color: onOpen != null
                      ? Theme.of(context).colorScheme.primary
                      : color,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// One aggregated reaction chip (US-008). The emoji/count/`mine` fields come
/// straight from Rust's [ReactionSummary]; the chip is tappable to toggle this
/// install's reaction and is visually marked when we are among the reactors.
class _ReactionChip extends StatelessWidget {
  const _ReactionChip({
    required this.reaction,
    required this.foreground,
    this.onTap,
  });

  final ReactionSummary reaction;
  final Color foreground;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final active = reaction.mine;
    final color = active ? cs.primary : foreground.withAlpha(150);
    return Semantics(
      label: '${reaction.emoji} ${reaction.count}',
      button: onTap != null,
      child: InkWell(
        key: Key('reaction-${reaction.emoji}'),
        onTap: onTap,
        borderRadius: BorderRadius.circular(10),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 7, vertical: 2),
          decoration: BoxDecoration(
            color: active ? cs.primary.withAlpha(30) : foreground.withAlpha(18),
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: active ? cs.primary : foreground.withAlpha(70),
            ),
          ),
          child: Text(
            '${reaction.emoji} ${reaction.count}',
            style: TextStyle(
              fontSize: 12,
              fontWeight: active ? FontWeight.bold : FontWeight.normal,
              color: color,
            ),
          ),
        ),
      ),
    );
  }
}

/// Bubble timestamp: `sentAt` when the message carries one, else Rust's
/// canonical `format_time_hhmm`. The in-process transport resolves
/// synchronously; the remote (web) transport memoizes Rust's answers, so only
/// a cold miss shows the raw timestamp for a frame before the formatted one.
class _MessageTime extends StatefulWidget {
  const _MessageTime({
    required this.sentAt,
    required this.createdAt,
    required this.color,
  });

  final String? sentAt;
  final String createdAt;
  final Color color;

  @override
  State<_MessageTime> createState() => _MessageTimeState();
}

class _MessageTimeState extends State<_MessageTime> {
  Future<String>? _future;
  String? _resolved;

  @override
  void initState() {
    super.initState();
    _resolve();
  }

  @override
  void didUpdateWidget(covariant _MessageTime oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.createdAt != widget.createdAt ||
        oldWidget.sentAt != widget.sentAt) {
      _future = null;
      _resolved = null;
      _resolve();
    }
  }

  void _resolve() {
    final sentAt = widget.sentAt;
    if (sentAt != null) {
      _resolved = sentAt;
      return;
    }
    final sync = nodeApi.formatTimeHhmmSync(dt: widget.createdAt);
    if (sync != null) {
      _resolved = sync;
      return;
    }
    _future ??= nodeApi.formatTimeHhmm(dt: widget.createdAt);
  }

  @override
  Widget build(BuildContext context) {
    final resolved = _resolved;
    final style = TextStyle(fontSize: 10, color: widget.color);
    if (resolved != null) return Text(resolved, style: style);
    return FutureBuilder<String>(
      future: _future,
      builder: (context, snapshot) => Text(
        // Rust's raw timestamp until its formatted answer lands.
        snapshot.data ?? widget.createdAt,
        style: style,
      ),
    );
  }
}
