import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/rust/net_diag.dart';
import 'package:p2p_app_flutter/src/rust/network_config.dart';
import 'package:p2p_app_flutter/src/util/ui_utils.dart';

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
    required this.userId,
    required this.devices,
    this.networkDetails,
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
  final String? userId;
  final List<MobileDeviceInfo> devices;

  /// Live network diagnostics pushed by [HomeScreen] (interfaces, AutoNAT/
  /// DCUtR reachability, relay reservations, DHT state). Optional so widget
  /// tests without a node keep compiling; when null the card is hidden.
  final ValueNotifier<NetworkDetails?>? networkDetails;

  @override
  State<Settings> createState() => _SettingsState();
}

class _SettingsState extends State<Settings> {
  String? _nickname;
  late LiveStatus _live;
  MobilePeerStatus? _statusData;
  String _networkSize = '';
  NetworkDetails? _network;
  List<NetworkPeer>? _networkPeers;
  String? _networkPeersError;

  @override
  void initState() {
    super.initState();
    _live = widget.liveStatus.value;
    _statusData = widget.status.value;
    _nickname = _statusData?.selfNickname;
    _network = widget.networkDetails?.value;
    widget.liveStatus.addListener(_onLive);
    widget.status.addListener(_onStatus);
    widget.networkDetails?.addListener(_onNetwork);
    _refreshNetworkSize();
    _refreshNetworkPeers();
  }

  @override
  void dispose() {
    widget.liveStatus.removeListener(_onLive);
    widget.status.removeListener(_onStatus);
    widget.networkDetails?.removeListener(_onNetwork);
    super.dispose();
  }

