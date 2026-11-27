import 'dart:async' show unawaited;

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart'
    show MobileShare, MobileShareGrant;
import 'package:p2p_app_flutter/src/rust/shares.dart' show ShareManifest;
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/screens/share_network.dart';
import 'package:p2p_app_flutter/src/screens/share_publish.dart';

/// Peer IDs offered by share network/publish pickers.
///
/// Home tracks connected peers in arrival order; the TUI sorts the same set by
/// peer ID (`connected::sorted_connected_peer_ids`). Sorting here keeps both
/// frontends deterministic and prevents a known-but-disconnected peer from
/// being offered for an action that requires a live connection.
List<String> connectedShareActionPeerIds(Iterable<String> connectedPeerIds) {
  final ids = connectedPeerIds.toSet().toList()..sort();
  return ids;
}

/// List of this install's file shares. An empty private share can be created
/// here; binding a local folder and native folder snapshots stay on native
/// installs (see `docs/backlog.md`). Both transports can inspect shares,
/// discover and accept peer offers, re-advertise manifests and publish deletion
/// revisions.
class ShareList extends StatefulWidget {
  const ShareList({
    super.key,
    required this.shares,
    required this.onOpenShare,
    this.onCreateShare,
  });

  final List<MobileShare> shares;
  final void Function(MobileShare) onOpenShare;

  /// Optional creation seam. Home supplies it on both transports; hiding the
  /// action when absent keeps read-only embeds honest.
  final Future<void> Function(String name)? onCreateShare;

  @override
  State<ShareList> createState() => _ShareListState();
}

