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

import 'package:p2p_app_flutter/src/rust/messages.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/peer_list.dart';

MobilePeerRecord peer({
  required String id,
  String name = '',
  int dm = 0,
  int broadcast = 0,
}) => MobilePeerRecord(
  peerId: id,
  displayName: name.isEmpty ? id : name,
  firstSeen: '',
  lastSeen: '',
);

// Deterministic fake of the Rust `sortPeers` (mobile_api.rs). Mirrors the real
// comparator exactly: column `0`/unknown = lowercase display name, `1` DM count,
// `2` broadcast count, `3` last-seen, `4` first-seen (via strict
// `%Y-%m-%dT%H:%M:%S`-or-space parse with 0 on failure), each with an ascending
// `peer_id` tie-break. A descending sort reverses the WHOLE ordering — the
// tie-break included — matching Rust `ord.reverse()`.
int _parseSeenMs(String s) {
  final m = RegExp(
    r'^(\d{4})-(\d{2})-(\d{2})[T ](\d{2}):(\d{2}):(\d{2})$',
  ).firstMatch(s);
  if (m == null) return 0;
  final p = [for (var i = 1; i <= 6; i++) int.parse(m.group(i)!)];
  return DateTime.utc(
    p[0],
    p[1],
    p[2],
    p[3],
    p[4],
    p[5],
  ).millisecondsSinceEpoch;
}

List<PeerSortInput> _sortLikeRust(
  List<PeerSortInput> rows,
  int column,
  bool ascending,
) {
  int tie(PeerSortInput a, PeerSortInput b) => a.peerId.compareTo(b.peerId);
  int cmp(PeerSortInput a, PeerSortInput b) {
    final v = switch (column) {
      1 => a.dmCount.compareTo(b.dmCount),
      2 => a.broadcastCount.compareTo(b.broadcastCount),
      3 => _parseSeenMs(a.lastSeen).compareTo(_parseSeenMs(b.lastSeen)),
      4 => _parseSeenMs(a.firstSeen).compareTo(_parseSeenMs(b.firstSeen)),
      _ => a.displayName.toLowerCase().compareTo(b.displayName.toLowerCase()),
    };
    final combined = v != 0 ? v : tie(a, b);
    return ascending ? combined : -combined;
  }

  final out = [...rows]..sort(cmp);
  return out;
}

Widget _app({
  required List<MobilePeerRecord> peers,
  Map<String, PeerMessageStats> stats = const {},
  SortPeersOverride? sortOverride,
}) => MaterialApp(
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
  testWidgets(
    'Broadcast column shows sent-to-peer (broadcastSentToPeer) values',
    (tester) async {
      final peers = [
        peer(id: 'aaa', name: 'Alpha'),
        peer(id: 'bbb', name: 'Beta', broadcast: 5),
      ];
      final stats = {
        'aaa': const PeerMessageStats(dmCount: 1, broadcastSentToPeer: 0),
        'bbb': const PeerMessageStats(dmCount: 2, broadcastSentToPeer: 5),
      };
      await tester.pumpWidget(
        _app(peers: peers, stats: stats, sortOverride: _sortLikeRust),
      );
      await tester.pump();

      expect(find.text('Alpha'), findsOneWidget);
      expect(find.text('Beta'), findsOneWidget);
      // The Broadcast column reflects broadcastSentToPeer (5), not inbound.
      expect(find.text('5'), findsWidgets);
    },
  );

  testWidgets('clicking Name header sorts ascending then toggles descending', (
    tester,
  ) async {
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

  testWidgets('equal names tie-break by peer_id and header click toggles order', (
    tester,
  ) async {
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
    await tester.pumpWidget(_app(peers: peers, sortOverride: recordingSort));
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
    // Descending reverses the WHOLE ordering (tie-break included), mirroring
    // Rust `ord.reverse()`: equal-name ties now sort peer_id descending.
    expect(calls.last.$3, ['ccc', 'bbb', 'aaa']);

    // All three same-named peers still render (no rows dropped by the sort).
    expect(find.text('Same'), findsNWidgets(3));
  });

  testWidgets('Last-seen column sorts by parsed timestamp with 0-on-failure', (
    tester,
  ) async {
    final peers = [
      MobilePeerRecord(
        peerId: 'zulu',
        displayName: 'Zulu',
        firstSeen: '',
        lastSeen: '2026-08-03T10:00:00',
      ),
      MobilePeerRecord(
        peerId: 'alpha',
        displayName: 'Alpha',
        firstSeen: '',
        lastSeen: '2026-08-01T10:00:00',
      ),
      MobilePeerRecord(
        peerId: 'same',
        displayName: 'Same',
        firstSeen: '',
        lastSeen: 'junk', // unparseable -> 0, mirroring parse_last_seen_ms
      ),
    ];
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

    await tester.pumpWidget(_app(peers: peers, sortOverride: recordingSort));
    await tester.pump();

    // Default state: Last seen, descending. 2026-08-03 > 08-01 > 0 (junk).
    expect(calls.first.$3, ['zulu', 'alpha', 'same']);

    await tester.tap(find.text('Last seen'));
    await tester.pump();
    expect(calls.last.$1, 3);
    expect(calls.last.$2, isTrue);
    expect(calls.last.$3, ['same', 'alpha', 'zulu']);

    await tester.tap(find.text('Last seen'));
    await tester.pump();
    expect(calls.last.$1, 3);
    expect(calls.last.$2, isFalse);
    expect(calls.last.$3, ['zulu', 'alpha', 'same']);
  });

  testWidgets(
    'last/first-seen timestamps render T→space, truncated to seconds',
    (tester) async {
      final peers = [
        MobilePeerRecord(
          peerId: 'alpha',
          displayName: 'Alpha',
          firstSeen: '2026-08-01T10:00:00.000Z',
          lastSeen: '2026-08-28T15:26:41.123Z',
        ),
      ];
      await tester.pumpWidget(_app(peers: peers, sortOverride: _sortLikeRust));
      await tester.pump();

      // Truncated to the second, T replaced by a space, trailing Z dropped.
      expect(find.text('2026-08-28 15:26:41'), findsOneWidget);
      expect(find.text('2026-08-01 10:00:00'), findsOneWidget);
      expect(find.textContaining('T15'), findsNothing);
      expect(find.textContaining('.123Z'), findsNothing);
    },
  );

  testWidgets('empty last-seen renders as unknown', (tester) async {
    final peers = [peer(id: 'alpha', name: 'Alpha')];
    await tester.pumpWidget(_app(peers: peers, sortOverride: _sortLikeRust));
    await tester.pump();

    expect(find.text('unknown'), findsNWidgets(2)); // first + last seen
  });
}