  void _onNetwork() =>
      setState(() => _network = widget.networkDetails?.value);

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
      final label = await nodeApi.networkSizeLabel(peerCount: _live.connectedCount);
      if (mounted && label != _networkSize) {
        setState(() => _networkSize = label);
      }
    } catch (e) {
      debugPrint('Failed to resolve network size: $e');
    }
  }

  /// Load the persisted bootstrap/relay list. Best-effort: a transport or DB
  /// error is rendered in the card instead of failing the page.
  Future<void> _refreshNetworkPeers() async {
    try {
      final peers = await nodeApi.listNetworkPeers();
      if (mounted) {
        setState(() {
          _networkPeers = peers;
          _networkPeersError = null;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _networkPeersError = '$e';
        });
      }
    }
  }

  Future<void> _addNetworkPeer() async {
    final result = await showDialog<_NewNetworkPeer>(
      context: context,
      builder: (_) => const _NetworkPeerDialog(),
    );
    if (result == null || !mounted) {
      return;
    }
    final messenger = ScaffoldMessenger.of(context);
    try {
      await nodeApi.addNetworkPeer(
        kind: result.kind,
        multiaddr: result.multiaddr,
      );
      await _refreshNetworkPeers();
      messenger.showSnackBar(
        const SnackBar(
          content: Text('Network peer saved — applies on the next node start'),
        ),
      );
    } catch (e) {
      messenger.showSnackBar(SnackBar(content: Text(_errorText(e))));
    }
  }

  Future<void> _removeNetworkPeer(NetworkPeer peer) async {
    final messenger = ScaffoldMessenger.of(context);
    try {
      await nodeApi.removeNetworkPeer(id: peer.id);
      await _refreshNetworkPeers();
    } catch (e) {
      messenger.showSnackBar(SnackBar(content: Text(_errorText(e))));
    }
  }

  /// Rust returns plain Strings as errors; strip Dart's `Exception:` wrapper.
  static String _errorText(Object error) =>
      error is String ? error : '$error'.replaceFirst('Exception: ', '');

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
        // User ID (this install's account user, resolved via Rust bindings)
        Card(
          child: ListTile(
            leading: const Icon(Icons.badge),
            title: const Text('User ID'),
            subtitle: SelectableText(widget.userId ?? 'Not provisioned'),
          ),
        ),
        if (widget.userId != null && widget.devices.isNotEmpty) ...[
          const SizedBox(height: 8),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Bound devices (${widget.devices.length})',
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                  const SizedBox(height: 8),
                  for (final device in widget.devices)
                    InkWell(
                      onTap: () => widget.onOpenPeerInfo(device.peerId),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(vertical: 4),
                        child: Row(
                          children: [
                            const Icon(Icons.devices, size: 16),
                            const SizedBox(width: 8),
                            Expanded(
                              child: Text(
                                '${device.peerId} · bound ${device.boundAt}',
                                style: Theme.of(context).textTheme.bodyMedium,
                              ),
                            ),
                            const Icon(
                              Icons.chevron_right,
                              size: 16,
                              color: Colors.grey,
                            ),
                          ],
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
        ],
        const SizedBox(height: 8),
        // Manual peer address entry (US-053)
        Card(
          child: ListTile(
            leading: const Icon(Icons.add_link),
            title: const Text('Connect to address'),
            subtitle: const Text('Dial a peer by multiaddr'),
            onTap: _dialPeer,
          ),
        ),
        const SizedBox(height: 8),
        // Persisted bootstrap/relay configuration (US-050 follow-up 1)
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Row(
                  children: [
                    Text(
                      'Network peers',
                      style: Theme.of(context).textTheme.titleSmall,
                    ),
                    const Spacer(),
                    IconButton(
                      key: const Key('network-peers-add'),
                      tooltip: 'Add a bootstrap or relay multiaddr',
                      onPressed: _addNetworkPeer,
                      icon: const Icon(Icons.add),
                    ),
                  ],
                ),
                const Text(
                  'Bootstrap and relay multiaddrs. Changes apply on the next '
                  'node start.',
                  style: TextStyle(color: Colors.grey),
                ),
                const SizedBox(height: 8),
                if (_networkPeersError != null)
                  Text(
                    _networkPeersError!,
                    key: const Key('network-peers-error'),
                    style: TextStyle(color: Theme.of(context).colorScheme.error),
                  )
                else if (_networkPeers == null)
                  const Text('Loading…', style: TextStyle(color: Colors.grey))
                else if (_networkPeers!.isEmpty)
                  const Text(
                    'None configured (LAN discovery only)',
                    key: Key('network-peers-empty'),
                    style: TextStyle(color: Colors.grey),
                  )
                else
                  for (final peer in _networkPeers!)
                    Row(
                      key: ValueKey('network-peer-${peer.id}'),
                      children: [
                        Chip(label: Text(peer.kind)),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            peer.multiaddr,
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                        ),
                        IconButton(
                          key: Key('network-peer-remove-${peer.id}'),
                          tooltip: 'Remove',
                          onPressed: () => _removeNetworkPeer(peer),
                          icon: const Icon(Icons.delete_outline, size: 18),
                        ),
                      ],
                    ),
              ],
            ),
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
        if (_network != null) ...[
          const SizedBox(height: 8),
          _networkDetailsCard(_network!),
        ],
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

  Widget _networkDetailsCard(NetworkDetails net) {
    final dhtState = net.dhtConnected ? 'connected' : 'not connected';
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Network details',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 12),
            _statusRow(
              Icons.public,
              'Reachability',
              net.reachability,
              key: const Key('network-reachability'),
            ),
            _statusRow(
              Icons.router,
              'AutoNAT',
              '${net.natStatus} — probes '
                  '${net.natProbeSuccesses} ok / ${net.natProbeFailures} failed',
              key: const Key('network-autonat'),
            ),
            _statusRow(
              Icons.swap_horiz,
              'Hole punching',
              '${net.holePunchSuccesses}/${net.holePunchAttempts} succeeded',
              key: const Key('network-hole-punching'),
            ),
            _statusRow(
              Icons.alt_route,
              'Relay',
              net.relayReservations.isEmpty
                  ? net.relayStatus
                  : '${net.relayStatus} '
                        '(${net.relayReservations.join(', ')})',
              key: const Key('network-relay'),
            ),
            _statusRow(
              Icons.hub,
              'DHT',
              '$dhtState (${net.dhtMode}, '
                  '${net.dhtRoutingPeers} routing peers; bootstrap '
                  '${net.dhtBootstrapSuccesses} ok / '
                  '${net.dhtBootstrapFailures} failed)',
              key: const Key('network-dht'),
            ),
            _statusRow(
              Icons.campaign,
              'DHT announcement',
              net.dhtAnnounced
                  ? 'Announced (${net.dhtAnnounceStatus})'
                  : net.dhtAnnounceStatus,
              key: const Key('network-dht-announcement'),
            ),
            if (net.externalAddrs.isNotEmpty) ...[
              Text(
                'External addresses',
                style: Theme.of(context).textTheme.labelSmall,
              ),
              const SizedBox(height: 4),
              for (final addr in net.externalAddrs)
                Padding(
                  padding: const EdgeInsets.only(top: 4),
                  child: SelectableText(addr),
                ),
              const SizedBox(height: 8),
            ],
            Text(
              'Interfaces',
              style: Theme.of(context).textTheme.labelSmall,
            ),
            if (net.interfaces.isEmpty)
              const Padding(
                padding: EdgeInsets.only(top: 4),
                child: Text('—', style: TextStyle(color: Colors.grey)),
              )
            else
              for (final iface in net.interfaces)
                Padding(
                  key: Key('network-interface-${iface.name}'),
                  padding: const EdgeInsets.only(top: 6),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        '${iface.name} — ${iface.mac ?? 'no MAC'}'
                        '${iface.internal ? ' (loopback)' : ''}',
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      if (iface.ipv4.isNotEmpty)
                        Text('IPv4: ${iface.ipv4.join(', ')}'),
                      if (iface.ipv6.isNotEmpty)
                        Text('IPv6: ${iface.ipv6.join(', ')}'),
                    ],
                  ),
                ),
          ],
        ),
      ),
    );
  }

  Widget _statusRow(
    IconData icon,
    String label,
    String value, {
    Key? key,
  }) {
    return Padding(
      key: key,
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

  Future<void> _dialPeer() async {
    final ctrl = TextEditingController();
    final address = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Connect to peer'),
        content: TextField(
          controller: ctrl,
          autofocus: true,
          decoration: const InputDecoration(
            hintText: '/ip4/1.2.3.4/tcp/4001',
            border: OutlineInputBorder(),
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, ctrl.text.trim()),
            child: const Text('Dial'),
          ),
        ],
      ),
    );
    if (address == null || address.isEmpty) return;
    try {
      await nodeApi.dialPeer(address: address);
    } catch (e) {
      debugPrint('Failed to dial $address: $e');
    }
  }

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
    if (!await nodeApi.validateNickname(nickname: nick)) {
      setDialogState(() {
        _nicknameError = 'Use letters, numbers or dashes (max 20 characters)';
      });
      return;
    }
    try {
      await nodeApi.setSelfNickname(nickname: nick);
      if (!ctx.mounted) return;
      if (mounted) setState(() => _nickname = nick);
      Navigator.pop(ctx);
    } catch (e) {
      debugPrint('Failed to set nickname: $e');
    }
  }
}

