import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
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

class _DmChatScreenState extends State<DmChatScreen>
    with MessageChatMixin<DmChatScreen> {
  final List<ChatMessage> _messages = [];
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;

  String get _peerId => widget.peer.peerId;
  String get _label =>
      _peerId.length >= 16 ? _peerId.substring(0, 16) : _peerId;

  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events (e.g. when opened from Peer info).
  void Function(SwarmEventJson)? _previousSink;

  @override
  void initState() {
    super.initState();
    _previousSink = pushEventSink(_handleDmEvent);
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

  Future<void> _loadHistory() async {
    try {
      final msgs = await loadDmMessages(peerId: _peerId, limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(msgs);
        _loading = false;
      });
      scrollToBottom(_scrollController);
    } catch (_) {
      setState(() => _loading = false);
    }
  }

  void _handleDmEvent(SwarmEventJson event) {
    if (!mounted) return;
    final isOwnDm = event.eventType == 'dm' && event.peerId == _peerId;
    if (!isOwnDm) {
      // Not this peer's DM (or another event type): pass it to the sink we
      // replaced so the view underneath keeps persisting/processing it. DMs
      // from any other peer would otherwise be dropped forever.
      _previousSink?.call(event);
      return;
    }
    if (event.content != null) {
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
        if (!atBottom) unreadCount++;
      });
      scrollToBottom(_scrollController);
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
      forceJumpToBottom(_scrollController);
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
          MessageChatBody(
            messages: _messages,
            scrollController: _scrollController,
            loading: _loading,
            emptyText: 'No messages with this peer.',
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
          ),
          MessageInputBar(
            controller: _controller,
            enabled: widget.serviceRunning,
            hintText: widget.serviceRunning ? 'Message $_label...' : 'Offline',
            onSend: _send,
          ),
        ],
      ),
    );
  }
}
