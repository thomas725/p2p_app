import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';

/// Test-only seam for [ContactListState]: replaces the FFI-backed `sortContacts`
/// call so widget tests can exercise sorting without a loaded Rust library.
@visibleForTesting
typedef SortContactsOverride =
    List<ContactSortInput> Function(
      List<ContactSortInput> rows,
      int column,
      bool ascending,
    );

/// Contacts tab: a sortable table (Rust `sortContacts`-backed) of roster
/// contacts. One row per contact — a single discovered device or the aggregated
/// best-device row of a multi-device user. Multi-device contacts expand into
/// their device sub-rows (one per bound device), mirroring the TUI's expandable
/// plan; Peer-info and Direct-message actions operate on the row's exact device.
class ContactList extends StatefulWidget {
  const ContactList({
    super.key,
    required this.rows,
    required this.onOpenInfo,
    required this.onOpenDm,
    this.onLinkDevice,
    this.onCallDevice,
    this.onEditContactNickname,
    this.ownUserId,
    this.blockedPeerIds = const <String>{},
    this.sortOverride,
  });
  final List<MobileContactRow> rows;
  final void Function(MobileContactRow contact, MobileContactDevice device)
  onOpenInfo;
  final void Function(MobileContactRow contact, MobileContactDevice device)
  onOpenDm;

  /// Device-scoped action that sends our device-binding intro to the device's
  /// peer (so its account holder can bind us into their account). Optional:
  /// when null, no link button is rendered.
  final void Function(MobileContactDevice device)? onLinkDevice;

  /// Device-scoped action that places an outgoing call to the device. Optional:
  /// when null, no call button is rendered.
  final void Function(MobileContactDevice device)? onCallDevice;

  /// Owner-side nickname editor for a bound user contact. Optional: when null,
  /// no nickname button is rendered. Unbound device rows never get one — the
  /// override belongs to the contact (user), not to a device.
  final Future<void> Function(MobileContactRow contact, String? nickname)?
  onEditContactNickname;

  /// This install's user id. A sibling device is a user contact row, but the
  /// Rust writer refuses self nicknames, so no editor is offered for it.
  final String? ownUserId;

  /// Peer ids that are muted (on the blocklist). A contact row is marked when
  /// **every** one of its devices is muted; otherwise only the blocked device
  /// sub-rows carry the marker — the same rule as the TUI's `⊘` marker.
  /// Muted contacts stay listed; existing history is kept.
  final Set<String> blockedPeerIds;

  /// Test-only seam: when set, replaces the FFI-backed `sortContacts` call so
  /// widget tests can exercise sorting without a loaded Rust library.
  @visibleForTesting
  final SortContactsOverride? sortOverride;

  @override
  State<ContactList> createState() => ContactListState();
}

class ContactListState extends State<ContactList> {
  static const int _kLastSeen = 3;

  /// Counter cell for a device sub-row: per-device DM/broadcast stats aren't
  /// aggregated, so device rows show a dash (mirrors the TUI's rendered "–").
  static const String _kDeviceStatPlaceholder = '\u2013';

  final TextEditingController _searchController = TextEditingController();
  String _query = '';

  int _sortColumn = _kLastSeen;
  bool _ascending = false;

  /// Last resolved sort order with the input key it was computed for. Rust
  /// still owns the ordering (`nodeApi.sortContacts`); this only caches the
  /// resolved rows so builds stay synchronous, re-sorting when the inputs or
  /// sort params change.
  List<MobileContactRow>? _sortedCache;
  String? _sortedCacheKey;

  /// Contact ids whose device sub-rows are currently expanded (kept across
  /// re-sorts; sorted input only reorders, never drops, contacts).
  final Set<String> _expanded = {};

  int _dmCount(MobileContactRow c) => c.dmCount.toInt();
  int _broadcastCount(MobileContactRow c) => c.broadcastSentToPeer.toInt();

  /// Contacts matching the active query (empty = no filter).
  ///
  /// Case-insensitive substring match over the contact/device ids and display
  /// names, mirroring the TUI's `contact_matches` so searching a peer id finds
  /// a contact shown by its grouped user name. Filtering happens before the
  /// Rust sort, which stays the single ordering authority.
  List<MobileContactRow> _visibleRows() {
    if (_query.isEmpty) return widget.rows;
    final q = _query.toLowerCase();
    bool has(String s) => s.toLowerCase().contains(q);
    return widget.rows
        .where(
          (c) =>
              has(c.contactId) ||
              has(c.displayName) ||
              c.deviceRows.any((d) => has(d.peerId) || has(d.displayName)),
        )
        .toList();
  }

