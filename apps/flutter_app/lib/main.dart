import 'dart:async';
import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api.dart';
import 'src/rust/mobile_api.dart';
import 'src/rust/mobile_node.dart';

import 'src/rust/mobile_api.dart';

const _serviceChannel = MethodChannel('com.example.p2p_app_flutter/service');
final bool _isAndroid = Platform.isAndroid;

String get _defaultDbPath => _isAndroid
    ? '/data/data/com.example.p2p_app_flutter/databases/p2p.db'
    : 'p2p.db';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  if (_isAndroid) {
    _serviceChannel.setMethodCallHandler(_handleServiceCall);
  }
  runApp(const P2pApp());
}

Future<dynamic> _handleServiceCall(MethodCall call) async {
  switch (call.method) {
    case 'startNetworking':
      final dbPath = call.arguments as String? ?? _defaultDbPath;
      try {
        await startNode(dbPath: dbPath);
        _startEventPolling();
        return true;
      } catch (_) {
        return false;
      }
    case 'stopNetworking':
      _stopEventPolling();
      await stopNode();
      return true;
    default:
      return null;
  }
}

Timer? _pollTimer;
void Function(SwarmEventJson)? _onEvent;

void _startEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
    try {
      final event = await pollEvent();
      if (event != null) _onEvent?.call(event);
    } catch (_) {}
  });
}

void _stopEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = null;
}

