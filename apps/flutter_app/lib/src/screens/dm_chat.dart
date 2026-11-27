import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/chat_message_ext.dart';
import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/reply.dart';
import 'package:p2p_app_flutter/src/screens/messages.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/location_share.dart';
import 'package:p2p_app_flutter/src/screens/location_track.dart';
import 'package:p2p_app_flutter/src/screens/peer_info.dart';
import 'package:p2p_app_flutter/src/util/blocklist.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';
import 'package:p2p_app_flutter/src/util/location.dart' show LocationAdapter;
import 'package:p2p_app_flutter/src/util/location_device.dart'
    show createLocationAdapter;
import 'package:p2p_app_flutter/src/util/location_session.dart'
    show LocationSessionController, LocationShareConfig;
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
    this.onOpenPeerInfo,
    this.locationAdapterFactory,
  });
  final MobilePeerRecord peer;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  /// Opens a sender's Peer Info when a message's sender name is tapped — the
  /// Flutter mirror of the TUI's "message click opens the sender's Peer Info"
  /// rule. Optional so widget tests can pump without FFI; when absent the DM
  /// screen pushes its own Peer Info page for the conversation partner.
  final void Function(String)? onOpenPeerInfo;

  /// Injectable device adapter seam. Production uses the platform adapter;
  /// widget tests supply a fake so no plugin channel is touched.
  final LocationAdapter Function()? locationAdapterFactory;

  @override
  State<DmChatScreen> createState() => _DmChatScreenState();
}

