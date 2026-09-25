import 'package:flutter/material.dart';

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
/// best-device row of a multi-device user — with per-row Peer-info and
/// Direct-message actions (the parent resolves the row's best device).
class ContactList extends StatefulWidget {
  const ContactList({
    super.key,
    required this.rows,
    required this.onOpenInfo,
    required this.onOpenDm,
    this.sortOverride,
  });
  final List<MobileContactRow> rows;
  final void Function(MobileContactRow) onOpenInfo;
  final void Function(MobileContactRow) onOpenDm;

  /// Test-only seam: when set, replaces the FFI-backed `sortContacts` call so
  /// widget tests can exercise sorting without a loaded Rust library.
  @visibleForTesting
  final SortContactsOverride? sortOverride;

  @override
  State<ContactList> createState() => ContactListState();
}

class ContactListState extends State<ContactList> {
  static const int _kLastSeen = 3;

  int _sortColumn = _kLastSeen;
  bool _ascending = false;

  int _dmCount(MobileContactRow c) => c.dmCount.toInt();
  int _broadcastCount(MobileContactRow c) => c.broadcastSentToPeer.toInt();

  /// Display name, with a ` ×N` device-count suffix when the contact spans
  /// multiple bound devices (mirrors the TUI render-only suffix).
  String _label(MobileContactRow c) => c.deviceCount.toInt() > 1
      ? '${c.displayName} \u00d7${c.deviceCount}'
      : c.displayName;

  List<MobileContactRow> get _sorted {
    final rows = widget.rows
        .map(
          (c) => ContactSortInput(
            contactId: c.contactId,
            displayName: c.displayName,
            lastSeen: c.lastSeen,
            firstSeen: c.firstSeen,
            dmCount: _dmCount(c),
            broadcastSentToPeer: _broadcastCount(c),
          ),
        )
        .toList();
    final sorted = _runSort(rows);
    final byId = {for (final r in widget.rows) r.contactId: r};
    return [for (final row in sorted) byId[row.contactId]!];
  }

  List<ContactSortInput> _runSort(List<ContactSortInput> rows) {
    final override = widget.sortOverride;
    if (override != null) {
      return override(rows, _sortColumn, _ascending);
    }
    return sortContacts(
      contacts: rows,
      sortColumn: _sortColumn,
      ascending: _ascending,
    );
  }

  void _sort(int column, bool ascending) {
    setState(() {
      _sortColumn = column;
      _ascending = ascending;
    });
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final sorted = _sorted;
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
                'Contacts (${widget.rows.length})',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(color: cs.onPrimaryContainer),
              ),
            ],
          ),
        ),
        Expanded(
          child: sorted.isEmpty
              ? Center(
                  child: Text(
                    'No contacts yet.',
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
                      for (final c in sorted)
                        DataRow(
                          cells: [
                            DataCell(Text(_label(c))),
                            DataCell(Text('${_dmCount(c)}')),
                            DataCell(Text('${_broadcastCount(c)}')),
                            DataCell(Text(formatSeenTimestamp(c.lastSeen))),
                            DataCell(Text(formatSeenTimestamp(c.firstSeen))),
                            DataCell(
                              Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  IconButton(
                                    icon: const Icon(Icons.info_outline),
                                    tooltip: 'Peer info',
                                    onPressed: () => widget.onOpenInfo(c),
                                  ),
                                  IconButton(
                                    icon: const Icon(Icons.chat),
                                    tooltip: 'Direct message',
                                    onPressed: () => widget.onOpenDm(c),
                                  ),
                                ],
                              ),
                            ),
                          ],
                        ),
                    ],
                  ),
                ),
        ),
      ],
    );
  }
}