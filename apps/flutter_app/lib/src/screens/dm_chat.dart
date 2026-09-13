import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

/// Full-screen DM chat for a single peer (history + live incoming DMs while
/// open), with selection/copy and an unread "jump to latest" banner.
class DmChatScreen extends StatefulWidget {
  const DmChatScreen({
    super.key,
    required this.peer,
    required this.serviceRunning,
    this.onNavigate,
    this.currentTab = 1,
  });
  final MobilePeerRecord peer;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  State<DmChatScreen> createState() => _DmChatScreenState();
}

class _DmChatScreenState extends State<DmChatScreen> {
  final List<ChatMessage> _messages = [];
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;
  bool _atBottom = true;
  int _unreadCount = 0;
  bool _selectionMode = false;
  final Set<int> _selectedIndices = {};

  String get _peerId => widget.peer.peerId;
  String get _label =>
      _peerId.length >= 16 ? _peerId.substring(0, 16) : _peerId;

  @override
  void initState() {
    super.initState();
    setEventSink(_handleDmEvent);
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

  Future<void> _loadHistory() async {
    try {
      final msgs = await loadDmMessages(peerId: _peerId, limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs);
        _loading = false;
      });
      _scrollToBottom();
    } catch (_) {
      setState(() => _loading = false);
    }
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

  void _handleDmEvent(SwarmEventJson event) {
    if (!mounted || event.eventType != 'dm') return;
    if (event.peerId == _peerId && event.content != null) {
      unawaited(_persistIncomingDm(event));
    }
  }

  Future<void> _persistIncomingDm(SwarmEventJson event) async {
    try {
      final msg = await saveIncomingMessage(
        content: event.content!,
        peerId: event.peerId!,
        isDirect: true,
        nickname: event.nickname,
      );
      if (!mounted) return;
      setState(() {
        _messages.add(msg);
        if (!_atBottom) _unreadCount++;
      });
      _scrollToBottom();
    } catch (e) {
      debugPrint('Failed to persist incoming DM: $e');
    }
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingDm(peerId: _peerId, content: text);
      _controller.clear();
      setState(() => _messages.add(msg));
      _forceScrollToBottom();
    } catch (e) {
      debugPrint('DM send failed: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.peer.displayName),
        actions: [
          IconButton(
            icon: const Icon(Icons.info_outline),
            tooltip: 'Peer info',
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute(
                builder: (_) => PeerInfoScreen(
                  peer: widget.peer,
                  serviceRunning: widget.serviceRunning,
                  onNavigate: widget.onNavigate,
                  currentTab: widget.currentTab,
                ),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.only(right: 16),
            child: Center(
              child: Text(
                (_peerId.length >= 8 ? _peerId.substring(0, 8) : _peerId),
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
                          'No messages with this peer.',
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
                            ? 'Message $_label...'
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
