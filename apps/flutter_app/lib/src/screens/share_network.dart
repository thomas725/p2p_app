import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart' show MobileShare;

/// Network actions for one share: ask a connected peer for the latest manifest
/// and re-advertise our copy of the offer to that peer.
///
/// Peer IDs, not display names, are the protocol-level identity here; Home
/// supplies the currently connected peers. Both actions go through `NodeApi`,
/// so they behave identically on FRB and over the browser's JSON transport.
class ShareNetworkSection extends StatefulWidget {
  const ShareNetworkSection({
    super.key,
    required this.share,
    required this.peerIds,
    required this.onRequestManifest,
    required this.onReadvertise,
    this.onCompleted,
  });

  final MobileShare share;
  final List<String> peerIds;
  final Future<void> Function(String shareId, String peerId) onRequestManifest;
  final Future<void> Function(String shareId, String peerId) onReadvertise;

  /// Called after a successful action so the detail page can reload the
  /// manifest/grants Rust just persisted from the network.
  final Future<void> Function()? onCompleted;

  @override
  State<ShareNetworkSection> createState() => _ShareNetworkSectionState();
}

class _ShareNetworkSectionState extends State<ShareNetworkSection> {
  String? _peerId;
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _peerId = _firstPeer();
  }

  @override
  void didUpdateWidget(covariant ShareNetworkSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_peerId == null || !widget.peerIds.contains(_peerId)) {
      final peerId = _firstPeer();
      if (peerId != _peerId) setState(() => _peerId = peerId);
    }
  }

  String? _firstPeer() => widget.peerIds.isEmpty ? null : widget.peerIds.first;

  Future<void> _perform(
    String label,
    Future<void> Function(String peerId) action,
  ) async {
    final peerId = _peerId;
    if (peerId == null || _busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action(peerId);
      if (mounted) await widget.onCompleted?.call();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$label failed: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _refreshManifest() => _perform(
    'Manifest refresh',
    (peerId) => widget.onRequestManifest(widget.share.shareId, peerId),
  );

  Future<void> _readvertise() => _perform(
    'Re-advertise',
    (peerId) => widget.onReadvertise(widget.share.shareId, peerId),
  );

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('Network', style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 12),
            if (widget.peerIds.isEmpty)
              const Text('No connected peers. Network actions need a peer.')
            else ...[
              DropdownButtonFormField<String>(
                key: const Key('share-network-peer'),
                initialValue: _peerId,
                decoration: const InputDecoration(
                  labelText: 'Connected peer',
                  hintText: 'peer-…',
                ),
                items: [
                  for (final peerId in widget.peerIds)
                    DropdownMenuItem(value: peerId, child: Text(peerId)),
                ],
                onChanged: _busy
                    ? null
                    : (peerId) {
                        if (peerId != null) setState(() => _peerId = peerId);
                      },
              ),
              const SizedBox(height: 8),
              Row(
                children: [
                  FilledButton.tonalIcon(
                    key: const Key('share-network-refresh'),
                    icon: const Icon(Icons.refresh),
                    label: const Text('Refresh manifest'),
                    onPressed: _busy ? null : _refreshManifest,
                  ),
                  const SizedBox(width: 8),
                  FilledButton.tonalIcon(
                    key: const Key('share-network-readvertise'),
                    icon: const Icon(Icons.send),
                    label: const Text('Re-advertise'),
                    onPressed: _busy ? null : _readvertise,
                  ),
                ],
              ),
            ],
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(
                  _error!,
                  key: const Key('share-network-error'),
                  style: TextStyle(color: cs.error),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
