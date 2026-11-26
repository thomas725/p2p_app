import 'dart:convert';

import 'package:flutter/foundation.dart' show ValueListenable;
import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/shares_json.dart'
    show shareOfferFromJson;
import 'package:p2p_app_flutter/src/rust/shares.dart' show ShareOffer;
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';

/// One advertised share together with the connection peer that sent it. Peer
/// identity matters: the same share id from two peers is two separate offers,
/// and only the connection peer can serve its manifest.
typedef DiscoveredShare = ({String peerId, ShareOffer offer});

/// Merge an advertised offer into the discovered list without duplicating
/// retransmits. The newest announcement for the same peer/share replaces the
/// old row so a peer's current name/revision is what the user sees.
List<DiscoveredShare> mergeDiscoveredShares(
  List<DiscoveredShare> existing,
  String peerId,
  ShareOffer offer,
) {
  final index = existing.indexWhere(
    (discovered) =>
        discovered.peerId == peerId &&
        discovered.offer.shareId == offer.shareId,
  );
  final next = List<DiscoveredShare>.from(existing);
  final entry = (peerId: peerId, offer: offer);
  if (index == -1) {
    next.add(entry);
  } else {
    next[index] = entry;
  }
  return next;
}

/// Decode the `share_offered` event payload into a discovery row. Returns null
/// for a missing peer/content or a malformed payload: one bad announcement
/// must never break the rest of the event stream.
DiscoveredShare? discoveredShareFromEvent(String? peerId, String? content) {
  if (peerId == null || content == null) return null;
  try {
    return (
      peerId: peerId,
      offer: shareOfferFromJson(jsonDecode(content) as Map<String, dynamic>),
    );
  } catch (_) {
    return null;
  }
}

/// Ask a connected peer what it shares, then add an advertised share to this
/// install's local list. Adding is explicit: an offer is untrusted metadata,
/// and Rust's `accept_offer` creates a stub without granting access to ours.
class ShareDiscoverySection extends StatefulWidget {
  const ShareDiscoverySection({
    super.key,
    required this.peerIds,
    required this.offers,
    required this.onDiscover,
    required this.onAccept,
  });

  final List<String> peerIds;
  final ValueListenable<List<DiscoveredShare>> offers;
  final Future<void> Function(String peerId) onDiscover;
  final Future<void> Function(DiscoveredShare discovered) onAccept;

  @override
  State<ShareDiscoverySection> createState() => _ShareDiscoverySectionState();
}

class _ShareDiscoverySectionState extends State<ShareDiscoverySection> {
  String? _peerId;
  bool _busy = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _peerId = _firstPeer();
  }

  @override
  void didUpdateWidget(covariant ShareDiscoverySection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (_peerId == null || !widget.peerIds.contains(_peerId)) {
      final peerId = _firstPeer();
      if (peerId != _peerId) setState(() => _peerId = peerId);
    }
  }

  String? _firstPeer() => widget.peerIds.isEmpty ? null : widget.peerIds.first;

  Future<void> _perform(String label, Future<void> Function() action) async {
    if (_busy) return;
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await action();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = '$label failed: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _discover() async {
    final peerId = _peerId;
    if (peerId == null) return;
    await _perform('Discovery', () => widget.onDiscover(peerId));
  }

  Future<void> _accept(DiscoveredShare discovered) =>
      _perform('Add share', () => widget.onAccept(discovered));

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Card(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  'Ask a connected peer',
                  style: Theme.of(context).textTheme.titleSmall,
                ),
                const SizedBox(height: 12),
                if (widget.peerIds.isEmpty)
                  const Text('No connected peers. Discovery needs a peer.')
                else ...[
                  DropdownButtonFormField<String>(
                    key: const Key('share-discovery-peer'),
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
                            if (peerId != null) {
                              setState(() => _peerId = peerId);
                            }
                          },
                  ),
                  const SizedBox(height: 8),
                  FilledButton.tonalIcon(
                    key: const Key('share-discovery-search'),
                    icon: const Icon(Icons.search),
                    label: const Text('Search shares'),
                    onPressed: _busy ? null : _discover,
                  ),
                ],
              ],
            ),
          ),
        ),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Text(
              _error!,
              key: const Key('share-discovery-error'),
              style: TextStyle(color: cs.error),
            ),
          ),
        Expanded(
          child: ValueListenableBuilder<List<DiscoveredShare>>(
            valueListenable: widget.offers,
            builder: (context, offers, _) {
              if (offers.isEmpty) {
                return const Center(
                  child: Text(
                    'No offers received yet. Search a peer to see its shares.',
                  ),
                );
              }
              return ListView.separated(
                itemCount: offers.length,
                separatorBuilder: (_, _) => const Divider(height: 1),
                itemBuilder: (context, i) {
                  final discovered = offers[i];
                  final offer = discovered.offer;
                  final owner = offer.ownerUserId ?? 'unknown';
                  final key = '${discovered.peerId}-${offer.shareId}';
                  return ListTile(
                    key: Key('discovered-share-$key'),
                    title: Text(offer.name),
                    subtitle: Text(
                      '${offer.kind} · rev ${offer.revision} · from '
                      '${discovered.peerId}\nowner $owner (unverified)',
                    ),
                    isThreeLine: true,
                    trailing: FilledButton.tonal(
                      key: Key('discovered-accept-$key'),
                      onPressed: _busy ? null : () => _accept(discovered),
                      child: const Text('Add'),
                    ),
                  );
                },
              );
            },
          ),
        ),
      ],
    );
  }
}

/// Full-screen discovery route launched from the share list's app bar. The
/// offer list is a notifier owned by Home, so events arriving while the route
/// is open appear without needing to rebuild the route from scratch.
class ShareDiscoveryScreen extends StatelessWidget {
  const ShareDiscoveryScreen({
    super.key,
    required this.peerIds,
    required this.offers,
    required this.onDiscover,
    required this.onAccept,
    this.onNavigate,
    this.currentTab = 4,
  });

  final List<String> peerIds;
  final ValueListenable<List<DiscoveredShare>> offers;
  final Future<void> Function(String peerId) onDiscover;
  final Future<void> Function(DiscoveredShare discovered) onAccept;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Discover shares')),
      body: Padding(
        padding: const EdgeInsets.all(12),
        child: ShareDiscoverySection(
          peerIds: peerIds,
          offers: offers,
          onDiscover: onDiscover,
          onAccept: onAccept,
        ),
      ),
      bottomNavigationBar: onNavigate == null
          ? null
          : MainTabNavBar(
              currentIndex: currentTab,
              onSelect: (i) {
                Navigator.pop(context);
                onNavigate!(i);
              },
            ),
    );
  }
}
