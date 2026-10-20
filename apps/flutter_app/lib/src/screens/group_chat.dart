import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/attachments.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

/// Full-screen group chat (history + live incoming group messages while open),
/// with selection/copy and an unread "jump to latest" banner. Incoming messages
/// are persisted by the Rust swarm handler, so the DB stays the source of
/// truth and we reload history on each `group_message` event for this group.
class GroupChatScreen extends StatefulWidget {
  const GroupChatScreen({
    super.key,
    required this.group,
    required this.serviceRunning,
    this.onNavigate,
    this.currentTab = 2,
  });
  final MobileGroup group;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  State<GroupChatScreen> createState() => _GroupChatScreenState();
}

class _GroupChatScreenState extends State<GroupChatScreen>
    with MessageChatMixin<GroupChatScreen> {
  final List<ChatMessage> _messages = [];
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;

  String get _groupId => widget.group.groupId;
  int get _memberCount => widget.group.memberCount.toInt();

  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events.
  void Function(SwarmEventJson)? _previousSink;

  @override
  void initState() {
    super.initState();
    _previousSink = pushEventSink(_handleGroupEvent);
    _scrollController.addListener(_onScroll);
    _loadHistory();
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _controller.dispose();
    _scrollController.dispose();
    setEventSink(_previousSink);
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    onListScroll(_scrollController.position);
  }

  /// Adapter so group messages render through the shared [MessageBubble].
  ChatMessage _toChatMessage(MobileGroupMessage m) => ChatMessage(
    id: m.id,
    content: m.content,
    peerId: m.peerId,
    isBroadcast: false,
    sent: m.sent,
    msgId: m.msgId,
    sentAt: m.sentAt,
    createdAt: m.createdAt,
    senderNickname: m.senderNickname,
    attachment: m.attachment,
    // Group messages carry no per-recipient receipt tracking, so they never
    // show a delivery tick.
    confirmed: false,
  );

  Future<void> _loadHistory() async {
    try {
      final msgs = await nodeApi.loadGroupMessages(groupId: _groupId, limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs.map(_toChatMessage));
        _loading = false;
      });
      scrollToBottom(_scrollController);
    } catch (_) {
      setState(() => _loading = false);
    }
  }

  /// Serializes reloads: a concurrent [loadGroupMessages] run while one is
  /// already in flight would recompute `added` against stale messages and
  /// double-count unread (or lose messages) — an event burst can fire several
  /// of these back-to-back.
  bool _reloadInFlight = false;

  /// Reload history and reflect only the messages we did not already have, so
  /// live incoming messages accumulate without dropping our own optimistic
  /// copies.
  Future<void> _reloadForIncoming() async {
    if (_reloadInFlight) return;
    _reloadInFlight = true;
    try {
      final knownIds = {for (final m in _messages) m.id};
      final msgs = await nodeApi.loadGroupMessages(groupId: _groupId, limit: 200);
      if (!mounted) return;
      final added = msgs.where((m) => !knownIds.contains(m.id)).toList();
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs.map(_toChatMessage));
        if (added.isNotEmpty && !atBottom) unreadCount += added.length;
      });
      scrollToBottom(_scrollController);
    } catch (_) {
    } finally {
      _reloadInFlight = false;
    }
  }

  void _handleGroupEvent(SwarmEventJson event) {
    if (!mounted) return;
    if (event.eventType != 'group_message') {
      _previousSink?.call(event);
      return;
    }
    if (event.groupId == _groupId && event.content != null) {
      unawaited(_reloadForIncoming());
    } else {
      // A message for a different group (or a status-only event): let the
      // view underneath refresh group summaries, which keeps member counts
      // fresh while this screen is open.
      _previousSink?.call(event);
    }
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;
    try {
      final msg = await nodeApi.saveOutgoingGroup(groupId: _groupId, content: text);
      _controller.clear();
      setState(() => _messages.add(_toChatMessage(msg)));
      forceJumpToBottom(_scrollController);
    } catch (e) {
      debugPrint('Group send failed: $e');
    }
  }

  /// Pick a file and send it as an attachment-only group message (files lane
  /// §12). Native picks send by path; web picks upload the picked bytes.
  Future<void> _pickAndSend() async {
    final picked = await pickFileOverride();
    if (picked == null) return;
    try {
      final msg = await nodeApi.saveOutgoingGroup(
        groupId: _groupId,
        content: '',
        attachmentPath: picked.path,
        attachmentBytes: picked.bytes,
        attachmentName: picked.name,
      );
      setState(() => _messages.add(_toChatMessage(msg)));
      forceJumpToBottom(_scrollController);
    } catch (e) {
      debugPrint('Group attachment send failed: $e');
    }
  }

  /// Resolve and open a received attachment's stored blob.
  Future<void> _openAttachment(ChatMessage msg) async {
    final error = await openAttachedFile(msg);
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..clearSnackBars()
      ..showSnackBar(
        SnackBar(
          content: Text(error ?? 'Opening ${msg.attachment?.name}'),
          duration: const Duration(seconds: 2),
        ),
      );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(widget.group.displayName),
            Text(
              _memberCount == 1 ? '1 member' : '$_memberCount members',
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ],
        ),
        actions: [
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Center(
              child: Text(
                (_groupId.length >= 8 ? _groupId.substring(0, 8) : _groupId),
                style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
              ),
            ),
          ),
        ],
      ),
      bottomNavigationBar: MainTabNavBar(
        currentIndex: widget.currentTab,
        onSelect: (i) {
          Navigator.pop(context);
          widget.onNavigate?.call(i);
        },
      ),
      body: Column(
        children: [
          MessageChatBody(
            messages: _messages,
            scrollController: _scrollController,
            loading: _loading,
            emptyText: 'No messages in this group yet.',
            selectionMode: selectionMode,
            selectedIndices: selectedIndices,
            onBubbleDoubleTap: onBubbleDoubleTap,
            onBubbleTap: onBubbleTap,
            onCancelSelection: cancelSelection,
            onCopySelected: () {
              copySelection(_messages);
              cancelSelection();
            },
            unreadCount: unreadCount,
            unreadLabel: '$unreadCount new',
            onJumpToBottom: () => forceJumpToBottom(_scrollController),
            onOpenAttachment: _openAttachment,
          ),
          MessageInputBar(
            controller: _controller,
            enabled: widget.serviceRunning,
            hintText: widget.serviceRunning
                ? 'Message ${widget.group.displayName}...'
                : 'Offline',
            onSend: _send,
            onPickFile: _pickAndSend,
          ),
        ],
      ),
    );
  }
}
