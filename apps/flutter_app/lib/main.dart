import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/rust/frb_generated.dart';
import 'src/rust/api.dart';
import 'src/rust/mobile_api.dart';

const _serviceChannel = MethodChannel('com.example.p2p_app_flutter/service');

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  runApp(const P2pApp());
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

  static const _dbPath =
      '/data/data/com.example.p2p_app_flutter/databases/p2p.db';

  @override
  void initState() {
    super.initState();
    _init();
  }

  Future<void> _init() async {
    try {
      final running = await _serviceChannel.invokeMethod<bool>('isServiceRunning') ?? false;
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
      } else {
        await _serviceChannel.invokeMethod('startService', {'dbPath': _dbPath});
      }
      setState(() => _serviceRunning = !_serviceRunning);
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
                  onRefresh: _refresh,
                  onToggleService: _toggleService,
                  onShareApk: _shareApk,
                ),
    );
  }
}

class _StatusView extends StatelessWidget {
  const _StatusView({
    required this.status,
    required this.serviceRunning,
    required this.onRefresh,
    required this.onToggleService,
    required this.onShareApk,
  });

  final MobilePeerStatus status;
  final bool serviceRunning;
  final VoidCallback onRefresh;
  final VoidCallback onToggleService;
  final VoidCallback onShareApk;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('Mobile node', style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: 16),
        _StatusTile(
          label: 'Service',
          value: serviceRunning ? 'Running' : 'Stopped',
        ),
        _StatusTile(label: 'Local peer ID', value: status.localPeerId),
        _StatusTile(label: 'Database', value: status.databaseUrl),
        _StatusTile(
          label: 'Nickname',
          value: status.selfNickname ?? 'Not set',
        ),
        const SizedBox(height: 24),
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
