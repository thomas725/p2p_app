import 'dart:async' show unawaited;

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart'
    show MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareManifest, ShareOffer;
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/share_network.dart';
import 'package:p2p_app_flutter/src/screens/share_publish.dart';

/// List of this install's file shares. The list itself is read-only; creation,
/// binding and native folder snapshots stay in Rust (see `docs/backlog.md`).
/// Both the FRB and browser transports can inspect shares and submit a
/// revision of a loaded manifest.
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
    this.onDiscoverShares,
    this.onNavigate,
    this.currentTab = 4,
  });

  final List<MobileShare> shares;
  final void Function(MobileShare) onOpenShare;
  final void Function()? onDiscoverShares;
  final void Function(int)? onNavigate;
  final int currentTab;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('File shares'),
        actions: [
          if (onDiscoverShares != null)
            IconButton(
              key: const Key('shares-discover'),
              icon: const Icon(Icons.search),
              tooltip: 'Discover peer shares',
              onPressed: onDiscoverShares,
            ),
        ],
      ),
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

/// Detail route: share metadata, per-user grants and the files in the current
/// manifest. Loaders are injected (like `PeerInfoScreen`) so the page needs no
/// Rust library in widget tests; Home supplies the `NodeApi` calls in the app.
/// The grant editor, network actions and publish editor are optional: with no
/// callbacks the page stays read-only, and with them it uses the same facade
/// over both FRB and web transports.
class ShareDetailScreen extends StatefulWidget {
  const ShareDetailScreen({
    super.key,
    required this.share,
    required this.loadManifest,
    required this.loadGrants,
    this.onSetGrant,
    this.onRemoveGrant,
    this.peerIds = const [],
    this.onRequestManifest,
    this.onReadvertise,
    this.onPublish,
    this.onNavigate,
    this.currentTab = 4,
  });

  final MobileShare share;
  final Future<ShareManifest?> Function(String shareId) loadManifest;
  final Future<List<MobileShareGrant>> Function(String shareId) loadGrants;

  /// Optional permission editor seams. When omitted the page stays purely
  /// read-only (widget tests and transports that cannot edit a share).
  final Future<void> Function(String shareId, String userId, String mode)?
  onSetGrant;
  final Future<void> Function(String shareId, String userId)? onRemoveGrant;

  /// Optional network seams. Both are needed for [ShareNetworkSection] to
  /// appear; Home supplies the currently connected peer IDs alongside them.
  final List<String> peerIds;
  final Future<void> Function(String shareId, String peerId)? onRequestManifest;
  final Future<void> Function(String peerId, ShareOffer offer)? onReadvertise;
  final Future<void> Function(
    String peerId,
    BigInt baseRevision,
    ShareManifest manifest,
  )?
  onPublish;
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
  String? _actionError;

  @override
  void initState() {
    super.initState();
    unawaited(_reload());
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _error = null;
      _actionError = null;
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
                if (widget.share.kind == 'private') _grantsSection(),
                if (widget.onRequestManifest != null &&
                    widget.onReadvertise != null)
                  ShareNetworkSection(
                    share: widget.share,
                    peerIds: widget.peerIds,
                    onRequestManifest: widget.onRequestManifest!,
                    onReadvertise: widget.onReadvertise!,
                    onCompleted: _reload,
                  ),
                if (widget.onPublish != null)
                  SharePublishSection(
                    share: widget.share,
                    manifest: _manifest,
                    peerIds: widget.peerIds,
                    onPublish: widget.onPublish!,
                    onCompleted: _reload,
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

  /// Private-share grants, with revoke/add controls only when Home supplied
  /// the corresponding `NodeApi` callbacks. Public shares have no ACL rows.
  Widget _grantsSection() {
    final children = <Widget>[];
    if (_grants.isEmpty) {
      children.add(const Text('No explicit grants.'));
    } else {
      for (final grant in _grants) {
        children.add(
          Row(
            key: Key('share-grant-${grant.userId}'),
            children: [
              Expanded(child: _row(grant.userId, grant.mode)),
              if (widget.onRemoveGrant != null)
                IconButton(
                  key: Key('share-grant-revoke-${grant.userId}'),
                  icon: const Icon(Icons.person_remove_outlined),
                  tooltip: 'Revoke ${grant.userId}',
                  onPressed: () => _removeGrant(grant),
                ),
            ],
          ),
        );
      }
    }
    if (widget.onSetGrant != null) {
      children.add(
        Align(
          alignment: Alignment.centerLeft,
          child: FilledButton.tonalIcon(
            key: const Key('share-grant-add'),
            icon: const Icon(Icons.person_add_outlined),
            label: const Text('Add grant'),
            onPressed: _showAddGrantDialog,
          ),
        ),
      );
    }
    if (_actionError != null) {
      children.add(
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: Text(
            _actionError!,
            key: const Key('share-grant-error'),
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ),
      );
    }
    return _section('Grants', children);
  }

  Future<void> _removeGrant(MobileShareGrant grant) async {
    final remove = widget.onRemoveGrant;
    if (remove == null) return;
    setState(() => _actionError = null);
    try {
      await remove(widget.share.shareId, grant.userId);
      if (!mounted) return;
      await _reload();
    } catch (e) {
      if (!mounted) return;
      setState(() => _actionError = 'Failed to revoke ${grant.userId}: $e');
    }
  }

  Future<void> _showAddGrantDialog() async {
    final setGrant = widget.onSetGrant;
    if (setGrant == null) return;
    final added = await showDialog<bool>(
      context: context,
      builder: (_) => _AddGrantDialog(
        onSubmit: (userId, mode) =>
            setGrant(widget.share.shareId, userId, mode),
      ),
    );
    if (added == true && mounted) await _reload();
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

/// Adds one grant to a private share. The user ID must be a verified account
/// ID known to Rust; only the canonical `readonly`/`readwrite` modes are
/// offered because `parse_share_mode` rejects anything else.
class _AddGrantDialog extends StatefulWidget {
  const _AddGrantDialog({required this.onSubmit});

  final Future<void> Function(String userId, String mode) onSubmit;

  @override
  State<_AddGrantDialog> createState() => _AddGrantDialogState();
}

class _AddGrantDialogState extends State<_AddGrantDialog> {
  final _controller = TextEditingController();
  String _mode = 'readonly';
  bool _submitting = false;
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final userId = _controller.text.trim();
    if (userId.isEmpty || _submitting) return;
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      await widget.onSubmit(userId, _mode);
      if (mounted) Navigator.of(context).pop(true);
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _submitting = false;
        _error = 'Failed to add grant: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Add share grant'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            key: const Key('share-grant-user-id'),
            controller: _controller,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Verified user ID',
              hintText: 'user-…',
            ),
            enabled: !_submitting,
            onSubmitted: (_) => _submit(),
          ),
          const SizedBox(height: 12),
          DropdownButtonFormField<String>(
            key: const Key('share-grant-mode'),
            initialValue: _mode,
            decoration: const InputDecoration(labelText: 'Access'),
            items: const [
              DropdownMenuItem(value: 'readonly', child: Text('readonly')),
              DropdownMenuItem(value: 'readwrite', child: Text('readwrite')),
            ],
            onChanged: _submitting
                ? null
                : (mode) {
                    if (mode != null) setState(() => _mode = mode);
                  },
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(
                _error!,
                key: const Key('share-grant-dialog-error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
      actions: [
        TextButton(
          key: const Key('share-grant-cancel'),
          onPressed: _submitting ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('share-grant-submit'),
          onPressed: _submitting ? null : _submit,
          child: _submitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Add grant'),
        ),
      ],
    );
  }
}