class _ShareListState extends State<ShareList> {
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
                'Shares (${widget.shares.length})',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(color: cs.onPrimaryContainer),
              ),
              const Spacer(),
              if (widget.onCreateShare != null)
                IconButton(
                  key: const Key('shares-create'),
                  icon: const Icon(Icons.add),
                  tooltip: 'Create private share',
                  color: cs.onPrimaryContainer,
                  onPressed: _showCreateDialog,
                ),
            ],
          ),
        ),
        Expanded(
          child: widget.shares.isEmpty
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
                        'No shares yet. Create one here, or wait for a peer '
                        'share to be accepted.',
                        textAlign: TextAlign.center,
                        style: TextStyle(color: Colors.grey[500]),
                      ),
                    ],
                  ),
                )
              : ListView.separated(
                  itemCount: widget.shares.length,
                  separatorBuilder: (_, _) => const Divider(height: 1),
                  itemBuilder: (context, i) {
                    final share = widget.shares[i];
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
                      onTap: () => widget.onOpenShare(share),
                    );
                  },
                ),
        ),
      ],
    );
  }

  Future<void> _showCreateDialog() async {
    final create = widget.onCreateShare;
    if (create == null) return;
    await showDialog<bool>(
      context: context,
      builder: (_) => _CreateShareDialog(onSubmit: create),
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
    this.onCreateShare,
    this.onDiscoverShares,
    this.onNavigate,
    this.currentTab = 4,
  });

  final List<MobileShare> shares;
  final void Function(MobileShare) onOpenShare;
  final Future<void> Function(String name)? onCreateShare;
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
      body: ShareList(
        shares: shares,
        onOpenShare: onOpenShare,
        onCreateShare: onCreateShare,
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
    this.ownUserId,
    this.onSetVisibility,
    this.onSetGrant,
    this.onRemoveGrant,
    this.peerIds = const [],
    this.onRequestManifest,
    this.onReadvertise,
    this.onReviseManifest,
    this.onPublish,
    this.onNavigate,
    this.currentTab = 4,
  });

  final MobileShare share;
  final Future<ShareManifest?> Function(String shareId) loadManifest;
  final Future<List<MobileShareGrant>> Function(String shareId) loadGrants;

  /// This install's user id, used only to decide whether the local visibility
  /// controls apply. Rust repeats the ownership check before every write.
  final String? ownUserId;

  /// Optional visibility editor seam. Home supplies it so both transports use
  /// the same Rust-owned `private`/`public` + `publicWrite` setter.
  final Future<MobileShare> Function(
    String shareId,
    String kind,
    bool publicWrite,
  )?
  onSetVisibility;

  /// Optional permission editor seams. When omitted the page stays purely
  /// read-only (widget tests and transports that cannot edit a share).
  final Future<void> Function(String shareId, String userId, String mode)?
  onSetGrant;
  final Future<void> Function(String shareId, String userId)? onRemoveGrant;

  /// Optional network seams. Both are needed for [ShareNetworkSection] to
  /// appear; Home supplies the currently connected peer IDs alongside them.
  final List<String> peerIds;
  final Future<void> Function(String shareId, String peerId)? onRequestManifest;
  final Future<void> Function(String shareId, String peerId)? onReadvertise;

  /// Both publish seams are needed before the deletion editor appears. Rust
  /// builds the revised manifest; this transport only submits it to a peer.
  final Future<ShareManifest> Function(
    String shareId,
    List<String> removedPaths,
  )?
  onReviseManifest;
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
  late MobileShare _share;
  ShareManifest? _manifest;
  List<MobileShareGrant> _grants = const [];
  bool _loading = true;
  String? _error;
  String? _actionError;
  String? _visibilityError;

  @override
  void initState() {
    super.initState();
    _share = widget.share;
    unawaited(_reload());
  }

  Future<void> _reload() async {
    setState(() {
      _loading = true;
      _error = null;
      _actionError = null;
    });
    try {
      final manifest = await widget.loadManifest(_share.shareId);
      final grants = await widget.loadGrants(_share.shareId);
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
        title: Text(_share.name),
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
                  _row('Share ID', _share.shareId),
                  _row('Kind', _share.kind),
                  _row('Owner', _share.ownerUserId),
                  _row('Revision', '${_share.revision.toInt()}'),
                  _row('Public write', _share.publicWrite ? 'yes' : 'no'),
                  _row(
                    'Local root',
                    _share.localRoot ?? 'Not bound on this device',
                  ),
                ]),
                if (_canEditVisibility) _visibilitySection(),
                if (_share.kind == 'private') _grantsSection(),
                if (widget.onRequestManifest != null &&
                    widget.onReadvertise != null)
                  ShareNetworkSection(
                    share: _share,
                    peerIds: widget.peerIds,
                    onRequestManifest: widget.onRequestManifest!,
                    onReadvertise: widget.onReadvertise!,
                    onCompleted: _reload,
                  ),
                if (widget.onReviseManifest != null && widget.onPublish != null)
                  SharePublishSection(
                    share: _share,
                    manifest: _manifest,
                    peerIds: widget.peerIds,
                    onReviseManifest: widget.onReviseManifest!,
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

  /// Visibility controls are owner-only; Rust repeats the same check so a
  /// stale UI cannot reclassify a peer-owned share.
  bool get _canEditVisibility =>
      widget.onSetVisibility != null &&
      widget.ownUserId != null &&
      widget.ownUserId == _share.ownerUserId;

  Widget _visibilitySection() {
    final isPrivate = _share.kind == 'private';
    final children = <Widget>[
      Wrap(
        spacing: 8,
        children: [
          FilledButton(
            key: const Key('share-visibility-private'),
            onPressed: isPrivate
                ? null
                : () => _setVisibility('private', false),
            child: const Text('Private'),
          ),
          FilledButton.tonal(
            key: const Key('share-visibility-public'),
            onPressed: !isPrivate
                ? null
                : () => _setVisibility('public', false),
            child: const Text('Public'),
          ),
        ],
      ),
      if (!isPrivate)
        SwitchListTile(
          key: const Key('share-public-write'),
          title: const Text('Allow anonymous writes'),
          value: _share.publicWrite,
          onChanged: (value) => _setVisibility('public', value),
        ),
      if (_visibilityError != null)
        Padding(
          padding: const EdgeInsets.only(top: 8),
          child: Text(
            _visibilityError!,
            key: const Key('share-visibility-error'),
            style: TextStyle(color: Theme.of(context).colorScheme.error),
          ),
        ),
    ];
    return _section('Access', children);
  }

  Future<void> _setVisibility(String kind, bool publicWrite) async {
    final setter = widget.onSetVisibility;
    if (setter == null) return;
    setState(() => _visibilityError = null);
    try {
      final updated = await setter(_share.shareId, kind, publicWrite);
      if (!mounted) return;
      setState(() => _share = updated);
    } catch (e) {
      if (!mounted) return;
      setState(() => _visibilityError = 'Failed to change visibility: $e');
    }
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
      await remove(_share.shareId, grant.userId);
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
        onSubmit: (userId, mode) => setGrant(_share.shareId, userId, mode),
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

/// Creates an empty private share. Rust validates the name and resolves the
/// local owner through the same facade the TUI uses; this dialog only collects
/// the name and shows the error Rust returns. No local root is supplied here:
/// binding a folder is a separate native-only action, so both transports
/// create the same rootless share.
class _CreateShareDialog extends StatefulWidget {
  const _CreateShareDialog({required this.onSubmit});

  final Future<void> Function(String name) onSubmit;

  @override
  State<_CreateShareDialog> createState() => _CreateShareDialogState();
}

class _CreateShareDialogState extends State<_CreateShareDialog> {
  final _controller = TextEditingController();
  bool _submitting = false;
  String? _error;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final name = _controller.text.trim();
    if (name.isEmpty || _submitting) return;
    setState(() {
      _submitting = true;
      _error = null;
    });
    try {
      await widget.onSubmit(name);
      if (mounted) Navigator.of(context).pop(true);
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _submitting = false;
        _error = 'Failed to create share: $e';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Create private share'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            key: const Key('share-create-name'),
            controller: _controller,
            autofocus: true,
            decoration: const InputDecoration(
              labelText: 'Share name',
              hintText: 'Holiday photos',
            ),
            enabled: !_submitting,
            onSubmitted: (_) => _submit(),
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(
                _error!,
                key: const Key('share-create-dialog-error'),
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ),
        ],
      ),
      actions: [
        TextButton(
          key: const Key('share-create-cancel'),
          onPressed: _submitting ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          key: const Key('share-create-submit'),
          onPressed: _submitting ? null : _submit,
          child: _submitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Create'),
        ),
      ],
    );
  }
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
