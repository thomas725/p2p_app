import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/formats.dart';

/// Full-screen peer detail page (name origin, activity), with a shortcut to
/// open the DM chat for the peer.
class PeerInfoScreen extends StatelessWidget {
  const PeerInfoScreen({
    super.key,
    required this.peer,
    required this.serviceRunning,
    this.onNavigate,
    this.currentTab = 1,
  });
  final MobilePeerRecord peer;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  // Where the displayed name came from: local nickname > received > generated.
  static (String, String) _origin(MobilePeerRecord peer) {
    if (peer.localNickname != null) {
      return ('Local nickname', 'You set this nickname for the peer.');
    }
    if (peer.nickname != null) {
      return (
        'Received nickname',
        'Announced by the peer. (Receipt time is not tracked.)',
      );
    }
    return (
      'Generated petname',
      'No nickname was known, so a petname was assigned locally.',
    );
  }

  @override
  Widget build(BuildContext context) {
    final (origin, originDetail) = _origin(peer);
    return Scaffold(
      appBar: AppBar(title: Text(peer.displayName)),
      bottomNavigationBar: MainTabNavBar(
        currentIndex: currentTab,
        onSelect: (i) {
          Navigator.pop(context);
          onNavigate?.call(i);
        },
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Display name',
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                  const SizedBox(height: 4),
                  Text(
                    peer.displayName,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    'Peer ID',
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                  const SizedBox(height: 4),
                  SelectableText(
                    peer.peerId,
                    style: const TextStyle(
                      fontFamily: 'monospace',
                      fontSize: 12,
                    ),
                  ),
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text('Origin', style: Theme.of(context).textTheme.labelSmall),
                  const SizedBox(height: 4),
                  Text(origin, style: Theme.of(context).textTheme.titleMedium),
                  const SizedBox(height: 4),
                  Text(
                    originDetail,
                    style: Theme.of(context).textTheme.bodySmall,
                  ),
                  const SizedBox(height: 12),
                  if (peer.localNickname != null) ...[
                    _infoRow('Local nickname', peer.localNickname!),
                    const SizedBox(height: 8),
                  ],
                  if (peer.nickname != null) ...[
                    _infoRow('Received nickname', peer.nickname!),
                    const SizedBox(height: 8),
                  ],
                ],
              ),
            ),
          ),
          const SizedBox(height: 12),
          Card(
            child: Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'Activity',
                    style: Theme.of(context).textTheme.labelSmall,
                  ),
                  const SizedBox(height: 4),
                  _infoRow('First seen', formatSeenTimestamp(peer.firstSeen)),
                  const SizedBox(height: 8),
                  _infoRow('Last seen', formatSeenTimestamp(peer.lastSeen)),
                ],
              ),
            ),
          ),
          const SizedBox(height: 16),
          FilledButton.icon(
            icon: const Icon(Icons.chat),
            label: const Text('Open direct message'),
            onPressed: () => Navigator.push(
              context,
              MaterialPageRoute(
                builder: (_) => DmChatScreen(
                  peer: peer,
                  serviceRunning: serviceRunning,
                  onNavigate: onNavigate,
                  currentTab: currentTab,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  static Widget _infoRow(String label, String value) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SizedBox(
          width: 110,
          child: Text(
            label,
            style: const TextStyle(fontSize: 13, fontWeight: FontWeight.w600),
          ),
        ),
        Expanded(
          child: SelectableText(value, style: const TextStyle(fontSize: 13)),
        ),
      ],
    );
  }
}