  /// Display name, with a ` ×N` device-count suffix when the contact spans
  /// multiple bound devices (mirrors the TUI render-only suffix).
  String _label(MobileContactRow c) => c.deviceCount.toInt() > 1
      ? '${c.displayName} \u00d7${c.deviceCount}'
      : c.displayName;

  bool _isMuted(String peerId) => widget.blockedPeerIds.contains(peerId);

  /// A contact row is muted only when **every** bound device is muted; a
  /// partially muted contact marks its device sub-rows instead (TUI parity).
  bool _contactIsMuted(MobileContactRow c) =>
      c.deviceRows.isNotEmpty && c.deviceRows.every((d) => _isMuted(d.peerId));

  /// The muted marker for a row (nothing when the row isn't muted).
  Widget _mutedMarker(String keySuffix, bool muted) {
    if (!muted) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(right: 4),
      child: Tooltip(
        message: 'Muted',
        child: Icon(Icons.block, key: Key('muted-$keySuffix'), size: 16),
      ),
    );
  }

  @override
  void initState() {
    super.initState();
    unawaited(_resort());
  }

  @override
  void didUpdateWidget(covariant ContactList oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.rows, widget.rows)) {
      unawaited(_resort());
    }
  }

  void _toggleExpand(String contactId) {
    setState(() {
      if (!_expanded.remove(contactId)) {
        _expanded.add(contactId);
      }
    });
  }

  Widget _expandButton(MobileContactRow c) {
    final expanded = _expanded.contains(c.contactId);
    final chevron = Icon(
      expanded ? Icons.keyboard_arrow_down : Icons.keyboard_arrow_right,
      size: 20,
    );
    return Padding(
      padding: const EdgeInsets.only(right: 4),
      child: InkWell(
        key: Key('expand-${c.contactId}'),
        onTap: () => _toggleExpand(c.contactId),
        child: Tooltip(
          message: expanded ? 'Collapse' : 'Expand',
          child: chevron,
        ),
      ),
    );
  }

  /// Open the owner-side nickname dialog and refresh through [onSave] on a
  /// successful write. Rust owns validation and display-name resolution; this
  /// widget only collects the text and reports failures.
  Future<void> _editContactNickname(
    MobileContactRow contact,
    Future<void> Function(String? nickname) onSave,
  ) async {
    final saved = await showDialog<bool>(
      context: context,
      builder: (_) => _ContactNicknameDialog(onSave: onSave),
    );
    if (saved == true && mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(content: Text('Contact nickname updated.')),
      );
    }
  }

  Widget _actionButtons(
    String keyPrefix,
    MobileContactRow c,
    MobileContactDevice device, {
    Future<void> Function(String? nickname)? onEditNickname,
  }) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        IconButton(
          key: Key('info-$keyPrefix'),
          icon: const Icon(Icons.info_outline),
          tooltip: 'Peer info',
          onPressed: () => widget.onOpenInfo(c, device),
        ),
        IconButton(
          key: Key('dm-$keyPrefix'),
          icon: const Icon(Icons.chat),
          tooltip: 'Direct message',
          onPressed: () => widget.onOpenDm(c, device),
        ),
        if (onEditNickname != null)
          IconButton(
            key: Key('edit-contact-nickname-${c.contactId}'),
            icon: const Icon(Icons.edit),
            tooltip: 'Set contact nickname',
            onPressed: () => _editContactNickname(c, onEditNickname),
          ),
        if (widget.onLinkDevice != null)
          IconButton(
            key: Key('link-$keyPrefix'),
            icon: const Icon(Icons.person_add_alt_1),
            tooltip: 'Send device intro',
            onPressed: () => widget.onLinkDevice!(device),
          ),
        if (widget.onCallDevice != null)
          IconButton(
            key: Key('call-$keyPrefix'),
            icon: const Icon(Icons.phone),
            tooltip: 'Call',
            onPressed: () => widget.onCallDevice!(device),
          ),
      ],
    );
  }

  DataRow _contactRow(MobileContactRow c) {
    return DataRow(
      key: ValueKey('contact-row-${c.contactId}'),
      cells: [
        DataCell(
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              _mutedMarker('contact-${c.contactId}', _contactIsMuted(c)),
              if (c.deviceCount.toInt() > 1) _expandButton(c),
              Text(_label(c)),
            ],
          ),
        ),
        DataCell(Text('${_dmCount(c)}')),
        DataCell(Text('${_broadcastCount(c)}')),
        DataCell(Text(c.lastSeen)),
        DataCell(Text(c.firstSeen)),
        DataCell(
          _actionButtons(
            '${c.contactId}.best',
            c,
            c.deviceRows.first,
            // Only another bound user has an owner-side override; device rows
            // and this install's own user never receive the action (the Rust
            // writer rejects both).
            onEditNickname:
                c.isUser &&
                    c.contactId != widget.ownUserId &&
                    widget.onEditContactNickname != null
                ? (nickname) => widget.onEditContactNickname!(c, nickname)
                : null,
          ),
        ),
      ],
    );
  }

  DataRow _deviceRow(MobileContactRow c, MobileContactDevice d) {
    return DataRow(
      key: ValueKey('device-row-${d.peerId}'),
      cells: [
        DataCell(
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              const SizedBox(width: 24),
              _mutedMarker(d.peerId, _isMuted(d.peerId)),
              Text(d.displayName),
            ],
          ),
        ),
        const DataCell(Text(_kDeviceStatPlaceholder)),
        const DataCell(Text(_kDeviceStatPlaceholder)),
        DataCell(Text(d.lastSeen)),
        DataCell(Text(d.firstSeen)),
        DataCell(_actionButtons(d.peerId, c, d)),
      ],
    );
  }

  @override
  void dispose() {
    _searchController.dispose();
    super.dispose();
  }

  /// Identity of the current sort inputs: the rows list identity + shape +
  /// the sort params. Cache hits render synchronously; misses re-sort
  /// off-band and swap in via setState.
  String get _sortKey =>
      '${identityHashCode(widget.rows)}:${widget.rows.length}:'
      '$_query:$_sortColumn:$_ascending';

  Future<List<MobileContactRow>> _computeSorted() async {
    final visible = _visibleRows();
    final rows = visible
        .map(
          (c) => ContactSortInput(
            contactId: c.contactId,
            displayName: c.displayName,
            lastSeen: c.lastSeen,
            firstSeen: c.firstSeen,
            // Pass the Rust-side i64 values straight through (an int round
            // trip would not survive the web build's BigInt mapping).
            dmCount: c.dmCount,
            broadcastSentToPeer: c.broadcastSentToPeer,
          ),
        )
        .toList();
    final sorted = await _runSort(rows);
    final byId = {for (final r in visible) r.contactId: r};
    return [for (final row in sorted) byId[row.contactId]!];
  }

  Future<List<ContactSortInput>> _runSort(List<ContactSortInput> rows) {
    final override = widget.sortOverride;
    if (override != null) {
      return Future<List<ContactSortInput>>.value(
        override(rows, _sortColumn, _ascending),
      );
    }
    return nodeApi.sortContacts(
      contacts: rows,
      sortColumn: _sortColumn,
      ascending: _ascending,
    );
  }

  Future<void> _resort() async {
    final key = _sortKey;
    if (key == _sortedCacheKey && _sortedCache != null) return;
    final sorted = await _computeSorted();
    // Drop superseded results (a newer sort started while this one was in
    // flight); the newer call lands the then-current ordering.
    if (!mounted || key != _sortKey) return;
    setState(() {
      _sortedCache = sorted;
      _sortedCacheKey = key;
    });
  }

  void _sort(int column, bool ascending) {
    setState(() {
      _sortColumn = column;
      _ascending = ascending;
    });
    unawaited(_resort());
  }

  void _onQueryChanged(String value) {
    setState(() => _query = value);
    unawaited(_resort());
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final sorted = _sortedCache ?? _visibleRows();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
          color: cs.primaryContainer,
          child: Row(
            children: [
              const Icon(Icons.people, size: 20),
              const SizedBox(width: 8),
              Text(
                _query.isEmpty
                    ? 'Contacts (${widget.rows.length})'
                    : 'Contacts (${sorted.length}/${widget.rows.length})',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(color: cs.onPrimaryContainer),
              ),
              const Spacer(),
              SizedBox(
                width: 220,
                child: TextField(
                  key: const Key('contacts-search'),
                  controller: _searchController,
                  onChanged: _onQueryChanged,
                  decoration: InputDecoration(
                    isDense: true,
                    hintText: 'Search contacts',
                    prefixIcon: const Icon(Icons.search, size: 18),
                    suffixIcon: _query.isEmpty
                        ? null
                        : IconButton(
                            key: const Key('contacts-search-clear'),
                            icon: const Icon(Icons.close, size: 18),
                            tooltip: 'Clear search',
                            onPressed: () {
                              _searchController.clear();
                              _onQueryChanged('');
                            },
                          ),
                    filled: true,
                    fillColor: cs.surface,
                    border: const OutlineInputBorder(),
                  ),
                ),
              ),
            ],
          ),
        ),
        Expanded(
          child: sorted.isEmpty
              ? Center(
                  child: Text(
                    _query.isEmpty ? 'No contacts yet.' : 'No contacts match.',
                    style: TextStyle(color: Colors.grey[500]),
                  ),
                )
              : SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: DataTable(
                    sortColumnIndex: _sortColumn,
                    sortAscending: _ascending,
                    columns: [
                      DataColumn(label: const Text('Name'), onSort: _sort),
                      DataColumn(label: const Text('DMs'), onSort: _sort),
                      DataColumn(
                        label: const Text('Broadcasts'),
                        onSort: _sort,
                      ),
                      DataColumn(label: const Text('Last seen'), onSort: _sort),
                      DataColumn(
                        label: const Text('First seen'),
                        onSort: _sort,
                      ),
                      const DataColumn(label: Text('')),
                    ],
                    rows: [
                      for (final c in sorted) ...[
                        _contactRow(c),
                        if (_expanded.contains(c.contactId))
                          for (final d in c.deviceRows) _deviceRow(c, d),
                      ],
                    ],
                  ),
                ),
        ),
      ],
    );
  }
}

