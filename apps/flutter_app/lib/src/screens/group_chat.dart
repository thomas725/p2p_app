import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
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

class _GroupChatScreenState extends State<GroupChatScreen> {
  final List<ChatMessage> _messages = [];
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;
  bool _atBottom = true;
  int _unreadCount = 0;
  bool _selectionMode = false;
  final Set<int> _selectedIndices = {};

  String get _groupId => widget.group.groupId;
  int get _memberCount => widget.group.memberCount.toInt();

  @override
  void initState() {
    super.initState();
    setEventSink(_handleGroupEvent);
    _scrollController.addListener(_onScroll);
    _loadHistory();
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _controller.dispose();
    _scrollController.dispose();
    setEventSink(null);
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    // Use Rust's is_at_bottom logic for consistency
    final atBottom = isAtBottom(
      scrollOffset: pos.pixels.round(),
      total: pos.maxScrollExtent.round(),
      visible: 80,
    );
    if (atBottom != _atBottom) {
      setState(() {
        _atBottom = atBottom;
        if (_atBottom) _unreadCount = 0;
      });
    }
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
  );

  Future<void> _loadHistory() async {
    try {
      final msgs = await loadGroupMessages(groupId: _groupId, limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs.map(_toChatMessage));
        _loading = false;
      });
      _scrollToBottom();
    } catch (_) {
      setState(() => _loading = false);
    }
  }

  /// Reload history and reflect only the messages we did not already have, so
  /// live incoming messages accumulate without dropping our own optimistic
  /// copies.
  Future<void> _reloadForIncoming() async {
    try {
      final knownIds = {for (final m in _messages) m.id};
      final msgs = await loadGroupMessages(groupId: _groupId, limit: 200);
      if (!mounted) return;
      final added = msgs.where((m) => !knownIds.contains(m.id)).toList();
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs.map(_toChatMessage));
        if (added.isNotEmpty && !_atBottom) _unreadCount += added.length;
      });
      _scrollToBottom();
    } catch (_) {}
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients && _atBottom) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  void _forceScrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
        setState(() {
          _atBottom = true;
          _unreadCount = 0;
        });
      }
    });
  }

  void _onBubbleDoubleTap(int index) {
    setState(() {
      _selectionMode = true;
      _selectedIndices.add(index);
    });
  }

  void _onBubbleTap(int index) {
    if (!_selectionMode) return;
    setState(() {
      if (_selectedIndices.contains(index)) {
        _selectedIndices.remove(index);
        if (_selectedIndices.isEmpty) _selectionMode = false;
      } else {
        _selectedIndices.add(index);
      }
    });
  }

  void _cancelSelection() {
    setState(() {
      _selectionMode = false;
      _selectedIndices.clear();
    });
  }

  void _copySelected() {
    final buf = StringBuffer();
    final sorted = _selectedIndices.toList()..sort();
    for (var i = 0; i < sorted.length; i++) {
      final msg = _messages[sorted[i]];
      final isOwn = msg.peerId == null;
      final peerName = isOwn
          ? 'Me'
          : (msg.senderNickname ??
                (msg.peerId!.length >= 12
                    ? msg.peerId!.substring(0, 12)
                    : msg.peerId!));
      final time = msg.sentAt ?? formatTimeHhmm(dt: msg.createdAt);
      buf.writeln(peerName);
      buf.writeln(msg.content);
      buf.writeln(time);
      if (i < sorted.length - 1) buf.writeln();
    }
    Clipboard.setData(ClipboardData(text: buf.toString()));
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            '${_selectedIndices.length} message${_selectedIndices.length > 1 ? 's' : ''} copied',
          ),
          duration: const Duration(seconds: 2),
        ),
      );
    }
    _cancelSelection();
  }

  void _handleGroupEvent(SwarmEventJson event) {
    if (!mounted || event.eventType != 'group_message') return;
    if (event.groupId == _groupId && event.content != null) {
      unawaited(_reloadForIncoming());
    }
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingGroup(groupId: _groupId, content: text);
      _controller.clear();
      setState(() => _messages.add(_toChatMessage(msg)));
      _forceScrollToBottom();
    } catch (e) {
      debugPrint('Group send failed: $e');
    }
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
          Expanded(
            child: Stack(
              alignment: Alignment.bottomCenter,
              children: [
                _loading
                    ? const Center(child: CircularProgressIndicator())
                    : _messages.isEmpty
                    ? Center(
                        child: Text(
                          'No messages in this group yet.',
                          style: TextStyle(color: Colors.grey[500]),
                        ),
                      )
                    : SelectionArea(
                        child: SingleChildScrollView(
                          controller: _scrollController,
                          padding: const EdgeInsets.symmetric(
                            horizontal: 12,
                            vertical: 8,
                          ),
                          child: Column(
                            children: [
                              for (int i = 0; i < _messages.length; i++)
                                MessageBubble(
                                  message: _messages[i],
                                  isOwn: _messages[i].peerId == null,
                                  selected: _selectedIndices.contains(i),
                                  onDoubleTap: () => _onBubbleDoubleTap(i),
                                  onTap: () => _onBubbleTap(i),
                                ),
                            ],
                          ),
                        ),
                      ),
                if (_selectionMode)
                  Positioned(
                    bottom: 8,
                    child: Card(
                      elevation: 4,
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            onPressed: _cancelSelection,
                            icon: const Icon(Icons.close),
                            tooltip: 'Cancel',
                          ),
                          Padding(
                            padding: const EdgeInsets.symmetric(horizontal: 4),
                            child: Text(
                              '${_selectedIndices.length} selected',
                              style: const TextStyle(fontSize: 13),
                            ),
                          ),
                          IconButton(
                            onPressed: _selectedIndices.isEmpty
                                ? null
                                : _copySelected,
                            icon: const Icon(Icons.copy),
                            tooltip: 'Copy',
                          ),
                        ],
                      ),
                    ),
                  )
                else if (_unreadCount > 0)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 8),
                    child: FilledButton.icon(
                      onPressed: _forceScrollToBottom,
                      icon: const Icon(Icons.arrow_downward, size: 16),
                      label: Text(
                        '$_unreadCount new',
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
                            ? 'Message ${widget.group.displayName}...'
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
      ),
    );
  }
}