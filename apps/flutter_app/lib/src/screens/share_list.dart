import 'dart:async' show unawaited;

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart'
    show MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/shares.dart' show ShareManifest;
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';

/// Read-only list of this install's file shares. Creation, binding and
/// publishing stay in Rust for now (see `docs/backlog.md`); this slice only
/// surfaces what the node already knows so both the FRB and browser transports
/// can inspect shares and their manifests.
class ShareList extends StatelessWidget {
  const ShareList({super.key, required this.shares, required this.onOpenShare});

  final List<MobileShare> shares;
  final void Function(MobileShare) onOpenShare;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          color: cs.primaryContainer,
          child: Row(
            children: [
              const Icon(Icons.folder_shared, size: 20),
              const SizedBox(width: 8),
              Text(
                'Shares (${shares.length})',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(color: cs.onPrimaryContainer),
              ),
            ],
          ),
        ),
        Expanded(
          child: shares.isEmpty
              ? Center(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(
                        Icons.folder_shared_outlined,
                        size: 48,
                        color: Colors.grey[400],
                      ),
                      const SizedBox(height: 8),
                      Text(
                        'No shares yet. Shares appear when the node creates or '
                        'accepts one.',
                        textAlign: TextAlign.center,
                        style: TextStyle(color: Colors.grey[500]),
                      ),
                    ],
                  ),
                )
              : ListView.separated(
                  itemCount: shares.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, i) {
                    final share = shares[i];
                    final bound = share.localRoot == null ? '' : ' · bound';
                    return ListTile(
                      key: Key('share-row-${share.shareId}'),
                      leading: const CircleAvatar(
                        child: Icon(Icons.folder_shared),
                      ),
                      title: Text(share.name),
                      subtitle: Text(
                        '${share.kind} · rev ${share.revision.toInt()}$bound',
                      ),
                      trailing: const Icon(Icons.chevron_right),
                      onTap: () => onOpenShare(share),
                    );
                  },
                ),
        ),
      ],
    );
  }
}

/// Full-screen route hosting [ShareList], with the same bottom navigation as
/// the other detail routes so the user can jump back to any main tab.
class ShareListScreen extends StatelessWidget {
  const ShareListScreen({
    super.key,
    required this.shares,
    required this.onOpenShare,
    this.onNavigate,
    this.currentTab = 4,
  });

  final List<MobileShare> shares;
  final void Function(MobileShare) onOpenShare;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('File shares')),
      body: ShareList(shares: shares, onOpenShare: onOpenShare),
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

/// Read-only detail route: share metadata, per-user grants and the files in
/// the current manifest. Loaders are injected (like `PeerInfoScreen`) so the
/// page needs no Rust library in widget tests; Home supplies the `NodeApi`
/// calls in the app.
class ShareDetailScreen extends StatefulWidget {
  const ShareDetailScreen({
    super.key,
    required this.share,
    required this.loadManifest,
    required this.loadGrants,
    this.onNavigate,
    this.currentTab = 4,
  });

  final MobileShare share;
  final Future<ShareManifest?> Function(String shareId) loadManifest;
  final Future<List<MobileShareGrant>> Function(String shareId) loadGrants;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  State<ShareDetailScreen> createState() => _ShareDetailScreenState();
}

class _ShareDetailScreenState extends State<ShareDetailScreen> {
  ShareManifest? _manifest;
  List<MobileShareGrant> _grants = const [];
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    unawaited(_reload());
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final manifest = await widget.loadManifest(widget.share.shareId);
      final grants = await widget.loadGrants(widget.share.shareId);
      if (!mounted) return;
      setState(() {
        _manifest = manifest;
        _grants = grants;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = e.toString();
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.share.name),
        actions: [
          IconButton(
            key: const Key('share-detail-refresh'),
            icon: const Icon(Icons.refresh),
            tooltip: 'Refresh',
            onPressed: _reload,
          ),
        ],
      ),
      bottomNavigationBar: widget.onNavigate == null
          ? null
          : MainTabNavBar(
              currentIndex: widget.currentTab,
              onSelect: (i) {
                Navigator.pop(context);
                widget.onNavigate!(i);
              },
            ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _error != null
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(_error!, textAlign: TextAlign.center),
                    const SizedBox(height: 12),
                    FilledButton(
                      onPressed: _reload,
                      child: const Text('Retry'),
                    ),
                  ],
                ),
              ),
            )
          : ListView(
              padding: const EdgeInsets.all(16),
              children: [
                _section('Overview', [
                  _row('Share ID', widget.share.shareId),
                  _row('Kind', widget.share.kind),
                  _row('Owner', widget.share.ownerUserId),
                  _row('Revision', '${widget.share.revision.toInt()}'),
                  _row('Public write', widget.share.publicWrite ? 'yes' : 'no'),
                  _row(
                    'Local root',
                    widget.share.localRoot ?? 'Not bound on this device',
                  ),
                ]),
                if (widget.share.kind == 'private')
                  _section(
                    'Grants',
                    _grants.isEmpty
                        ? const [Text('No explicit grants.')]
                        : _grants.map((g) => _row(g.userId, g.mode)).toList(),
                  ),
                _section(
                  'Manifest files',
                  _manifest == null
                      ? const [Text('No manifest received yet.')]
                      : _manifest!.entries.isEmpty
                      ? const [Text('The manifest is empty.')]
                      : _manifest!.entries
                            .map(
                              (entry) => ListTile(
                                leading: const Icon(Icons.insert_drive_file),
                                title: Text(entry.path),
                                subtitle: Text(
                                  '${entry.size.toInt()} B · '
                                  '${_shortId(entry.fileId)}',
                                ),
                              ),
                            )
                            .toList(),
                ),
              ],
            ),
    );
  }

  Widget _section(String title, List<Widget> children) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(title, style: Theme.of(context).textTheme.titleSmall),
            const SizedBox(height: 12),
            ...children,
          ],
        ),
      ),
    );
  }

  Widget _row(String label, String value) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(label, style: Theme.of(context).textTheme.labelSmall),
          const SizedBox(height: 2),
          SelectableText(value),
        ],
      ),
    );
  }

  /// Content addresses are 64 hex characters; keep the subtitle readable.
  String _shortId(String id) => id.length > 8 ? '${id.substring(0, 8)}…' : id;
}