class P2pApp extends StatelessWidget {
  const P2pApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'p2p_app',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff2f5d50)),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xff2f5d50),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      themeMode: ThemeMode.system,
      home: const HomeScreen(),
    );
  }
}

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key});

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  int _tabIndex = 0;
  MobilePeerStatus? _status;
  String? _error;
  bool _loading = true;
  bool _serviceRunning = false;
  bool _atBottom = true;
  int _unreadCount = 0;
  bool _selectionMode = false;
  final Set<int> _selectedIndices = {};

  final List<ChatMessage> _messages = [];
  List<MobilePeerRecord> _peers = [];

  @override
  void initState() {
    super.initState();
    _onEvent = _handleSwarmEvent;
    _scrollController.addListener(_onScroll);
    _init();
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _onEvent = null;
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
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text('${_selectedIndices.length} message${_selectedIndices.length > 1 ? 's' : ''} copied'),
        duration: const Duration(seconds: 2),
      ),
    );
    _cancelSelection();
  }


  Future<void> _init() async {
    try {
      bool running = false;
      if (_isAndroid) {
        running = await _serviceChannel.invokeMethod<bool>('isServiceRunning') ?? false;
      }
      if (_isAndroid) {
        await initMobileDatabase(dbPath: _defaultDbPath);
      }
      final status = await getMobilePeerStatus();
      setState(() {
        _serviceRunning = running;
        _status = status;
        _loading = false;
      });
      await _loadHistory();
      await _refreshPeers();
      await _scrollToFirstUnread();
      // Auto-start the node (on Android this also starts the foreground service)
      if (!_serviceRunning) {
        await _toggleService();
      }
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  Future<void> _loadHistory() async {
    try {
      final messages = await loadBroadcastMessages(limit: 200);
      setState(() {
        _messages
          ..clear()
          ..addAll(messages);
      });
    } catch (_) {}
  }

  Future<void> _refreshPeers() async {
    try {
      final peers = await getKnownPeers();
      setState(() => _peers = peers);
    } catch (_) {}
  }

  void _handleSwarmEvent(SwarmEventJson event) {
    if (!mounted) return;
    switch (event.eventType) {
      case 'broadcast':
      case 'dm':
        if (event.content != null && event.peerId != null) {
          _saveIncoming(event.content!, event.peerId!,
              event.eventType == 'dm', event.nickname);
        }
        break;
      case 'peer_connected':
      case 'peer_discovered':
      case 'peer_disconnected':
        _refreshPeers();
        break;
    }
  }

  Future<void> _saveIncoming(String content, String peerId, bool isDirect,
      String? nickname) async {
    try {
      final msg = await saveIncomingMessage(
          content: content, peerId: peerId, isDirect: isDirect, nickname: nickname);
      if (mounted) {
        setState(() {
          _messages.add(msg);
          if (!_atBottom) _unreadCount++;
        });
        _scrollToBottom();
      }
    } catch (e) {
      debugPrint('Failed to save incoming: $e');
    }
  }

  final _scrollController = ScrollController();

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
    _saveViewedCount();
  }

  static const _lastViewedKey = 'broadcast_last_viewed_count';

  Future<void> _saveViewedCount() async {
    final prefs = await SharedPreferences.getInstance();
    await prefs.setInt(_lastViewedKey, _messages.length);
  }

  Future<void> _scrollToFirstUnread() async {
    final prefs = await SharedPreferences.getInstance();
    final lastViewed = prefs.getInt(_lastViewedKey) ?? 0;
    final unreadCount = _messages.length - lastViewed;

    if (unreadCount > 0 && _messages.isNotEmpty) {
      _unreadCount = unreadCount;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (_scrollController.hasClients) {
          final firstUnreadIndex = lastViewed.clamp(0, _messages.length - 1);
          final targetOffset = (firstUnreadIndex * 70.0) - 20.0;
          _scrollController.jumpTo(targetOffset.clamp(
            0.0, _scrollController.position.maxScrollExtent));
        }
      });
    } else {
      _forceScrollToBottom();
    }
  }

  Future<void> _sendBroadcast(String text) async {
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingBroadcast(content: text);
      setState(() => _messages.add(msg));
      _forceScrollToBottom();
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _toggleService() async {
    try {
      if (_serviceRunning) {
        if (_isAndroid) {
          await _serviceChannel.invokeMethod('stopService');
        }
        _stopEventPolling();
        await stopNode();
        setState(() => _serviceRunning = false);
      } else {
        if (_isAndroid) {
          await _serviceChannel.invokeMethod('startService', {'dbPath': _defaultDbPath});
        }
        final peerId = _isAndroid
            ? await startNode(dbPath: _defaultDbPath)
            : await startNodeAuto();
        _startEventPolling();
        setState(() => _serviceRunning = true);
        debugPrint('Node started: $peerId');
        await _loadHistory();
        await _refreshPeers();
      }
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _shareApk() async {
    if (!_isAndroid) return;
    try {
      await _serviceChannel.invokeMethod('shareApk');
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  void _openDmChat(MobilePeerRecord peer) {
    Navigator.push(
      context,
      MaterialPageRoute(
        builder: (_) => DmChatScreen(peer: peer, serviceRunning: _serviceRunning),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (_loading) {
      return const Scaffold(body: Center(child: CircularProgressIndicator()));
    }
    if (_error != null) {
      return Scaffold(
        body: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(_error!, textAlign: TextAlign.center),
              const SizedBox(height: 16),
              FilledButton(onPressed: _init, child: const Text('Retry')),
            ],
          ),
        ),
      );
    }

    final screens = [
      _BroadcastChat(
        messages: _messages,
        scrollController: _scrollController,
        serviceRunning: _serviceRunning,
        onSend: _sendBroadcast,
        unreadCount: _unreadCount,
        onJumpToBottom: _forceScrollToBottom,
        selectionMode: _selectionMode,
        selectedIndices: _selectedIndices,
        onBubbleDoubleTap: _onBubbleDoubleTap,
        onBubbleTap: _onBubbleTap,
        onCancelSelection: _cancelSelection,
        onCopySelected: _copySelected,
      ),
      _PeerList(peers: _peers, onTap: _openDmChat),
      const _LogTab(),
      _Settings(
        status: _status!,
        serviceRunning: _serviceRunning,
        onToggleService: _toggleService,
        onShareApk: _shareApk,
        isAndroid: _isAndroid,
      ),
    ];

    return Scaffold(
      body: IndexedStack(index: _tabIndex, children: screens),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tabIndex,
        onDestinationSelected: (i) => setState(() => _tabIndex = i),
        destinations: const [
          NavigationDestination(icon: Icon(Icons.chat), label: 'Chat'),
          NavigationDestination(icon: Icon(Icons.people), label: 'Peers'),
          NavigationDestination(icon: Icon(Icons.list), label: 'Log'),
          NavigationDestination(icon: Icon(Icons.settings), label: 'Settings'),
        ],
      ),
    );
  }
}

// --- Broadcast Chat Tab ---

class _BroadcastChat extends StatefulWidget {
  const _BroadcastChat({
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

  @override
  State<_BroadcastChat> createState() => _BroadcastChatState();
}

class _BroadcastChatState extends State<_BroadcastChat> {
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
                const Text('Offline', style: TextStyle(fontSize: 12, color: Colors.orange)),
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
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                        child: Column(
                          children: [
                            for (int i = 0; i < widget.messages.length; i++)
                              _MessageBubble(
                                message: widget.messages[i],
                                isOwn: widget.messages[i].peerId == null,
                                selected: widget.selectedIndices.contains(i),
                                onDoubleTap: () => widget.onBubbleDoubleTap(i),
                                onTap: () => widget.onBubbleTap(i),
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
                          onPressed: widget.selectedIndices.isEmpty ? null : widget.onCopySelected,
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
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
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
                      hintText: widget.serviceRunning ? 'Broadcast...' : 'Offline',
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

// --- Message Bubble ---

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.isOwn,
    this.selected = false,
    this.onDoubleTap,
    this.onTap,
  });
  final ChatMessage message;
  final bool isOwn;
  final bool selected;
  final VoidCallback? onDoubleTap;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final bg = isOwn ? cs.primaryContainer : cs.surfaceContainerHighest;
    final fg = isOwn ? cs.onPrimaryContainer : cs.onSurface;

    return Align(
      alignment: isOwn ? Alignment.centerRight : Alignment.centerLeft,
      child: GestureDetector(
        onDoubleTap: onDoubleTap,
        onTap: onTap,
        child: Container(
          margin: const EdgeInsets.symmetric(vertical: 3),
          constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.75),
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          decoration: BoxDecoration(
            color: selected ? cs.primary.withAlpha(40) : bg,
            borderRadius: BorderRadius.circular(12),
            border: selected ? Border.all(color: cs.primary, width: 2) : null,
          ),
          child: Text.rich(
            TextSpan(
              style: TextStyle(fontSize: 14, color: fg),
              children: [
                TextSpan(
                  text: '\n\n',
                  style: TextStyle(fontSize: 0, height: 0),
                ),
                TextSpan(
                  text: isOwn ? 'Me\n' : '${message.senderNickname ?? (message.peerId != null ? (message.peerId!.length >= 12 ? message.peerId!.substring(0, 12) : message.peerId!) : 'Unknown')}\n',
                  style: isOwn
                      ? TextStyle(fontSize: 0, height: 0)
                      : TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: fg.withAlpha(180)),
                ),
                TextSpan(text: message.content),
                TextSpan(
                  text: '\n${message.sentAt}',
                  style: TextStyle(fontSize: 10, color: fg.withAlpha(130)),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

}

// --- Peers Tab ---

class _PeerList extends StatelessWidget {
  const _PeerList({required this.peers, required this.onTap});
  final List<MobilePeerRecord> peers;
  final void Function(MobilePeerRecord) onTap;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          color: Theme.of(context).colorScheme.primaryContainer,
          child: Row(
            children: [
              const Icon(Icons.people, size: 20),
              const SizedBox(width: 8),
              Text(
                'Peers (${peers.length})',
                style: Theme.of(context).textTheme.titleMedium?.copyWith(
                      color: Theme.of(context).colorScheme.onPrimaryContainer,
                    ),
              ),
            ],
          ),
        ),
        Expanded(
          child: peers.isEmpty
              ? Center(
                  child: Text(
                    'No peers discovered yet.',
                    style: TextStyle(color: Colors.grey[500]),
                  ),
                )
              : ListView.builder(
                  itemCount: peers.length,
                  itemBuilder: (_, i) {
                    final p = peers[i];
                    return ListTile(
                      leading: CircleAvatar(
                        child: Text(
                          p.displayName.substring(0, 1).toUpperCase(),
                          style: const TextStyle(fontWeight: FontWeight.bold),
                        ),
                      ),
                      title: Text(
                        p.displayName,
                        style: const TextStyle(fontSize: 14),
                      ),
                      subtitle: Text(
                        'Seen ${(p.lastSeen.length >= 19 ? p.lastSeen.substring(0, 19) : p.lastSeen).replaceAll('T', ' ')}',
                        style: const TextStyle(fontSize: 11),
                      ),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => onTap(p),
                    );
                  },
                ),
        ),
      ],
    );
  }
}

// --- DM Chat Screen ---

class DmChatScreen extends StatefulWidget {
  const DmChatScreen({super.key, required this.peer, required this.serviceRunning});
  final MobilePeerRecord peer;
  final bool serviceRunning;

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
  String get _label => _peerId.length >= 16 ? _peerId.substring(0, 16) : _peerId;

  @override
  void initState() {
    super.initState();
    _onEvent = _handleDmEvent;
    _scrollController.addListener(_onScroll);
    _loadHistory();
  }

  @override
  void dispose() {
    _scrollController.removeListener(_onScroll);
    _controller.dispose();
    _scrollController.dispose();
    _onEvent = null;
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
          content: Text('${_selectedIndices.length} message${_selectedIndices.length > 1 ? 's' : ''} copied'),
          duration: const Duration(seconds: 2),
        ),
      );
    }
    _cancelSelection();
  }


  void _handleDmEvent(SwarmEventJson event) {
    if (!mounted || event.eventType != 'dm') return;
    if (event.peerId == _peerId && event.content != null) {
      saveIncomingMessage(
        content: event.content!,
        peerId: event.peerId!,
        isDirect: true,
        nickname: event.nickname,
      ).then((msg) {
        setState(() {
          _messages.add(msg);
          if (!_atBottom) _unreadCount++;
        });
        _scrollToBottom();
      });
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
        title: Text(_label),
        actions: [
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
                              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                              child: Column(
                                children: [
                                  for (int i = 0; i < _messages.length; i++)
                                    _MessageBubble(
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
                            onPressed: _selectedIndices.isEmpty ? null : _copySelected,
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
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
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

// --- Settings Tab ---

class _Settings extends StatefulWidget {
  const _Settings({
    required this.status,
    required this.serviceRunning,
    required this.onToggleService,
    required this.onShareApk,
    required this.isAndroid,
  });

  final MobilePeerStatus status;
  final bool serviceRunning;
  final VoidCallback onToggleService;
  final VoidCallback onShareApk;
  final bool isAndroid;

  @override
  State<_Settings> createState() => _SettingsState();
}

class _SettingsState extends State<_Settings> {
  String? _nickname;

  @override
  void initState() {
    super.initState();
    _nickname = widget.status.selfNickname;
  }

  @override
  Widget build(BuildContext context) {
    final platformLabel = widget.isAndroid ? 'Android' : 'Desktop';

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        Text('Settings', style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: 16),
        // Nickname
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text('Nickname', style: Theme.of(context).textTheme.titleSmall),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        _nickname ?? 'Not set',
                        style: const TextStyle(fontSize: 16),
                      ),
                    ),
                    IconButton(
                      onPressed: _editNickname,
                      icon: const Icon(Icons.edit),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 8),
        // Peer ID
        Card(
          child: ListTile(
            title: const Text('Peer ID'),
            subtitle: SelectableText(widget.status.localPeerId),
          ),
        ),
        // Database
        Card(
          child: ListTile(
            title: const Text('Database'),
            subtitle: SelectableText(widget.status.databaseUrl),
          ),
        ),
        // Platform
        Card(
          child: ListTile(
            leading: Icon(widget.isAndroid ? Icons.phone_android : Icons.computer),
            title: const Text('Platform'),
            subtitle: Text(platformLabel),
          ),
        ),
        const SizedBox(height: 16),
        // Service control
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Icon(
                      widget.serviceRunning ? Icons.wifi : Icons.wifi_off,
                      color: widget.serviceRunning ? Colors.green : Colors.grey,
                    ),
                    const SizedBox(width: 8),
                    Text(
                      widget.serviceRunning ? 'Node Running' : 'Node Stopped',
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                  ],
                ),
                const SizedBox(height: 12),
                Row(
                  children: [
                    FilledButton(
                      onPressed: widget.serviceRunning ? null : widget.onToggleService,
                      child: const Text('Start'),
                    ),
                    const SizedBox(width: 8),
                    OutlinedButton(
                      onPressed: widget.serviceRunning ? widget.onToggleService : null,
                      child: const Text('Stop'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        if (widget.isAndroid) ...[
          const SizedBox(height: 8),
          Card(
            child: ListTile(
              leading: const Icon(Icons.share),
              title: const Text('Share App'),
              subtitle: const Text('Send this APK to another device'),
              onTap: widget.onShareApk,
            ),
          ),
        ],
      ],
    );
  }

  void _editNickname() {
    final ctrl = TextEditingController(text: _nickname ?? '');
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Edit Nickname'),
        content: TextField(
          controller: ctrl,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: 'Enter nickname',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
          FilledButton(
            onPressed: () async {
              final nick = ctrl.text.trim();
              if (nick.isNotEmpty) {
                try {
                  await setSelfNickname(nickname: nick);
                  setState(() => _nickname = nick);
                } catch (e) {
                  debugPrint('Failed to set nickname: $e');
                }
              }
              if (ctx.mounted) Navigator.pop(ctx);
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }
}

// --- Log Tab ---

class _LogTab extends StatefulWidget {
  const _LogTab();
  @override
  State<_LogTab> createState() => _LogTabState();
}

class _LogTabState extends State<_LogTab> {
  final _scrollController = ScrollController();
  List<String> _logs = [];
  late Timer _pollTimer;

  @override
  void initState() {
    super.initState();
    // Poll for logs every second
    _pollTimer = Timer.periodic(const Duration(seconds: 1), (_) async {
      try {
        final logs = await getLogs();
        setState(() => _logs = logs);
        // Auto-scroll to bottom after new logs are displayed
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (_scrollController.hasClients) {
            _scrollController
                .jumpTo(_scrollController.position.maxScrollExtent);
          }
        });
      } catch (e) {
        debugPrint('Failed to get logs: $e');
      }
    });
  }

  @override
  void dispose() {
    _pollTimer.cancel();
    super.dispose();
    _scrollController.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Log'),
        actions: [
          if (_logs.isNotEmpty)
            IconButton(
              icon: const Icon(Icons.copy),
              onPressed: _copyAll,
              tooltip: 'Copy All',
            ),
          IconButton(
            icon: const Icon(Icons.clear),
            onPressed: () => setState(() => _logs.clear()),
            tooltip: 'Clear',
          ),
        ],
      ),
      body: _logs.isEmpty
          ? const Center(child: Text('No logs yet'))
          : SelectionArea(
              child: SingleChildScrollView(
                controller: _scrollController,
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                child: SelectableText(
                  _logs.join('\n'),
                  style: const TextStyle(
                    fontSize: 11,
                    fontFamily: 'monospace',
                    height: 1.4,
                  ),
                  textAlign: TextAlign.left,
                ),
              ),
            ),
    );
  }

  void _copyAll() {
    final text = _logs.join('\n');
    Clipboard.setData(ClipboardData(text: text));
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(content: Text('Logs copied to clipboard')),
    );
  }
}
