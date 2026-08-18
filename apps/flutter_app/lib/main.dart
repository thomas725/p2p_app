import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api.dart';
import 'src/rust/mobile_api.dart';
import 'src/rust/mobile_node.dart';

const _serviceChannel = MethodChannel('com.example.p2p_app_flutter/service');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();

  _serviceChannel.setMethodCallHandler(_handleServiceCall);

  runApp(const P2pApp());
}

Future<dynamic> _handleServiceCall(MethodCall call) async {
  switch (call.method) {
    case 'startNetworking':
      final dbPath = call.arguments as String? ??
          '/data/data/com.example.p2p_app_flutter/databases/p2p.db';
      try {
        await startNode(dbPath: dbPath);
        _startEventPolling();
        return true;
      } catch (e) {
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
Function(SwarmEventJson)? _onEvent;

void _startEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
    try {
      final event = await pollEvent();
      if (event != null) {
        _onEvent?.call(event);
      }
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
  MobilePeerStatus? _status;
  String? _error;
  bool _loading = true;
  bool _serviceRunning = false;
  bool _nodeRunning = false;

  final List<ChatMessage> _messages = [];
  final _messageController = TextEditingController();
  final _scrollController = ScrollController();

  static const _dbPath =
      '/data/data/com.example.p2p_app_flutter/databases/p2p.db';

  @override
  void initState() {
    super.initState();
    _onEvent = _handleSwarmEvent;
    _init();
  }

  @override
  void dispose() {
    _messageController.dispose();
    _scrollController.dispose();
    _onEvent = null;
    super.dispose();
  }

  Future<void> _init() async {
    try {
      final running =
          await _serviceChannel.invokeMethod<bool>('isServiceRunning') ?? false;
      await initMobileDatabase(dbPath: _dbPath);
      final status = await getMobilePeerStatus();
      setState(() {
        _serviceRunning = running;
        _status = status;
        _loading = false;
      });
      // Load message history
      await _loadHistory();
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
      _scrollToBottom();
    } catch (_) {}
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  void _refresh() {
    setState(() {
      _loading = true;
      _error = null;
    });
    _init();
  }

  void _handleSwarmEvent(SwarmEventJson event) {
    if (!mounted) return;

    switch (event.eventType) {
      case 'broadcast':
      case 'dm':
        if (event.content != null && event.peerId != null) {
          _saveIncoming(
            event.content!,
            event.peerId!,
            event.eventType == 'dm',
            event.nickname,
          );
        }
        break;
      default:
        debugPrint('Swarm event: ${event.eventType} peer=${event.peerId}');
    }
  }

  Future<void> _saveIncoming(
    String content,
    String peerId,
    bool isDirect,
    String? nickname,
  ) async {
    try {
      final msg = await saveIncomingMessage(
        content: content,
        peerId: peerId,
        isDirect: isDirect,
        nickname: nickname,
      );
      if (mounted) {
        setState(() => _messages.add(msg));
        _scrollToBottom();
      }
    } catch (e) {
      debugPrint('Failed to save incoming: $e');
    }
  }

  Future<void> _sendBroadcast() async {
    final text = _messageController.text.trim();
    if (text.isEmpty) return;
    try {
      final msg = await saveOutgoingBroadcast(content: text);
      _messageController.clear();
      setState(() => _messages.add(msg));
      _scrollToBottom();
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _toggleService() async {
    try {
      if (_serviceRunning) {
        await _serviceChannel.invokeMethod('stopService');
        _stopEventPolling();
        await stopNode();
        setState(() {
          _serviceRunning = false;
          _nodeRunning = false;
        });
      } else {
        await _serviceChannel
            .invokeMethod('startService', {'dbPath': _dbPath});
        final peerId = await startNode(dbPath: _dbPath);
        _startEventPolling();
        setState(() {
          _serviceRunning = true;
          _nodeRunning = true;
        });
        debugPrint('Node started: $peerId');
        await _loadHistory();
      }
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _shareApk() async {
    try {
      await _serviceChannel.invokeMethod('shareApk');
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('p2p_app'),
        actions: [
          IconButton(
            onPressed: _shareApk,
            icon: const Icon(Icons.share),
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? _ErrorState(error: _error!, onRetry: _refresh)
              : _ChatView(
                  status: _status!,
                  serviceRunning: _serviceRunning,
                  messages: _messages,
                  scrollController: _scrollController,
                  onRefresh: _refresh,
                  onToggleService: _toggleService,
                  onSendBroadcast: _sendBroadcast,
                  messageController: _messageController,
                ),
    );
  }
}

class _ChatView extends StatelessWidget {
  const _ChatView({
    required this.status,
    required this.serviceRunning,
    required this.messages,
    required this.scrollController,
    required this.onRefresh,
    required this.onToggleService,
    required this.onSendBroadcast,
    required this.messageController,
  });

  final MobilePeerStatus status;
  final bool serviceRunning;
  final List<ChatMessage> messages;
  final ScrollController scrollController;
  final VoidCallback onRefresh;
  final VoidCallback onToggleService;
  final VoidCallback onSendBroadcast;
  final TextEditingController messageController;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        // Status bar
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
          color: Theme.of(context).colorScheme.surfaceContainerHighest,
          child: Row(
            children: [
              Icon(
                serviceRunning ? Icons.wifi : Icons.wifi_off,
                size: 16,
                color: serviceRunning ? Colors.green : Colors.grey,
              ),
              const SizedBox(width: 8),
              Text(
                status.localPeerId.substring(0, 12),
                style: Theme.of(context).textTheme.bodySmall,
              ),
              const Spacer(),
              TextButton(
                onPressed: onToggleService,
                child: Text(serviceRunning ? 'Stop' : 'Start'),
              ),
            ],
          ),
        ),
        // Messages
        Expanded(
          child: messages.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        serviceRunning ? Icons.chat_bubble_outline : Icons.wifi_off,
                        size: 48,
                        color: Colors.grey,
                      ),
                      const SizedBox(height: 16),
                      Text(
                        serviceRunning
                            ? 'No messages yet. Send one!'
                            : 'Start the service to chat.',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                              color: Colors.grey,
                            ),
                      ),
                    ],
                  ),
                )
              : ListView.builder(
                  controller: scrollController,
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                  itemCount: messages.length,
                  itemBuilder: (context, index) {
                    final msg = messages[index];
                    final isOwn = msg.peerId == null;
                    return _MessageBubble(
                      message: msg,
                      isOwn: isOwn,
                    );
                  },
                ),
        ),
        // Input
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: messageController,
                    decoration: InputDecoration(
                      hintText: serviceRunning
                          ? 'Broadcast message...'
                          : 'Start the service first',
                      border: const OutlineInputBorder(),
                      isDense: true,
                      enabled: serviceRunning,
                    ),
                    onSubmitted: (_) => onSendBroadcast(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filled(
                  onPressed: serviceRunning ? onSendBroadcast : null,
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

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({
    required this.message,
    required this.isOwn,
  });

  final ChatMessage message;
  final bool isOwn;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final bubbleColor =
        isOwn ? colorScheme.primaryContainer : colorScheme.surfaceContainerHighest;
    final textColor =
        isOwn ? colorScheme.onPrimaryContainer : colorScheme.onSurface;

    return Align(
      alignment: isOwn ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        margin: const EdgeInsets.symmetric(vertical: 4),
        constraints: BoxConstraints(
          maxWidth: MediaQuery.of(context).size.width * 0.75,
        ),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: bubbleColor,
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (!isOwn) ...[
              Text(
                message.senderNickname ??
                    (message.peerId?.substring(0, 12) ?? 'Unknown'),
                style: TextStyle(
                  fontSize: 11,
                  fontWeight: FontWeight.bold,
                  color: textColor.withValues(alpha: 0.7),
                ),
              ),
              const SizedBox(height: 2),
            ],
            Text(
              message.content,
              style: TextStyle(fontSize: 14, color: textColor),
            ),
            const SizedBox(height: 4),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(
                  message.sentAt ?? _formatTime(message.createdAt),
                  style: TextStyle(
                    fontSize: 10,
                    color: textColor.withValues(alpha: 0.5),
                  ),
                ),
                if (isOwn) ...[
                  const SizedBox(width: 4),
                  Icon(
                    message.sent ? Icons.check : Icons.access_time,
                    size: 12,
                    color: textColor.withValues(alpha: 0.5),
                  ),
                ],
              ],
            ),
          ],
        ),
      ),
    );
  }

  String _formatTime(String dt) {
    // Extract HH:MM from "2026-08-18 07:29:19"
    if (dt.length >= 16) return dt.substring(11, 16);
    return dt;
  }
}

class _ErrorState extends StatelessWidget {
  const _ErrorState({required this.error, required this.onRetry});

  final String error;
  final VoidCallback onRetry;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(error, textAlign: TextAlign.center),
            const SizedBox(height: 16),
            FilledButton(onPressed: onRetry, child: const Text('Retry')),
          ],
        ),
      ),
    );
  }
}
