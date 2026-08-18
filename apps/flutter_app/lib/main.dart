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

  // Register platform channel callbacks so Kotlin service can call into Rust
  _serviceChannel.setMethodCallHandler(_handleServiceCall);

  runApp(const P2pApp());
}

/// Called by the Kotlin foreground service to start/stop Rust networking.
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

void _startEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
    try {
      final event = await pollEvent();
      if (event != null) {
        _handleSwarmEvent(event);
      }
    } catch (_) {
      // Swarm may have stopped
    }
  });
}

void _stopEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = null;
}

void _handleSwarmEvent(SwarmEventJson event) {
  debugPrint('Swarm event: ${event.eventType} '
      'peer=${event.peerId} content=${event.content}');
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
  final List<String> _events = [];
  final _messageController = TextEditingController();

  static const _dbPath =
      '/data/data/com.example.p2p_app_flutter/databases/p2p.db';

  @override
  void initState() {
    super.initState();
    _init();
    _startEventPolling();
  }

  @override
  void dispose() {
    _messageController.dispose();
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
    } catch (e) {
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  void _refresh() {
    setState(() {
      _loading = true;
      _error = null;
    });
    _init();
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
      }
    } catch (e) {
      setState(() => _error = e.toString());
    }
  }

  Future<void> _sendBroadcast() async {
    final text = _messageController.text.trim();
    if (text.isEmpty) return;
    try {
      await sendBroadcast(content: text);
      _messageController.clear();
      setState(() => _events.add('Sent: $text'));
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
      appBar: AppBar(title: const Text('p2p_app')),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
              ? _ErrorState(error: _error!, onRetry: _refresh)
              : _StatusView(
                  status: _status!,
                  serviceRunning: _serviceRunning,
                  nodeRunning: _nodeRunning,
                  events: _events,
                  onRefresh: _refresh,
                  onToggleService: _toggleService,
                  onShareApk: _shareApk,
                  onSendBroadcast: _sendBroadcast,
                  messageController: _messageController,
                ),
    );
  }
}

class _StatusView extends StatelessWidget {
  const _StatusView({
    required this.status,
    required this.serviceRunning,
    required this.nodeRunning,
    required this.events,
    required this.onRefresh,
    required this.onToggleService,
    required this.onShareApk,
    required this.onSendBroadcast,
    required this.messageController,
  });

  final MobilePeerStatus status;
  final bool serviceRunning;
  final bool nodeRunning;
  final List<String> events;
  final VoidCallback onRefresh;
  final VoidCallback onToggleService;
  final VoidCallback onShareApk;
  final VoidCallback onSendBroadcast;
  final TextEditingController messageController;

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Expanded(
          child: ListView(
            padding: const EdgeInsets.all(24),
            children: [
              Text('Mobile node',
                  style: Theme.of(context).textTheme.headlineMedium),
              const SizedBox(height: 16),
              _StatusTile(
                label: 'Service',
                value: serviceRunning ? 'Running' : 'Stopped',
              ),
              _StatusTile(
                label: 'Node',
                value: nodeRunning ? 'Active' : 'Inactive',
              ),
              _StatusTile(label: 'Local peer ID', value: status.localPeerId),
              _StatusTile(label: 'Database', value: status.databaseUrl),
              _StatusTile(
                label: 'Nickname',
                value: status.selfNickname ?? 'Not set',
              ),
              const SizedBox(height: 16),
              Wrap(
                spacing: 12,
                runSpacing: 12,
                children: [
                  FilledButton(
                    onPressed: serviceRunning ? null : onToggleService,
                    child: const Text('Start service'),
                  ),
                  OutlinedButton(
                    onPressed: serviceRunning ? onToggleService : null,
                    child: const Text('Stop service'),
                  ),
                  TextButton(onPressed: onRefresh, child: const Text('Refresh')),
                  OutlinedButton.icon(
                    onPressed: onShareApk,
                    icon: const Icon(Icons.share),
                    label: const Text('Share App'),
                  ),
                ],
              ),
              const SizedBox(height: 16),
              if (events.isNotEmpty) ...[
                Text('Events',
                    style: Theme.of(context).textTheme.titleMedium),
                const SizedBox(height: 8),
                ...events.reversed.take(20).map(
                      (e) => Card(
                        child: ListTile(
                          dense: true,
                          title: Text(e, style: const TextStyle(fontSize: 12)),
                        ),
                      ),
                    ),
              ],
            ],
          ),
        ),
        // Broadcast input
        SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(12),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: messageController,
                    decoration: const InputDecoration(
                      hintText: 'Broadcast message...',
                      border: OutlineInputBorder(),
                      isDense: true,
                    ),
                    onSubmitted: (_) => onSendBroadcast(),
                  ),
                ),
                const SizedBox(width: 8),
                IconButton.filled(
                  onPressed: onSendBroadcast,
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

class _StatusTile extends StatelessWidget {
  const _StatusTile({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: ListTile(
        title: Text(label),
        subtitle: SelectableText(value),
      ),
    );
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
