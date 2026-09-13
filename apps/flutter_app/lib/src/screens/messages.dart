import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// Broadcast chat tab (peer-to-peer public messages), with selection/copy and
/// an unread "jump to latest" banner.
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
        Expanded(
          child: Stack(
            alignment: Alignment.bottomCenter,
            children: [
              widget.messages.isEmpty
                  ? Center(
                      child: Text(
                        widget.serviceRunning
                            ? 'No messages yet. Send one!'
                            : 'Start the service to chat.',
                        style: TextStyle(color: Colors.grey[500]),
                      ),
                    )
                  : SelectionArea(
                      child: SingleChildScrollView(
                        controller: widget.scrollController,
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 8,
                        ),
                        child: Column(
                          children: [
                            for (int i = 0; i < widget.messages.length; i++)
                              MessageBubble(
                                message: widget.messages[i],
                                isOwn: widget.messages[i].peerId == null,
                                selected: widget.selectedIndices.contains(i),
                                selectionMode: widget.selectionMode,
                                onDoubleTap: () => widget.onBubbleDoubleTap(i),
                                onTap: () => widget.onBubbleTap(i),
                                onOpenPeerInfo: widget.onOpenPeerInfo,
                              ),
                          ],
                        ),
                      ),
                    ),
              if (widget.selectionMode)
                Positioned(
                  bottom: 8,
                  child: Card(
                    elevation: 4,
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        IconButton(
                          onPressed: widget.onCancelSelection,
                          icon: const Icon(Icons.close),
                          tooltip: 'Cancel',
                        ),
                        Padding(
                          padding: const EdgeInsets.symmetric(horizontal: 4),
                          child: Text(
                            '${widget.selectedIndices.length} selected',
                            style: const TextStyle(fontSize: 13),
                          ),
                        ),
                        IconButton(
                          onPressed: widget.selectedIndices.isEmpty
                              ? null
                              : widget.onCopySelected,
                          icon: const Icon(Icons.copy),
                          tooltip: 'Copy',
                        ),
                      ],
                    ),
                  ),
                )
              else if (widget.unreadCount > 0)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: FilledButton.icon(
                    onPressed: widget.onJumpToBottom,
                    icon: const Icon(Icons.arrow_downward, size: 16),
                    label: Text(
                      '${widget.unreadCount} new message${widget.unreadCount > 1 ? 's' : ''}',
                      style: const TextStyle(fontSize: 12),
                    ),
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
        ),
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    decoration: InputDecoration(
                      hintText: widget.serviceRunning
                          ? 'Broadcast...'
                          : 'Offline',
                      border: const OutlineInputBorder(),
                      isDense: true,
                      enabled: widget.serviceRunning,
                    ),
                    onSubmitted: (_) => _send(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filled(
                  onPressed: widget.serviceRunning ? _send : null,
                  icon: const Icon(Icons.send),
                ),
              ],
            ),
          ),
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
  });
  final ChatMessage message;
  final bool isOwn;
  final bool selected;
  final bool selectionMode;
  final VoidCallback? onDoubleTap;
  final VoidCallback? onTap;
  final void Function(String)? onOpenPeerInfo;

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
              Text(message.content, style: TextStyle(fontSize: 14, color: fg)),
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(
                  message.sentAt ?? formatTimeHhmm(dt: message.createdAt),
                  style: TextStyle(fontSize: 10, color: fg.withAlpha(130)),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
