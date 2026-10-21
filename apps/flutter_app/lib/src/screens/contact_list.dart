import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/util/ui_utils.dart';

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

  Widget _actionButtons(
    String keyPrefix,
    MobileContactRow c,
    MobileContactDevice device,
  ) {
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
              if (c.deviceCount.toInt() > 1) _expandButton(c),
              Text(_label(c)),
            ],
          ),
        ),
        DataCell(Text('${_dmCount(c)}')),
        DataCell(Text('${_broadcastCount(c)}')),
        DataCell(Text(formatSeenTimestamp(c.lastSeen))),
        DataCell(Text(formatSeenTimestamp(c.firstSeen))),
        DataCell(_actionButtons('${c.contactId}.best', c, c.deviceRows.first)),
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
            children: [const SizedBox(width: 24), Text(d.displayName)],
          ),
        ),
        const DataCell(Text(_kDeviceStatPlaceholder)),
        const DataCell(Text(_kDeviceStatPlaceholder)),
        DataCell(Text(formatSeenTimestamp(d.lastSeen))),
        DataCell(Text(formatSeenTimestamp(d.firstSeen))),
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
