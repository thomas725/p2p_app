import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/attachments.dart';
import 'package:p2p_app_flutter/src/util/reply.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';
import 'package:p2p_app_flutter/src/util/notifications.dart'
    show ConversationFocusGuard, pushFocusedConversation;

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
    this.onOpenPeerInfo,
  });
  final MobileGroup group;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  /// Opens a message sender's Peer Info when their name is tapped — the Flutter
  /// mirror of the TUI's "message click opens the sender's Peer Info" rule.
  /// Optional so widget tests can pump without a host callback; when absent,
  /// sender names are plain (non-interactive) labels.
  final void Function(String)? onOpenPeerInfo;

  @override
  State<GroupChatScreen> createState() => _GroupChatScreenState();
}

class _GroupChatScreenState extends State<GroupChatScreen>
    with MessageChatMixin<GroupChatScreen> {
  final List<ChatMessage> _messages = [];
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;

  /// Incremented per initial history load so a reply arriving after the fetch
  /// started cannot clobber messages received live in the meantime.
  int _historyGeneration = 0;

  /// Frozen quote for the next group message we send; independent of selection.
  ReplyRef? _replyTo;

  void _onReply(int index) async {
    if (index < 0 || index >= _messages.length) return;
    final draft = await replyDraftFor(_messages[index]);
    if (draft != null) setState(() => _replyTo = draft);
  }

  void _cancelReply() => setState(() => _replyTo = null);

  String get _groupId => widget.group.groupId;
  int get _memberCount => widget.group.memberCount.toInt();

  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events.
  void Function(SwarmEventJson)? _previousSink;
  late final ConversationFocusGuard _focusGuard;

  @override
  void initState() {
    super.initState();
    _previousSink = pushEventSink(_handleGroupEvent);
    _focusGuard = pushFocusedConversation('group:${widget.group.groupId}');
    _scrollController.addListener(_onScroll);
    _loadHistory();
  }

  @override
  void dispose() {
    _focusGuard.release();
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
    origin: ChatMessageOrigin(isBroadcast: false, sent: m.sent),
    edited: m.edited,
    msgId: m.msgId,
    replyTo: m.replyTo,
    sentAt: m.sentAt,
    createdAt: m.createdAt,
    senderNickname: m.senderNickname,
    attachment: m.attachment,
    // Group messages are never end-to-end encrypted (multi-recipient).
    encrypted: false,
    // Group messages carry no per-recipient receipt tracking, so they never
    // show a delivery tick.
    confirmed: false,
    // …and the group wire format carries no latency either.
    latency: null,
    reactions: m.reactions,
  );

  Future<void> _loadHistory() async {
    final generation = ++_historyGeneration;
    try {
      final msgs = await nodeApi.loadGroupMessages(
        groupId: _groupId,
        limit: 200,
      );
      if (!mounted || generation != _historyGeneration) return;
      setState(() {
        // Preserve rows appended live while the fetch was in flight.
        final historyIds = msgs.map((m) => m.id).toSet();
        final live = _messages.where((m) => !historyIds.contains(m.id)).toList();
        _messages
          ..clear()
          ..addAll(msgs.map(_toChatMessage))
          ..addAll(live);
        _loading = false;
      });
      scrollToBottom(_scrollController);
    } catch (_) {
      if (!mounted || generation != _historyGeneration) return;
      setState(() => _loading = false);
    }
  }

  /// Serializes reloads: a concurrent [loadGroupMessages] run while one is
  /// already in flight would recompute `added` against stale messages and
  /// double-count unread (or lose messages) — an event burst can fire several
  /// of these back-to-back.
  bool _reloadInFlight = false;

  /// An event arrived while a reload was in flight. Its DB row may postdate the
  /// in-flight snapshot, so we must run one more reload rather than drop it.
  bool _reloadPending = false;

  /// Reload history and reflect only the messages we did not already have, so
  /// live incoming messages accumulate without dropping our own optimistic
  /// copies.
  Future<void> _reloadForIncoming() async {
    if (_reloadInFlight) {
      _reloadPending = true;
      return;
    }
    _reloadInFlight = true;
    try {
      final knownIds = {for (final m in _messages) m.id};
      final msgs = await nodeApi.loadGroupMessages(
        groupId: _groupId,
        limit: 200,
      );
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
      // An event that landed while we were fetching may postdate the snapshot
      // we just applied. Run one more reload instead of dropping it.
      if (_reloadPending) {
        _reloadPending = false;
        if (mounted) unawaited(_reloadForIncoming());
      }
    }
  }

  /// Refresh chips for a live reaction event (US-008) on any matching row.
  Future<void> _applyReaction(String msgId) async {
    await applyReactionEvent(_messages, msgId);
    if (mounted) setState(() {});
  }

  /// Toggle a reaction from a chip tap, applying the canonical returned chips.
  Future<void> _onReact(int index, String emoji) async {
    await toggleReactionAt(_messages, index, emoji);
    if (mounted) setState(() {});
  }

  void _handleGroupEvent(SwarmEventJson event) {
    if (!mounted) return;
    if (event.eventType == 'reaction') {
      // Rust owns the aggregated chips; apply them to any row with the wire id
      // and forward so other open chat views see the same change.
      if (event.msgId != null) {
        unawaited(_applyReaction(event.msgId!));
      }
      _previousSink?.call(event);
      return;
    }
    if (event.eventType != 'group_message') {
      _previousSink?.call(event);
      return;
    }
    if (event.groupId == _groupId && event.content != null) {
      unawaited(_reloadForIncoming());
      // Home only refreshes the group list on this event (member counts stay
      // fresh), so forwarding it is safe and keeps the summary in sync.
      _previousSink?.call(event);
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
    final reply = _replyTo;
    try {
      final msg = await nodeApi.saveOutgoingGroup(
        groupId: _groupId,
        content: text,
        replyTo: reply,
      );
      _controller.clear();
      setState(() {
        _messages.add(_toChatMessage(msg));
        _replyTo = null;
      });
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
    final reply = _replyTo;
    try {
      final msg = await nodeApi.saveOutgoingGroup(
        groupId: _groupId,
        content: '',
        attachmentPath: picked.path,
        attachmentBytes: picked.bytes,
        attachmentName: picked.name,
        replyTo: reply,
      );
      setState(() {
        _messages.add(_toChatMessage(msg));
        _replyTo = null;
      });
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
            onDeleteSelected: () => deleteSelection(_messages, isGroup: true),
            onReply: _onReply,
            onEdit: () => editSelection(_messages, isGroup: true),
            unreadCount: unreadCount,
            unreadLabel: '$unreadCount new',
            onJumpToBottom: () => forceJumpToBottom(_scrollController),
            onOpenPeerInfo: widget.onOpenPeerInfo,
            onOpenAttachment: _openAttachment,
            onReact: _onReact,
          ),
          MessageInputBar(
            controller: _controller,
            enabled: widget.serviceRunning,
            hintText: widget.serviceRunning
                ? 'Message ${widget.group.displayName}...'
                : 'Offline',
            onSend: _send,
            onPickFile: _pickAndSend,
            replyTo: _replyTo,
            onCancelReply: _cancelReply,
          ),
        ],
      ),
    );
  }
}
