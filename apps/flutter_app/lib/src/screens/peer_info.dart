import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/dm_chat.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/ui_utils.dart';

/// Full-screen peer detail page (name origin, activity), with a shortcut to
/// open the DM chat for the peer and — when [onEditLocalNickname] is supplied —
/// an action to set the local nickname this install shows for the peer.
class PeerInfoScreen extends StatefulWidget {
  const PeerInfoScreen({
    super.key,
    required this.peer,
    required this.serviceRunning,
    this.onNavigate,
    this.currentTab = 1,
    this.onCall,
    this.onEditLocalNickname,
  });
  final MobilePeerRecord peer;
  final bool serviceRunning;
  final void Function(int)? onNavigate;
  final int currentTab;

  /// Places an outgoing call to this peer. Optional: when null, no call action
  /// is rendered.
  final void Function(MobilePeerRecord peer)? onCall;

  /// Persists a per-peer local nickname and returns the refreshed record (or
  /// null when the peer is no longer in the roster). Optional: when null, no
  /// nickname-edit action is rendered.
  final Future<MobilePeerRecord?> Function(MobilePeerRecord peer, String nickname)?
  onEditLocalNickname;

  @override
  State<PeerInfoScreen> createState() => _PeerInfoScreenState();
}

class _PeerInfoScreenState extends State<PeerInfoScreen> {
  /// Nickname set during this visit, and the refreshed record if the roster
  /// answered — so the page reflects the edit without being reopened.
  String? _localNickname;
  MobilePeerRecord? _refreshed;

  MobilePeerRecord get _peer => _refreshed ?? widget.peer;

  String get _displayName => _refreshed?.displayName ?? _localNickname ?? widget.peer.displayName;

  String? get _effectiveLocalNickname =>
      _refreshed?.localNickname ?? _localNickname ?? widget.peer.localNickname;

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

  Future<void> _editNickname() async {
    final save = widget.onEditLocalNickname;
    if (save == null) return;
    // The dialog performs the save (so a rejected nickname keeps it open) and
    // pops with `(nickname, refreshed record)` on success.
    final result = await showDialog<(String, MobilePeerRecord?)>(
      context: context,
      builder: (_) => _NicknameDialog(
        initial: _effectiveLocalNickname ?? '',
        onSave: (name) => save(widget.peer, name),
      ),
    );
    if (result == null || !mounted) return;
    setState(() {
      _localNickname = result.$1;
      if (result.$2 != null) _refreshed = result.$2;
    });
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text('Local nickname set to "${result.$1}".')),
    );
  }

  @override
  Widget build(BuildContext context) {
    final peer = _peer;
    final localNickname = _effectiveLocalNickname;
    final (origin, originDetail) = _origin(
      MobilePeerRecord(
        peerId: peer.peerId,
        firstSeen: peer.firstSeen,
        lastSeen: peer.lastSeen,
        nickname: peer.nickname,
        localNickname: localNickname,
        displayName: _displayName,
      ),
    );
    return Scaffold(
      appBar: AppBar(
        title: Text(_displayName),
        actions: [
          if (widget.onEditLocalNickname != null)
            IconButton(
              key: const Key('edit-nickname-peer-info'),
              icon: const Icon(Icons.edit),
              tooltip: 'Set local nickname',
              onPressed: _editNickname,
            ),
          if (widget.onCall != null)
            IconButton(
              key: const Key('call-peer-info'),
              icon: const Icon(Icons.phone),
              tooltip: 'Call',
              onPressed: () => widget.onCall!(peer),
            ),
        ],
      ),
      bottomNavigationBar: MainTabNavBar(
        currentIndex: widget.currentTab,
        onSelect: (i) {
          Navigator.pop(context);
          widget.onNavigate?.call(i);
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
                    _displayName,
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
                  if (localNickname != null) ...[
                    _infoRow('Local nickname', localNickname),
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
                  serviceRunning: widget.serviceRunning,
                  onNavigate: widget.onNavigate,
                  currentTab: widget.currentTab,
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

/// Modal dialog for setting a per-peer local nickname. Submitting calls
/// [onSave] (which persists through Rust and returns the refreshed record) and
/// pops with the nickname on success, or shows an error snackbar (and stays
/// open) on failure. Validation is Rust's (`setPeerLocalNickname` rejects
/// names the canonical rules disallow), so the rules live in one place.
class _NicknameDialog extends StatefulWidget {
  const _NicknameDialog({required this.initial, required this.onSave});
  final String initial;
  final Future<MobilePeerRecord?> Function(String nickname) onSave;

  @override
  State<_NicknameDialog> createState() => _NicknameDialogState();
}

class _NicknameDialogState extends State<_NicknameDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initial,
  );
  bool _submitting = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit() async {
    final name = _controller.text.trim();
    if (name.isEmpty || name == widget.initial) {
      Navigator.of(context).pop();
      return;
    }
    setState(() => _submitting = true);
    try {
      final refreshed = await widget.onSave(name);
      if (mounted) Navigator.of(context).pop((name, refreshed));
    } catch (e) {
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Invalid nickname: $e')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Set local nickname'),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: const InputDecoration(
          labelText: 'Nickname',
          hintText: 'letters, digits and dashes',
        ),
        onSubmitted: (_) => _submitting ? null : _submit(),
        enabled: !_submitting,
      ),
      actions: [
        TextButton(
          onPressed: _submitting ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _submitting ? null : _submit,
          child: _submitting
              ? const SizedBox(
                  width: 16,
                  height: 16,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : const Text('Save'),
        ),
      ],
    );
  }
}