/// Modal editor for the owner-side nickname override of a user contact.
///
/// Validation and display-name resolution stay in Rust: this dialog only
/// collects trimmed text (empty means clear), calls [onSave], and surfaces a
/// rejection in a SnackBar without closing the dialog.
class _ContactNicknameDialog extends StatefulWidget {
  const _ContactNicknameDialog({required this.onSave});

  /// Called with the trimmed nickname, or `null` for a clear request (either
  /// the Clear action or an empty Save). Throws when Rust rejects the write.
  final Future<void> Function(String? nickname) onSave;

  @override
  State<_ContactNicknameDialog> createState() => _ContactNicknameDialogState();
}

class _ContactNicknameDialogState extends State<_ContactNicknameDialog> {
  final TextEditingController _controller = TextEditingController();
  bool _submitting = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _submit(String? nickname) async {
    if (_submitting) return;
    setState(() => _submitting = true);
    try {
      await widget.onSave(nickname);
      if (mounted) Navigator.of(context).pop(true);
    } catch (e) {
      if (!mounted) return;
      setState(() => _submitting = false);
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text('Invalid nickname: $e')));
    }
  }

  void _submitFromField() {
    final text = _controller.text.trim();
    unawaited(_submit(text.isEmpty ? null : text));
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Set contact nickname'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          const Text(
            'Name shown for this contact on your device. Leave the field empty '
            'to use the default name.',
          ),
          const SizedBox(height: 12),
          TextField(
            key: const Key('contact-nickname-field'),
            controller: _controller,
            autofocus: true,
            enabled: !_submitting,
            decoration: const InputDecoration(
              labelText: 'Nickname',
              hintText: 'letters, digits and dashes',
            ),
            onSubmitted: (_) {
              if (!_submitting) _submitFromField();
            },
          ),
        ],
      ),
      actions: [
        TextButton(
          key: const Key('contact-nickname-cancel'),
          onPressed: _submitting
              ? null
              : () => Navigator.of(context).pop(false),
          child: const Text('Cancel'),
        ),
        TextButton(
          key: const Key('contact-nickname-clear'),
          onPressed: _submitting ? null : () => _submit(null),
          child: const Text('Clear'),
        ),
        FilledButton(
          key: const Key('contact-nickname-save'),
          onPressed: _submitting ? null : _submitFromField,
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
