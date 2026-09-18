import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';

/// Log tab: polls the Rust log buffer every second with copy-all and clear
/// actions.
class LogTab extends StatefulWidget {
  const LogTab({super.key});

  @override
  State<LogTab> createState() => _LogTabState();
}

class _LogTabState extends State<LogTab> {
  final _scrollController = ScrollController();
  List<String> _logs = [];
  late Timer _pollTimer;
  // Only follow the tail while the user is already at the bottom, so scrolling
  // back through older lines isn't yanked away on the next poll.
  bool _atBottom = true;

  @override
  void initState() {
    super.initState();
    _scrollController.addListener(_onScroll);
    // Poll for logs every second
    _pollTimer = Timer.periodic(const Duration(seconds: 1), (_) async {
      try {
        final logs = await getLogs();
        if (!mounted) return;
        setState(() => _logs = logs);
        if (_atBottom) _scrollToBottom();
      } catch (e) {
        debugPrint('Failed to get logs: $e');
      }
    });
  }

  @override
  void dispose() {
    _pollTimer.cancel();
    _scrollController.removeListener(_onScroll);
    _scrollController.dispose();
    super.dispose();
  }

  void _onScroll() {
    if (!_scrollController.hasClients) return;
    final pos = _scrollController.position;
    _atBottom = pos.pixels >= pos.maxScrollExtent - 4;
  }

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.jumpTo(_scrollController.position.maxScrollExtent);
      }
    });
  }

  Future<void> _clear() async {
    setState(() => _logs = []);
    try {
      await clearLogs();
    } catch (e) {
      debugPrint('Failed to clear logs: $e');
    }
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
            onPressed: _clear,
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
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(const SnackBar(content: Text('Logs copied to clipboard')));
  }
}
