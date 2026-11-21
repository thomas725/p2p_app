import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/chat_message_ext.dart';
import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';
import 'package:p2p_app_flutter/src/util/blocklist.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';
import 'package:p2p_app_flutter/src/util/notifications.dart'
    show pushFocusedConversation, setFocusedConversation;
import 'package:p2p_app_flutter/src/util/attachments.dart';
import 'package:p2p_app_flutter/src/util/reply.dart';

/// How long an incoming "is typing" indicator stays visible without a refresh.
const _typingTtl = Duration(seconds: 3);
/// Minimum gap between outgoing "started typing" signals in one conversation.
const _typingThrottle = Duration(milliseconds: 700);

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

  /// Live typing state for this conversation (US-009). Incoming signals are
  /// frontend-only and expire locally; nothing is persisted.
  bool _peerTyping = false;
  Timer? _typingTimer;
  DateTime? _lastTypingStart;

  /// Frozen quote for the next DM we send; independent of selection mode.
  ReplyRef? _replyTo;

  void _onReply(int index) {
    if (index < 0 || index >= _messages.length) return;
    final draft = replyDraftFor(_messages[index]);
    if (draft != null) setState(() => _replyTo = draft);
  }

  void _cancelReply() => setState(() => _replyTo = null);

  String get _peerId => widget.peer.peerId;
  String get _label =>
      _peerId.length >= 16 ? _peerId.substring(0, 16) : _peerId;

  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events (e.g. when opened from Peer info).
  void Function(SwarmEventJson)? _previousSink;
  String? _previousFocus;

  @override
  void initState() {
    super.initState();
    _previousSink = pushEventSink(_handleDmEvent);
    _previousFocus = pushFocusedConversation('dm:${widget.peer.peerId}');
    _scrollController.addListener(_onScroll);
    _loadHistory();
  }

  @override
  void dispose() {
    setFocusedConversation(_previousFocus);
    _typingTimer?.cancel();
    if (_lastTypingStart != null) {
      unawaited(_emitTyping(false));
    }
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
      final msgs = await nodeApi.loadDmMessages(peerId: _peerId, limit: 200);
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

  /// Send a transient typing state, throttling repeated "start" signals while
  /// always forwarding an explicit "stop".
  Future<void> _emitTyping(bool isTyping) async {
    if (!widget.serviceRunning) return;
    final now = DateTime.now();
    if (isTyping &&
        _lastTypingStart != null &&
        now.difference(_lastTypingStart!) < _typingThrottle) {
      return;
    }
    _lastTypingStart = isTyping ? now : null;
    try {
      await nodeApi.sendTyping(peerId: _peerId, isTyping: isTyping);
    } catch (e) {
      debugPrint('Typing signal failed: $e');
    }
  }

  void _onComposerChanged(String text) {
    unawaited(_emitTyping(text.trim().isNotEmpty));
  }

  void _applyTypingEvent(SwarmEventJson event) {
    final typing = event.content == 'start';
    _typingTimer?.cancel();
    setState(() => _peerTyping = typing);
    if (typing) {
      _typingTimer = Timer(_typingTtl, () {
        if (mounted) setState(() => _peerTyping = false);
      });
    }
  }

  void _handleDmEvent(SwarmEventJson event) {
    if (!mounted) return;
    if (event.eventType == 'reaction') {
      // The event carries only the wire id; Rust owns the aggregated chips.
      // Apply to this conversation, then forward so the broadcast view (and a
      // group screen underneath) can update any rows with the same id too.
      if (event.msgId != null) {
        unawaited(_applyReaction(event.msgId!));
      }
      _previousSink?.call(event);
      return;
    }
    if (event.eventType == 'typing') {
      if (event.peerId == _peerId) {
        _applyTypingEvent(event);
      } else {
        _previousSink?.call(event);
      }
      return;
    }
    if (event.eventType == 'receipt') {
      // The peer acknowledged one of our DMs (or someone acknowledged a
      // broadcast): tick our own copy, then keep forwarding so the broadcast
      // chat under us sees it too.
      if (event.msgId != null) {
        _markConfirmed(event.msgId!);
      }
      _previousSink?.call(event);
      return;
    }
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

  /// Flip the delivery tick on the message a `receipt` acknowledged. A no-op
  /// when the id belongs to a message this screen doesn't show.
  void _markConfirmed(String msgId) {
    final idx = confirmedMessageIndex(_messages, msgId);
    if (idx < 0 || _messages[idx].confirmed) return;
    setState(() => _messages[idx] = _messages[idx].withConfirmed(true));
  }

  /// Refresh the chips for a live reaction event (US-008) on any row with the
  /// matching wire id. No-op for ids this screen does not display.
  Future<void> _applyReaction(String msgId) async {
    await applyReactionEvent(_messages, msgId);
    if (mounted) setState(() {});
  }

  /// Toggle a reaction from a chip tap, applying the canonical returned chips
  /// to just that row (US-008).
  Future<void> _onReact(int index, String emoji) async {
    await toggleReactionAt(_messages, index, emoji);
    if (mounted) setState(() {});
  }

  Future<void> _persistIncomingDm(SwarmEventJson event) async {
    try {
      final msg = await nodeApi.saveIncomingMessage(
        content: event.content!,
        peerId: event.peerId!,
        isDirect: true,
        nickname: event.nickname,
        latency: event.latency,
        msgId: event.msgId,
        replyTo: event.replyTo,
        encrypted: event.encrypted,
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
    unawaited(_emitTyping(false));
    final reply = _replyTo;
    try {
      final msg = await nodeApi.saveOutgoingDm(
        peerId: _peerId,
        content: text,
        replyTo: reply,
      );
      _controller.clear();
      setState(() {
        _messages.add(msg);
        _replyTo = null;
      });
      forceJumpToBottom(_scrollController);
    } catch (e) {
      debugPrint('DM send failed: $e');
    }
  }

  /// Pick a file and send it as an attachment-only DM (files lane §12).
  /// Native picks send by path; web picks upload the picked bytes.
  Future<void> _pickAndSend() async {
    final picked = await pickFileOverride();
    if (picked == null) return;
    unawaited(_emitTyping(false));
    final reply = _replyTo;
    try {
      final msg = await nodeApi.saveOutgoingDm(
        peerId: _peerId,
        content: '',
        attachmentPath: picked.path,
        attachmentBytes: picked.bytes,
        attachmentName: picked.name,
        replyTo: reply,
      );
      setState(() {
        _messages.add(msg);
        _replyTo = null;
      });
      forceJumpToBottom(_scrollController);
    } catch (e) {
      debugPrint('DM attachment send failed: $e');
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
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(widget.peer.displayName),
            if (_peerTyping)
              const Text(
                'typing…',
                key: Key('peer-typing'),
                style: TextStyle(fontSize: 11, fontStyle: FontStyle.italic),
              ),
          ],
        ),
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
                  loadQuality: nodeApi.getPeerQuality,
                  loadBlocked: peerIsBlocked,
                  toggleBlock: togglePeerBlock,
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
            onDeleteSelected: () => deleteSelection(_messages, isGroup: false),
            onReply: _onReply,
            onEdit: () => editSelection(_messages, isGroup: false),
            unreadCount: unreadCount,
            unreadLabel: '$unreadCount new',
            onJumpToBottom: () => forceJumpToBottom(_scrollController),
            onOpenAttachment: _openAttachment,
            onReact: _onReact,
          ),
          MessageInputBar(
            controller: _controller,
            enabled: widget.serviceRunning,
            hintText: widget.serviceRunning ? 'Message $_label...' : 'Offline',
            onSend: _send,
            onPickFile: _pickAndSend,
            onChanged: _onComposerChanged,
            replyTo: _replyTo,
            onCancelReply: _cancelReply,
          ),
        ],
      ),
    );
  }
}
