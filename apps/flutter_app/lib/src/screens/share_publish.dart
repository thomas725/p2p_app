import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart' show MobileShare;
import 'package:p2p_app_flutter/src/rust/shares.dart'
    show ShareEntry, ShareManifest;

/// Build the snapshot submitted to a share's owner after removing files.
///
/// `null` means "nothing publishable": no selection, a corrupt base revision,
/// or an empty share id. Rust remains the authority for path/file-id validation
/// and for the ACL, so this helper only changes the entry list and advances the
/// optimistic-concurrency revision by one.
ShareManifest? revisedManifestForDeletion(
  ShareManifest base,
  Set<String> removedPaths,
) {
  if (base.shareId.isEmpty || base.revision <= BigInt.zero) {
    return null;
  }
  final entries = base.entries
      .where((entry) => !removedPaths.contains(entry.path))
      .toList(growable: false);
  if (entries.length == base.entries.length) {
    return null;
  }
  return ShareManifest(
    shareId: base.shareId,
    name: base.name,
    revision: base.revision + BigInt.one,
    entries: entries,
  );
}

/// Publish a deletion back to the peer that owns a share.
///
/// The manifest loaded in the detail route is the base revision. The selected
/// peer is only a destination: Rust checks its verified user id against the
/// share's ACL and rejects a stale `baseRevision` atomically. A browser can
/// publish a deletion because it needs no new filesystem snapshot; adding new
/// files stays a native snapshot/binding operation.
class SharePublishSection extends StatefulWidget {
  const SharePublishSection({
    super.key,
    required this.share,
    required this.manifest,
    required this.peerIds,
    required this.onPublish,
    this.onCompleted,
  });

  final MobileShare share;
  final ShareManifest? manifest;
  final List<String> peerIds;
  final Future<void> Function(
    String peerId,
    BigInt baseRevision,
    ShareManifest manifest,
  )
  onPublish;

  /// Called after a successful submission so the detail page reloads the
  /// manifest and share row Rust received from the owner.
  final Future<void> Function()? onCompleted;

  @override
  State<SharePublishSection> createState() => _SharePublishSectionState();
}

class _SharePublishSectionState extends State<SharePublishSection> {
  final Set<String> _removed = <String>{};
  String? _peerId;
  bool _busy = false;
  String? _error;
  String? _status;

  @override
  void initState() {
    super.initState();
    _peerId = _firstPeer();
  }

  @override
  void didUpdateWidget(covariant SharePublishSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    final manifestChanged = oldWidget.manifest != widget.manifest;
    final nextPeer = _peerId == null || !widget.peerIds.contains(_peerId)
        ? _firstPeer()
        : _peerId;
    if (manifestChanged || nextPeer != _peerId) {
      setState(() {
        if (manifestChanged) {
          _removed.clear();
          _error = null;
        }
        _peerId = nextPeer;
      });
    }
  }

  String? _firstPeer() => widget.peerIds.isEmpty ? null : widget.peerIds.first;

  /// Editing a manifest whose cached revision is not the share row's revision
  /// would ask the owner to accept a write based on a snapshot we no longer
  /// have. Refreshing first is the only safe path.
  bool _matchesShare(ShareManifest manifest) =>
      manifest.shareId == widget.share.shareId &&
      manifest.revision == BigInt.from(widget.share.revision.toInt());

  void _toggle(String path, bool? selected) {
    setState(() {
      if (selected == true) {
        _removed.add(path);
      } else {
        _removed.remove(path);
      }
      _error = null;
    });
  }

  Future<void> _publish() async {
    final base = widget.manifest;
    final peerId = _peerId;
    if (base == null || peerId == null || _busy || !_matchesShare(base)) {
      return;
    }
    final revised = revisedManifestForDeletion(base, _removed);
    if (revised == null) return;

    setState(() {
      _busy = true;
      _error = null;
      _status = null;
    });
    try {
      await widget.onPublish(peerId, base.revision, revised);
      if (!mounted) return;
      setState(() {
        _removed.clear();
        _status =
            'Submitted revision ${revised.revision.toInt()}; '
            'the owner may still reject it.';
      });
      await widget.onCompleted?.call();
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = 'Publish failed: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final manifest = widget.manifest;
    final usable = manifest != null && _matchesShare(manifest);

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              'Publish revision',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 12),
            if (manifest == null)
              const Text('No manifest received yet. Refresh before publishing.')
            else if (!usable)
              Text(
                'Manifest is stale for ${widget.share.shareId}. Refresh before '
                'publishing.',
                key: const Key('share-publish-stale'),
                style: TextStyle(color: cs.error),
              )
            else ...[
              if (manifest.entries.isEmpty)
                const Text('The manifest is empty; there is nothing to remove.')
              else ...[
                const Text(
                  'Select files to remove from the owner\'s manifest:',
                ),
                for (final entry in manifest.entries) _entryRow(entry),
              ],
              const SizedBox(height: 8),
              if (widget.peerIds.isEmpty)
                const Text('No connected peers. Publishing needs a peer.')
              else
                DropdownButtonFormField<String>(
                  key: const Key('share-publish-peer'),
                  initialValue: _peerId,
                  decoration: const InputDecoration(
                    labelText: 'Owner peer',
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
                key: const Key('share-publish-submit'),
                icon: const Icon(Icons.publish),
                label: const Text('Publish deletion'),
                onPressed: _busy || _removed.isEmpty || widget.peerIds.isEmpty
                    ? null
                    : _publish,
              ),
            ],
            if (_status != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(
                  _status!,
                  key: const Key('share-publish-status'),
                  style: TextStyle(color: cs.primary),
                ),
              ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 12),
                child: Text(
                  _error!,
                  key: const Key('share-publish-error'),
                  style: TextStyle(color: cs.error),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _entryRow(ShareEntry entry) => CheckboxListTile(
    key: Key('share-publish-delete-${entry.path}'),
    value: _removed.contains(entry.path),
    title: Text(entry.path),
    subtitle: Text('${entry.size.toInt()} B'),
    enabled: !_busy,
    onChanged: (selected) => _toggle(entry.path, selected),
  );
}
