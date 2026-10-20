import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

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
    final time =
        msg.sentAt ?? await nodeApi.formatTimeHhmm(dt: msg.createdAt);
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
    required this.unreadCount,
    required this.unreadLabel,
    required this.onJumpToBottom,
    this.onOpenPeerInfo,
    this.onOpenAttachment,
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
  final int unreadCount;
  final String unreadLabel;
  final VoidCallback onJumpToBottom;
  final void Function(String)? onOpenPeerInfo;
  final void Function(ChatMessage)? onOpenAttachment;

  @override
  Widget build(BuildContext context) {
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
  });

  final int count;
  final VoidCallback onCancel;
  final VoidCallback onCopy;

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
  });

  final TextEditingController controller;
  final bool enabled;
  final String hintText;
  final VoidCallback onSend;
  final VoidCallback? onPickFile;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
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
    required this.onOpenPeerInfo,
    this.onPickFile,
    this.onOpenAttachment,
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
  final void Function(String) onOpenPeerInfo;
  final VoidCallback? onPickFile;
  final void Function(ChatMessage)? onOpenAttachment;

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
          unreadCount: widget.unreadCount,
          unreadLabel:
              '${widget.unreadCount} new message${widget.unreadCount > 1 ? 's' : ''}',
          onJumpToBottom: widget.onJumpToBottom,
          onOpenPeerInfo: widget.onOpenPeerInfo,
          onOpenAttachment: widget.onOpenAttachment,
        ),
        MessageInputBar(
          controller: _controller,
          enabled: widget.serviceRunning,
          hintText: widget.serviceRunning ? 'Broadcast...' : 'Offline',
          onSend: _send,
          onPickFile: widget.onPickFile,
        ),
      ],
    );
  }
}

/// A single message bubble: peer name (tappable to open peer info), content,
/// and a timestamp. Double-tap enters selection mode; tapping toggles the
/// bubble's selection.
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
  });
  final ChatMessage message;
  final bool isOwn;
  final bool selected;
  final bool selectionMode;
  final VoidCallback? onDoubleTap;
  final VoidCallback? onTap;
  final void Function(String)? onOpenPeerInfo;
  final VoidCallback? onOpenAttachment;

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
              if (message.content.isNotEmpty)
                Text(message.content, style: TextStyle(fontSize: 14, color: fg)),
              if (message.attachment != null)
                _AttachmentChip(
                  attachment: message.attachment!,
                  color: fg.withAlpha(200),
                  onOpen: (selectionMode || onOpenAttachment == null)
                      ? null
                      : onOpenAttachment,
                ),
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
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
/// tappable `[file] name (size)` summary that opens the stored blob.
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
              color: onOpen != null ? Theme.of(context).colorScheme.primary : color,
            ),
            const SizedBox(width: 6),
            Flexible(
              child: Text(
                '[file] ${attachment.name} (${attachment.size})',
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
