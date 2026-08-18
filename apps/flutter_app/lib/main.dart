import 'dart:async';
import 'dart:io' show Platform;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api.dart';
import 'src/rust/mobile_api.dart';
import 'src/rust/mobile_node.dart';

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

  final List<ChatMessage> _messages = [];
  List<MobilePeerRecord> _peers = [];

  @override
  void initState() {
    super.initState();
    _onEvent = _handleSwarmEvent;
    _init();
  }

  @override
  void dispose() {
    _onEvent = null;
    super.dispose();
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
      // On desktop, auto-start the node (no foreground service needed)
      if (!_isAndroid && !_serviceRunning) {
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
        setState(() => _messages.add(msg));
        _scrollToBottom();
      }
    } catch (e) {
      debugPrint('Failed to save incoming: $e');
    }
  }

  final _scrollController = ScrollController();

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  Future<void> _sendBroadcast(String text) async {
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingBroadcast(content: text);
      setState(() => _messages.add(msg));
      _scrollToBottom();
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
  });

  final List<ChatMessage> messages;
  final ScrollController scrollController;
  final bool serviceRunning;
  final Future<void> Function(String) onSend;

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
          child: widget.messages.isEmpty
              ? Center(
                  child: Text(
                    widget.serviceRunning
                        ? 'No messages yet. Send one!'
                        : 'Start the service to chat.',
                    style: TextStyle(color: Colors.grey[500]),
                  ),
                )
              : ListView.builder(
                  controller: widget.scrollController,
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  itemCount: widget.messages.length,
                  itemBuilder: (_, i) {
                    final msg = widget.messages[i];
                    return _MessageBubble(message: msg, isOwn: msg.peerId == null);
                  },
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
  const _MessageBubble({required this.message, required this.isOwn});
  final ChatMessage message;
  final bool isOwn;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final bg = isOwn ? cs.primaryContainer : cs.surfaceContainerHighest;
    final fg = isOwn ? cs.onPrimaryContainer : cs.onSurface;

    return Align(
      alignment: isOwn ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 3),
        constraints: BoxConstraints(maxWidth: MediaQuery.of(context).size.width * 0.75),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: bg,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (!isOwn)
              Padding(
                padding: const EdgeInsets.only(bottom: 2),
                child: Text(
                  message.senderNickname ??
                      (message.peerId != null
                        ? (message.peerId!.length >= 12
                            ? message.peerId!.substring(0, 12)
                            : message.peerId!)
                        : 'Unknown'),
                  style: TextStyle(fontSize: 11, fontWeight: FontWeight.bold, color: fg.withAlpha(180)),
                ),
              ),
            Text(message.content, style: TextStyle(fontSize: 14, color: fg)),
            const SizedBox(height: 4),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  message.sentAt ?? _fmtTime(message.createdAt),
                  style: TextStyle(fontSize: 10, color: fg.withAlpha(130)),
                ),
                if (isOwn) ...[
                  const SizedBox(width: 4),
                  Icon(
                    message.sent ? Icons.check : Icons.access_time,
                    size: 12,
                    color: fg.withAlpha(130),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  String _fmtTime(String dt) => dt.length >= 16 ? dt.substring(11, 16) : dt;
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

  String get _peerId => widget.peer.peerId;
  String get _label => _peerId.length >= 16 ? _peerId.substring(0, 16) : _peerId;

  @override
  void initState() {
    super.initState();
    _onEvent = _handleDmEvent;
    _loadHistory();
  }

  @override
  void dispose() {
    _controller.dispose();
    _scrollController.dispose();
    _onEvent = null;
    super.dispose();
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
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
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
        setState(() => _messages.add(msg));
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
      _scrollToBottom();
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
            child: _loading
                ? const Center(child: CircularProgressIndicator())
                : _messages.isEmpty
                    ? Center(
                        child: Text(
                          'No messages with this peer.',
                          style: TextStyle(color: Colors.grey[500]),
                        ),
                      )
                    : ListView.builder(
                        controller: _scrollController,
                        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                        itemCount: _messages.length,
                        itemBuilder: (_, i) {
                          final msg = _messages[i];
                          return _MessageBubble(message: msg, isOwn: msg.peerId == null);
                        },
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
              child: ListView.builder(
                controller: _scrollController,
                itemCount: _logs.length,
                itemBuilder: (_, i) {
                  return Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
                    child: SelectableText(
                      _logs[i],
                      style: const TextStyle(fontSize: 11, fontFamily: 'monospace'),
                    ),
                  );
                },
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
