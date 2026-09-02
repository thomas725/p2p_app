// Widget tests for the Peers tab `PeerList` table.
//
// The real sort path calls the Rust `sortPeers` FFI function, which is not
// loaded in a `flutter test` harness. The widget therefore exposes a
// `@visibleForTesting` `sortOverride` seam that replaces the FFI call. We use a
// deterministic fake that replicates the Rust comparator's documented behaviour
// (sort by column honouring `ascending`, always tie-breaking on `peer_id`) so
// the widget's header-click sorting interaction can be asserted end to end.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/main.dart';
import 'package:p2p_app_flutter/src/rust/messages.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

MobilePeerRecord peer({
  required String id,
  String name = '',
  int dm = 0,
  int broadcast = 0,
}) =>
    MobilePeerRecord(
      peerId: id,
      displayName: name.isEmpty ? id : name,
      firstSeen: '',
      lastSeen: '',
    );

// Deterministic fake of the Rust `sortPeers`: sort ascending or descending on a
// key column, and always tie-break on `peer_id` (ascending on the tie).
List<PeerSortInput> _sortLikeRust(
  List<PeerSortInput> rows,
  int column,
  bool ascending,
) {
  int tie(PeerSortInput a, PeerSortInput b) => a.peerId.compareTo(b.peerId);
  int keyCompare(PeerSortInput a, PeerSortInput b) {
    final v = switch (column) {
      0 => a.displayName.compareTo(b.displayName),
      1 => a.dmCount - b.dmCount,
      2 => a.broadcastCount - b.broadcastCount,
      _ => a.peerId.compareTo(b.peerId),
    };
    if (v != 0) return ascending ? v : -v;
    return tie(a, b);
  }

  final out = [...rows]..sort(keyCompare);
  return out;
}

Widget _app({
  required List<MobilePeerRecord> peers,
  Map<String, PeerMessageStats> stats = const {},
  SortPeersOverride? sortOverride,
}) =>
    MaterialApp(
      home: Scaffold(
        body: PeerList(
          peers: peers,
          stats: stats,
          serviceRunning: true,
          onOpenInfo: (_) {},
          onOpenDm: (_) {},
          sortOverride: sortOverride,
        ),
      ),
    );

// Read the display-name text per row in top-to-bottom order. `DataTable`
// renders cells as `TableCell`s (not `DataCell` elements), so we locate the name
// `Text`s by their data and sort by vertical screen position.
List<String?> _displayNames(WidgetTester tester) {
  final names = <String?>[];
  for (final name in const ['Alpha', 'Zulu', 'Same']) {
    if (find.text(name).evaluate().isNotEmpty) {
      names.add(name);
    }
  }
  // Reorder by the widget's y-position so we capture row order.
  names.sort((a, b) {
    final ay = tester.getTopLeft(find.text(a!)).dy;
    final by = tester.getTopLeft(find.text(b!)).dy;
    return ay.compareTo(by);
  });
  return names;
}

void main() {
  testWidgets('Broadcast column shows broadcastReceived (inbound) values',
      (tester) async {
    final peers = [
      peer(id: 'aaa', name: 'Alpha'),
      peer(id: 'bbb', name: 'Beta', broadcast: 5),
    ];
    final stats = {
      'aaa': const PeerMessageStats(dmCount: 1, broadcastReceived: 0),
      'bbb': const PeerMessageStats(dmCount: 2, broadcastReceived: 5),
    };
    await tester.pumpWidget(
      _app(peers: peers, stats: stats, sortOverride: _sortLikeRust),
    );
    await tester.pump();

    expect(find.text('Alpha'), findsOneWidget);
    expect(find.text('Beta'), findsOneWidget);
    // The Broadcast column reflects broadcastReceived (5), not a removed
    // broadcastSent.
    expect(find.text('5'), findsWidgets);
  });

  testWidgets('clicking Name header sorts ascending then toggles descending',
      (tester) async {
    final peers = [
      peer(id: 'beta', name: 'Zulu'),
      peer(id: 'alpha', name: 'Alpha'),
    ];
    await tester.pumpWidget(_app(peers: peers, sortOverride: _sortLikeRust));
    await tester.pump();

    // Default column is Last seen with `_ascending = false` (descending); both
    // peers have empty last-seen, so the tie-break (peer_id, descending) puts
    // Zulu first.
    expect(_displayNames(tester), ['Zulu', 'Alpha']);

    // Click the Name header -> sorts ascending by display name (Alpha, Zulu).
    await tester.tap(find.text('Name'));
    await tester.pump();
    expect(_displayNames(tester), ['Alpha', 'Zulu']);

    // Click it again -> toggles descending (Zulu, Alpha).
    await tester.tap(find.text('Name'));
    await tester.pump();
    expect(_displayNames(tester), ['Zulu', 'Alpha']);
  });

  testWidgets('equal names tie-break by peer_id and header click toggles order',
      (tester) async {
    // Record every sort invocation the widget makes, so we can assert both the
    // column/ascending wiring on header clicks and that peer_id deterministically
    // breaks ties among equal-name rows (mirroring Rust `sortPeers`).
    final calls = <(int, bool, List<String>)>[];
    List<PeerSortInput> recordingSort(
      List<PeerSortInput> rows,
      int column,
      bool ascending,
    ) {
      final out = _sortLikeRust(rows, column, ascending);
      calls.add((column, ascending, [for (final r in out) r.peerId]));
      return out;
    }

    final peers = [
      peer(id: 'ccc', name: 'Same'),
      peer(id: 'aaa', name: 'Same'),
      peer(id: 'bbb', name: 'Same'),
    ];
    await tester.pumpWidget(
      _app(peers: peers, sortOverride: recordingSort),
    );
    await tester.pump();

    // First click on Name sorts ascending (column 0, ascending true); all names
    // equal, so order is the peer_id tie-break: aaa, bbb, ccc. (Records compare
    // Lists by identity, so assert the tuple fields and the list by elements.)
    await tester.tap(find.text('Name'));
    await tester.pump();
    expect(calls.last.$1, 0);
    expect(calls.last.$2, isTrue);
    expect(calls.last.$3, ['aaa', 'bbb', 'ccc']);

    // Second click toggles to descending (column 0, ascending false).
    await tester.tap(find.text('Name'));
    await tester.pump();
    expect(calls.last.$1, 0);
    expect(calls.last.$2, isFalse);

    // All three same-named peers still render (no rows dropped by the sort).
    expect(find.text('Same'), findsNWidgets(3));
  });
}