class _DmChatScreenState extends State<DmChatScreen>
    with MessageChatMixin<DmChatScreen> {
  final List<ChatMessage> _messages = [];

  /// The conversation partner's public broadcasts, shown above the DM thread
  /// as context. Mirrors the TUI's `Broadcast from …` pane. Loaded from
  /// the same DB-backed broadcast feed the home tab shows, filtered by sender;
  /// live events from this peer are added in memory only (home still owns
  /// persistence via the forwarded event sink).
  final List<ChatMessage> _broadcasts = [];
  final _broadcastScrollController = ScrollController();
  int _liveBroadcastId = -1;

  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _loading = true;

  /// Incremented for every history load. A reply that arrives after an earlier
  /// load was kicked off is stale: applying it would clobber messages received
  /// live while the fetch was in flight.
  int _historyGeneration = 0;

  /// Live typing state for this conversation (US-009). Incoming signals are
  /// frontend-only and expire locally; nothing is persisted.
  bool _peerTyping = false;
  Timer? _typingTimer;
  DateTime? _lastTypingStart;

  /// Frozen quote for the next DM we send; independent of selection mode.
  ReplyRef? _replyTo;

  /// One live location session at a time. The dialog only returns a config;
  /// this screen owns the controller so leaving the DM ends the session.
  LocationSessionController? _locationSession;
  LocationAdapter? _sessionAdapter;
  int _locationSequence = 0;
  String? _locationError;

  void _onReply(int index) async {
    if (index < 0 || index >= _messages.length) return;
    final draft = await replyDraftFor(_messages[index]);
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
    _locationSession?.dispose();
    _typingTimer?.cancel();
    if (_lastTypingStart != null) {
      unawaited(_emitTyping(false));
    }
    _scrollController.removeListener(_onScroll);
    _controller.dispose();
    _scrollController.dispose();
    _broadcastScrollController.dispose();
    setEventSink(_previousSink);
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    onListScroll(_scrollController.position);
  }

  Future<void> _loadHistory() async {
    final generation = ++_historyGeneration;
    try {
      final msgs = await nodeApi.loadDmMessages(peerId: _peerId, limit: 200);
      if (!mounted || generation != _historyGeneration) return;
      setState(() {
        // A live DM can be appended while the fetch is in flight. Merge by id
        // so those rows are not wiped by the older history snapshot.
        final historyIds = msgs.map((m) => m.id).toSet();
        final live = _messages
            .where((m) => !historyIds.contains(m.id))
            .toList();
        _messages
          ..clear()
          ..addAll(msgs)
          ..addAll(live);
        _loading = false;
      });
      scrollToBottom(_scrollController);
    } catch (_) {
      if (!mounted || generation != _historyGeneration) return;
      setState(() => _loading = false);
    }
    // The TUI's Direct tab shows the peer's public broadcasts above the DM
    // thread as context. Load that slice from the same DB-backed feed the home
    // tab shows; a failure here must never blank the DM thread.
    unawaited(_loadPeerBroadcasts(generation));
  }

  /// Load this peer's broadcasts for the context pane. Filter by sender here
  /// (Dart) rather than adding a dedicated FRB binding: `loadBroadcastMessages`
  /// is already the canonical loader and the filter mirrors the TUI's in-memory
  /// `state.messages` filter in `render_dm_content`.
  Future<void> _loadPeerBroadcasts(int generation) async {
    try {
      final all = await nodeApi.loadBroadcastMessages(limit: 200);
      if (!mounted || generation != _historyGeneration) return;
      final mine = all
          .where((m) => m.isBroadcast && m.peerId == _peerId)
          .toList();
      setState(() {
        // Preserve broadcasts that arrived live while the fetch was in flight
        // (dedup by wire id, the same key the DB rows carry).
        final loadedIds = mine.map((m) => m.msgId).whereType<String>().toSet();
        final live = _broadcasts
            .where((m) => m.msgId == null || !loadedIds.contains(m.msgId))
            .toList();
        _broadcasts
          ..clear()
          ..addAll(mine)
          ..addAll(live);
      });
      _scrollBroadcastsToBottom();
    } catch (e) {
      debugPrint('Failed to load peer broadcasts: $e');
    }
  }

  /// Jump the read-only broadcast pane to its newest row after the framework
  /// has laid out the freshly loaded list.
  void _scrollBroadcastsToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_broadcastScrollController.hasClients) {
        _broadcastScrollController.jumpTo(
          _broadcastScrollController.position.maxScrollExtent,
        );
      }
    });
  }

  /// Mirror a live broadcast from this peer into the context pane. The event is
  /// still forwarded to the home sink (which persists it); this in-memory copy
  /// is a display decoration only — like the TUI's live message list — so it is
  /// never written to the DB here and cannot double-persist.
  void _appendLiveBroadcast(SwarmEventJson event) {
    final msg = ChatMessage(
      id: _liveBroadcastId--,
      content: event.content ?? '',
      peerId: event.peerId,
      origin: const ChatMessageOrigin(isBroadcast: true, sent: false),
      msgId: event.msgId,
      replyTo: event.replyTo,
      attachment: event.attachment,
      createdAt: '',
      senderNickname: event.nickname,
      encrypted: event.encrypted,
      confirmed: false,
      edited: false,
      latency: event.latency,
      reactions: const [],
    );
    setState(() {
      if (msg.msgId == null || !_broadcasts.any((m) => m.msgId == msg.msgId)) {
        _broadcasts.add(msg);
      }
    });
    _scrollBroadcastsToBottom();
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
    if (event.eventType == 'broadcast' &&
        event.content != null &&
        event.peerId == _peerId) {
      // Mirror this peer's public broadcast into the context pane, then hand
      // the event to home so it stays the single owner of persistence (and the
      // broadcast tab stays current). Never persist here: `save_message_with_meta`
      // is a plain INSERT, so persisting twice would duplicate the row.
      _appendLiveBroadcast(event);
      _previousSink?.call(event);
      return;
    }
    if (event.eventType == 'peer_disconnected' && event.peerId == _peerId) {
      // TUI clears typing immediately when the peer drops; without this the
      // stale 3-second TTL keeps "typing…" visible after the disconnect.
      _typingTimer?.cancel();
      setState(() => _peerTyping = false);
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
    // The read-only broadcast pane can show the same wire id as the DM thread
    // (a message replied to in both), so refresh both lists like the TUI does.
    await applyReactionEvent(_messages, msgId);
    await applyReactionEvent(_broadcasts, msgId);
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
        attachment: event.attachment,
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

  void _showSnack(String message) {
    if (!mounted) return;
    ScaffoldMessenger.of(context)
      ..clearSnackBars()
      ..showSnackBar(
        SnackBar(content: Text(message), duration: const Duration(seconds: 2)),
      );
  }

  void _onLocationLinkError(String message) => _showSnack(message);

  void _reportLocationError(String message) {
    if (mounted) setState(() => _locationError = message);
    _showSnack(message);
  }

  /// Open the share dialog. Rust supplies defaults and recipients; the dialog
  /// returns only the user's choices, and the controller created here owns the
  /// live session for as long as this DM screen is open.
  Future<void> _openLocationShare() async {
    final adapter = (widget.locationAdapterFactory ?? createLocationAdapter)();
    final active = _locationSession;
    await showDialog<LocationShareConfig>(
      context: context,
      builder: (_) => LocationShareDialog(
        adapter: adapter,
        loadDefaults: nodeApi.getLocationDefaults,
        saveDefaults: (input) => nodeApi.setLocationDefaults(input: input),
        loadRecipients: nodeApi.listLocationRecipients,
        onOnce: (config) => _startLocation(config, adapter, once: true),
        onStart: (config) => _startLocation(config, adapter, once: false),
        onStop: _stopLocation,
        active: active?.active ?? false,
        activeLabel: active == null
            ? null
            : 'Sharing location · sample ${active.sequence}',
      ),
    );
    // A dialog cancelled without adopting the adapter must release it.
    if (!identical(_sessionAdapter, adapter)) unawaited(adapter.close());
  }

  Future<void> _startLocation(
    LocationShareConfig config,
    LocationAdapter adapter, {
    required bool once,
  }) async {
    late final LocationSessionController session;
    session = LocationSessionController(
      api: nodeApi,
      adapter: adapter,
      config: config,
      onError: _reportLocationError,
      onSent: (_) {
        if (!identical(_locationSession, session) || !mounted) return;
        setState(() => _locationSequence = session.sequence);
      },
      onFinished: () {
        if (!identical(_locationSession, session) || !mounted) return;
        setState(() {
          _locationSession = null;
          _sessionAdapter = null;
        });
      },
    );
    setState(() {
      _locationSession?.dispose();
      _locationSession = session;
      _sessionAdapter = adapter;
      _locationSequence = 0;
      _locationError = null;
    });
    if (once) {
      await session.startOnce();
    } else {
      await session.startContinued();
    }
    if (mounted) setState(() => _locationSequence = session.sequence);
  }

  Future<void> _stopLocation() async {
    final session = _locationSession;
    if (session == null) return;
    await session.stop();
    if (mounted) setState(() {});
  }

  /// Open this sender's device-scoped track. The DM partner is never resolved
  /// through a user/contact: [widget.peer.peerId] is the only key Rust needs.
  void _openLocationTrack() {
    Navigator.push(
      context,
      MaterialPageRoute<void>(
        builder: (_) => LocationTrackScreen(
          peerId: _peerId,
          deviceLabel: widget.peer.displayName,
        ),
      ),
    );
  }

  /// Open the tapped message sender's Peer Info. The host (HomeScreen) usually
  /// supplies [DmChatScreen.onOpenPeerInfo] so the info page gets the full
  /// roster/contact context; standalone use (widget tests) falls back to a
  /// local push for this DM's partner.
  void _openPeerInfo(String peerId) {
    final callback = widget.onOpenPeerInfo;
    if (callback != null) {
      callback(peerId);
      return;
    }
    Navigator.push(
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
    );
  }

  /// Read-only context pane showing the conversation partner's public
  /// broadcasts, mirroring the TUI Direct tab's "Broadcast from …" pane. No
  /// selection, reply/edit/react, or composer — those belong to the DM thread.
  Widget _buildBroadcastPane() {
    final cs = Theme.of(context).colorScheme;
    return Column(
      key: const Key('dm-broadcast-pane'),
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          color: cs.secondaryContainer,
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
          child: Text(
            'Broadcast from ${widget.peer.displayName}',
            key: const Key('dm-broadcast-header'),
            style: TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.bold,
              color: cs.onSecondaryContainer,
            ),
          ),
        ),
        Expanded(
          child: ListView.builder(
            controller: _broadcastScrollController,
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 4),
            itemCount: _broadcasts.length,
            itemBuilder: (context, i) => MessageBubble(
              message: _broadcasts[i],
              isOwn: false,
              onOpenPeerInfo: _openPeerInfo,
            ),
          ),
        ),
      ],
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
            if (_locationSession?.active ?? false)
              Text(
                'Sharing location · sample $_locationSequence',
                key: const Key('location-sharing-status'),
                style: const TextStyle(fontSize: 11),
              ),
          ],
        ),
        actions: [
          IconButton(
            key: const Key('dm-location'),
            icon: const Icon(Icons.location_on_outlined),
            tooltip: 'Share location',
            onPressed: () => unawaited(_openLocationShare()),
          ),
          IconButton(
            key: const Key('dm-location-track'),
            icon: const Icon(Icons.route_outlined),
            tooltip: 'Location track',
            onPressed: _openLocationTrack,
          ),
          if (_locationSession?.active ?? false)
            IconButton(
              key: const Key('location-session-stop'),
              icon: const Icon(Icons.stop_circle_outlined),
              tooltip: 'Stop sharing location',
              onPressed: () => unawaited(_stopLocation()),
            ),
          IconButton(
            icon: const Icon(Icons.info_outline),
            tooltip: 'Peer info',
            key: const Key('dm-peer-info'),
            onPressed: () => _openPeerInfo(widget.peer.peerId),
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
          if (_locationError != null)
            Container(
              width: double.infinity,
              color: Theme.of(context).colorScheme.errorContainer,
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              child: Row(
                children: [
                  Expanded(
                    child: Text(
                      _locationError!,
                      key: const Key('location-session-error'),
                      style: TextStyle(
                        color: Theme.of(context).colorScheme.onErrorContainer,
                        fontSize: 12,
                      ),
                    ),
                  ),
                  IconButton(
                    onPressed: () => setState(() => _locationError = null),
                    icon: const Icon(Icons.close, size: 16),
                    tooltip: 'Dismiss',
                    visualDensity: VisualDensity.compact,
                  ),
                ],
              ),
            ),
          if (_broadcasts.isNotEmpty)
            Expanded(flex: 1, child: _buildBroadcastPane()),
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
            onOpenPeerInfo: _openPeerInfo,
            onOpenAttachment: _openAttachment,
            onReact: _onReact,
            onLocationLinkError: _onLocationLinkError,
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
