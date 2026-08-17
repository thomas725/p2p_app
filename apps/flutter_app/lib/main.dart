import 'package:flutter/material.dart';

import 'src/mobile_host_api.dart';

void main() {
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
  final _hostApi = const MobileHostApi();
  late Future<MobileHostStatus> _status;

  @override
  void initState() {
    super.initState();
    _status = _hostApi.getStatus();
  }

  void _refreshStatus() {
    setState(() {
      _status = _hostApi.getStatus();
    });
  }

  Future<void> _setServiceRunning(bool running) async {
    final nextStatus =
        running ? await _hostApi.startService() : await _hostApi.stopService();
    setState(() {
      _status = Future.value(nextStatus);
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('p2p_app')),
      body: FutureBuilder<MobileHostStatus>(
        future: _status,
        builder: (context, snapshot) {
          if (snapshot.connectionState != ConnectionState.done) {
            return const Center(child: CircularProgressIndicator());
          }

          if (snapshot.hasError) {
            return _ErrorState(
              error: snapshot.error.toString(),
              onRetry: _refreshStatus,
            );
          }

          final status = snapshot.requireData;
          return _StatusView(
            status: status,
            onRefresh: _refreshStatus,
            onStart: () => _setServiceRunning(true),
            onStop: () => _setServiceRunning(false),
          );
        },
      ),
    );
  }
}

class _StatusView extends StatelessWidget {
  const _StatusView({
    required this.status,
    required this.onRefresh,
    required this.onStart,
    required this.onStop,
  });

  final MobileHostStatus status;
  final VoidCallback onRefresh;
  final VoidCallback onStart;
  final VoidCallback onStop;

  @override
  Widget build(BuildContext context) {
    return ListView(
      padding: const EdgeInsets.all(24),
      children: [
        Text('Mobile node', style: Theme.of(context).textTheme.headlineMedium),
        const SizedBox(height: 16),
        _StatusTile(
          label: 'Service',
          value: status.serviceRunning ? 'Running' : 'Stopped',
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
              onPressed: status.serviceRunning ? null : onStart,
              child: const Text('Start service'),
            ),
            OutlinedButton(
              onPressed: status.serviceRunning ? onStop : null,
              child: const Text('Stop service'),
            ),
            TextButton(onPressed: onRefresh, child: const Text('Refresh')),
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
