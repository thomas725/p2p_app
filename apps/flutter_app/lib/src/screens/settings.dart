import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/util/formats.dart';

/// Immutable snapshot of the live connection state shown on the Settings page.
///
/// Held in a [ValueNotifier] so the Settings page can repaint on connect/
/// disconnect events even while it is offscreen (it then reads the latest value
/// once it becomes visible again).
class LiveStatus {
  const LiveStatus({
    required this.connectedCount,
    required this.connectedPeerIds,
    required this.listenAddresses,
    required this.lastConnectionAt,
  });

  final int connectedCount;
  final List<String> connectedPeerIds;
  final List<String> listenAddresses;
  final DateTime? lastConnectionAt;
}

/// Settings tab: node/service control, nickname editing, peer/network status,
/// and (Android) APK sharing.
class Settings extends StatefulWidget {
  const Settings({
    super.key,
    required this.liveStatus,
    required this.status,
    required this.serviceRunning,
    required this.onToggleService,
    required this.onShareApk,
    required this.isAndroid,
    required this.peers,
    required this.networkName,
    required this.onOpenPeerInfo,
  });

  final ValueNotifier<LiveStatus> liveStatus;
  final ValueNotifier<MobilePeerStatus?> status;
  final bool serviceRunning;
  final VoidCallback onToggleService;
  final VoidCallback onShareApk;
  final bool isAndroid;
  final List<MobilePeerRecord> peers;
  final String networkName;
  final void Function(String) onOpenPeerInfo;

  @override
  State<Settings> createState() => _SettingsState();
}

class _SettingsState extends State<Settings> {
  String? _nickname;
  late LiveStatus _live;
  MobilePeerStatus? _statusData;
  String _networkSize = '';

  @override
  void initState() {
    super.initState();
    _live = widget.liveStatus.value;
    _statusData = widget.status.value;
    _nickname = _statusData?.selfNickname;
    widget.liveStatus.addListener(_onLive);
    widget.status.addListener(_onStatus);
    _refreshNetworkSize();
  }

  @override
  void dispose() {
    widget.liveStatus.removeListener(_onLive);
    widget.status.removeListener(_onStatus);
    super.dispose();
  }

  void _onLive() {
    setState(() => _live = widget.liveStatus.value);
    _refreshNetworkSize();
  }

  void _onStatus() => setState(() {
    _statusData = widget.status.value;
    _nickname = _statusData?.selfNickname;
  });

  /// Resolve the network-size label through the Rust binding so the Dart UI
  /// never re-derives the thresholds.
  Future<void> _refreshNetworkSize() async {
    try {
      final label = await networkSizeLabel(peerCount: _live.connectedCount);
      if (mounted && label != _networkSize) {
        setState(() => _networkSize = label);
      }
    } catch (e) {
      debugPrint('Failed to resolve network size: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final platformLabel = widget.isAndroid ? 'Android' : 'Desktop';
    final status = _statusData;

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
            subtitle: SelectableText(status?.localPeerId ?? ''),
          ),
        ),
        // Database
        Card(
          child: ListTile(
            title: const Text('Database'),
            subtitle: SelectableText(status?.databaseUrl ?? ''),
          ),
        ),
        // Platform
        Card(
          child: ListTile(
            leading: Icon(
              widget.isAndroid ? Icons.phone_android : Icons.computer,
            ),
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
                      onPressed: widget.serviceRunning
                          ? null
                          : widget.onToggleService,
                      child: const Text('Start'),
                    ),
                    const SizedBox(width: 8),
                    OutlinedButton(
                      onPressed: widget.serviceRunning
                          ? widget.onToggleService
                          : null,
                      child: const Text('Stop'),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 16),
        // Node & network status
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Node & Network',
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 12),
                _statusRow(
                  Icons.link,
                  'Connected peers',
                  _live.connectedCount.toString(),
                ),
                if (_live.connectedPeerIds.isEmpty)
                  const Padding(
                    padding: EdgeInsets.only(top: 4),
                    child: Text('—', style: TextStyle(color: Colors.grey)),
                  )
                else
                  ..._connectedPeerRows(),
                if (_live.connectedCount == 0)
                  _statusRow(
                    Icons.history,
                    'Last connection lost',
                    formatTimestamp(_live.lastConnectionAt),
                  ),
                _statusRow(
                  Icons.network_cell,
                  'Network name',
                  widget.networkName,
                ),
                _statusRow(Icons.hub, 'Network size', _networkSize),
                const SizedBox(height: 8),
                Text(
                  'Listen addresses',
                  style: Theme.of(context).textTheme.labelSmall,
                ),
                if (_live.listenAddresses.isEmpty)
                  const Padding(
                    padding: EdgeInsets.only(top: 4),
                    child: Text('—', style: TextStyle(color: Colors.grey)),
                  )
                else
                  ..._live.listenAddresses.map(
                    (a) => Padding(
                      padding: const EdgeInsets.only(top: 4),
                      child: SelectableText(a),
                    ),
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

  Widget _statusRow(IconData icon, String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Row(
        children: [
          Icon(icon, size: 18),
          const SizedBox(width: 8),
          Expanded(
            child: Text(label, style: Theme.of(context).textTheme.bodyMedium),
          ),
          Text(
            value,
            style: Theme.of(
              context,
            ).textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.bold),
          ),
        ],
      ),
    );
  }

  List<Widget> _connectedPeerRows() {
    final byId = {for (final p in widget.peers) p.peerId: p};
    return [
      for (final id in _live.connectedPeerIds)
        InkWell(
          onTap: () => widget.onOpenPeerInfo(id),
          child: Padding(
            padding: const EdgeInsets.only(top: 4),
            child: Row(
              children: [
                const Icon(Icons.person, size: 16),
                const SizedBox(width: 8),
                Expanded(child: Text(byId[id]?.displayName ?? id)),
                const Icon(Icons.chevron_right, size: 16, color: Colors.grey),
              ],
            ),
          ),
        ),
    ];
  }

  String? _nicknameError;

  void _editNickname() {
    final ctrl = TextEditingController(text: _nickname ?? '');
    _nicknameError = null;
    showDialog(
      context: context,
      builder: (ctx) => StatefulBuilder(
        builder: (ctx, setDialogState) => AlertDialog(
          title: const Text('Edit Nickname'),
          content: TextField(
            controller: ctrl,
            autofocus: true,
            onSubmitted: (_) => _saveNickname(ctx, ctrl, setDialogState),
            decoration: InputDecoration(
              hintText: 'Enter nickname',
              border: const OutlineInputBorder(),
              errorText: _nicknameError,
            ),
          ),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx),
              child: const Text('Cancel'),
            ),
            FilledButton(
              onPressed: () => _saveNickname(ctx, ctrl, setDialogState),
              child: const Text('Save'),
            ),
          ],
        ),
      ),
    );
  }

  Future<void> _saveNickname(
    BuildContext ctx,
    TextEditingController ctrl,
    StateSetter setDialogState,
  ) async {
    final nick = ctrl.text.trim();
    if (nick.isEmpty) return;
    // Reject via the Rust single source of truth: alphanumeric + dash, max 20.
    if (!await validateNickname(nickname: nick)) {
      setDialogState(() {
        _nicknameError = 'Use letters, numbers or dashes (max 20 characters)';
      });
      return;
    }
    try {
      await setSelfNickname(nickname: nick);
      if (!ctx.mounted) return;
      if (mounted) setState(() => _nickname = nick);
      Navigator.pop(ctx);
    } catch (e) {
      debugPrint('Failed to set nickname: $e');
    }
  }
}
