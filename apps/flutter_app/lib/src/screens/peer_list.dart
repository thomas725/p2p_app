import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/messages.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/util/formats.dart';

/// Test-only seam for [PeerListState]: replaces the FFI-backed `sortPeers`
/// call so widget tests can exercise sorting without a loaded Rust library.
@visibleForTesting
typedef SortPeersOverride =
    List<PeerSortInput> Function(
      List<PeerSortInput> rows,
      int column,
      bool ascending,
    );

/// Peers tab: a sortable table (Rust `sortPeers`-backed) plus per-row
/// Peer-info and Direct-message actions.
class PeerList extends StatefulWidget {
  const PeerList({
    super.key,
    required this.peers,
    required this.stats,
    required this.onOpenInfo,
    required this.onOpenDm,
    required this.serviceRunning,
    this.sortOverride,
  });
  final List<MobilePeerRecord> peers;
  final Map<String, PeerMessageStats> stats;
  final void Function(MobilePeerRecord) onOpenInfo;
  final void Function(MobilePeerRecord) onOpenDm;
  final bool serviceRunning;

  /// Test-only seam: when set, replaces the FFI-backed `sortPeers` call so
  /// widget tests can exercise sorting without a loaded Rust library.
  @visibleForTesting
  final SortPeersOverride? sortOverride;

  @override
  State<PeerList> createState() => PeerListState();
}

class PeerListState extends State<PeerList> {
  static const int _kLastSeen = 3;

  int _sortColumn = _kLastSeen;
  bool _ascending = false;

  int _dmCount(MobilePeerRecord p) =>
      (widget.stats[p.peerId]?.dmCount ?? 0).toInt();
  int _broadcastCount(MobilePeerRecord p) =>
      (widget.stats[p.peerId]?.broadcastSentToPeer ?? 0).toInt();

  List<MobilePeerRecord> get _sorted {
    final rows = widget.peers
        .map(
          (p) => PeerSortInput(
            peerId: p.peerId,
            displayName: p.displayName,
            lastSeen: p.lastSeen,
            firstSeen: p.firstSeen,
            dmCount: _dmCount(p),
            broadcastCount: _broadcastCount(p),
          ),
        )
        .toList();
    final sorted = _runSort(rows);
    final byId = {for (final p in widget.peers) p.peerId: p};
    return [for (final row in sorted) byId[row.peerId]!];
  }

  List<PeerSortInput> _runSort(List<PeerSortInput> rows) {
    final override = widget.sortOverride;
    if (override != null) {
      return override(rows, _sortColumn, _ascending);
    }
    return sortPeers(
      peers: rows,
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
                'Peers (${widget.peers.length})',
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
                    'No peers discovered yet.',
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
                      for (final p in sorted)
                        DataRow(
                          cells: [
                            DataCell(Text(p.displayName)),
                            DataCell(Text('${_dmCount(p)}')),
                            DataCell(Text('${_broadcastCount(p)}')),
                            DataCell(Text(formatSeenTimestamp(p.lastSeen))),
                            DataCell(Text(formatSeenTimestamp(p.firstSeen))),
                            DataCell(
                              Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  IconButton(
                                    icon: const Icon(Icons.info_outline),
                                    tooltip: 'Peer info',
                                    onPressed: () => widget.onOpenInfo(p),
                                  ),
                                  IconButton(
                                    icon: const Icon(Icons.chat),
                                    tooltip: 'Direct message',
                                    onPressed: () => widget.onOpenDm(p),
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