/// Result of [_NetworkPeerDialog]: the chosen kind + raw multiaddr text.
class _NewNetworkPeer {
  const _NewNetworkPeer(this.kind, this.multiaddr);

  final String kind;
  final String multiaddr;
}

/// Add-a-network-peer dialog: kind selector (bootstrap/relay) + multiaddr.
///
/// Validation happens in Rust ([NodeApi.addNetworkPeer]) exactly like the TUI
/// path, so the dialog only checks that the field is non-empty; a validation
/// error comes back to the Settings page as a SnackBar.
class _NetworkPeerDialog extends StatefulWidget {
  const _NetworkPeerDialog();

  @override
  State<_NetworkPeerDialog> createState() => _NetworkPeerDialogState();
}

class _NetworkPeerDialogState extends State<_NetworkPeerDialog> {
  String _kind = 'bootstrap';
  final TextEditingController _addr = TextEditingController();

  @override
  void dispose() {
    _addr.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add network peer'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          DropdownButtonFormField<String>(
            key: const Key('network-peer-kind'),
            initialValue: _kind,
            decoration: const InputDecoration(labelText: 'Kind'),
            items: const [
              DropdownMenuItem(value: 'bootstrap', child: Text('Bootstrap')),
              DropdownMenuItem(value: 'relay', child: Text('Relay')),
            ],
            onChanged: (value) {
              if (value != null) setState(() => _kind = value);
            },
          ),
          const SizedBox(height: 12),
          TextField(
            key: const Key('network-peer-multiaddr'),
            controller: _addr,
            autofocus: true,
            decoration: InputDecoration(
              labelText: 'Multiaddr',
              hintText: _kind == 'relay'
                  ? '/ip4/…/tcp/4001/p2p/<peer-id>'
                  : '/ip4/…/tcp/4001',
            ),
          ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('network-peer-save'),
          onPressed: () {
            final addr = _addr.text.trim();
            if (addr.isEmpty) return;
            Navigator.pop(context, _NewNetworkPeer(_kind, addr));
          },
          child: const Text('Add'),
        ),
      ],
    );
  }
}
