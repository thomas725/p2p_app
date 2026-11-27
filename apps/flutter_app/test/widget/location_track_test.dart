import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/location.dart' show LocationTrackPoint;
import 'package:p2p_app_flutter/src/screens/location_track.dart';
import 'package:p2p_app_flutter/src/util/attachments.dart'
    show OpenRemoteUrlOverride, openRemoteUrlOverride;
import 'package:p2p_app_flutter/src/util/location_export.dart'
    show SaveGpxOverride, saveGpxOverride;

import '../helpers/fake_node_api.dart';

void main() {
  late NodeApi previousApi;
  late FakeNodeApi api;
  late OpenRemoteUrlOverride previousOpen;
  late SaveGpxOverride previousSave;
  final points = [
    const LocationTrackPoint(
      latitude: 48.137,
      longitude: 11.575,
      accuracy: 15,
      timestampSecs: 1795790400,
      sequence: 1,
      isFinal: false,
    ),
    const LocationTrackPoint(
      latitude: 48.138,
      longitude: 11.576,
      accuracy: 12,
      timestampSecs: 1795790700,
      sequence: 2,
      isFinal: true,
    ),
  ];
  String? loadedPeer;
  String? loadedFrom;
  String? loadedTo;
  String? openedUrl;
  String? savedName;
  String savedGpx = '';

  setUp(() {
    previousApi = nodeApi;
    previousOpen = openRemoteUrlOverride;
    previousSave = saveGpxOverride;
    api = FakeNodeApi();
    loadedPeer = null;
    loadedFrom = null;
    loadedTo = null;
    openedUrl = null;
    savedName = null;
    savedGpx = '';
    api.loadLocationTrackOverride = (peerId, fromText, toText) {
      loadedPeer = peerId;
      loadedFrom = fromText;
      loadedTo = toText;
      return points;
    };
    api.locationTrackUrlOverride = (_) => 'https://maps.example/route?from=1';
    api.locationGpxOverride = (_, label) => '<gpx label="$label" />';
    openRemoteUrlOverride = (url) async {
      openedUrl = url;
      return null;
    };
    saveGpxOverride = ({required fileName, required bytes}) async {
      savedName = fileName;
      savedGpx = utf8.decode(bytes);
      return Uri.parse('file:///tmp/location.gpx');
    };
    nodeApi = api;
  });

  tearDown(() {
    nodeApi = previousApi;
    openRemoteUrlOverride = previousOpen;
    saveGpxOverride = previousSave;
  });

  Widget screen() => MaterialApp(
    home: LocationTrackScreen(peerId: 'sender-device', deviceLabel: 'Phone A'),
  );

  testWidgets('loads one device track and shows Rust-supplied point count', (
    tester,
  ) async {
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();

    expect(loadedPeer, 'sender-device');
    expect(find.byKey(const Key('location-track-count')), findsOneWidget);
    expect(find.text('2 points from Phone A'), findsOneWidget);
    expect(find.textContaining('48.13700'), findsOneWidget);
  });

  testWidgets('opens the capped Rust map URL through the browser seam', (
    tester,
  ) async {
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('location-map-link')));
    await tester.pumpAndSettle();
    expect(openedUrl, 'https://maps.example/route?from=1');
  });

  testWidgets('exports Rust-built GPX through the save seam', (tester) async {
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();

    await tester.tap(find.byKey(const Key('location-track-export')));
    await tester.pumpAndSettle();

    expect(savedName, 'location-sender-device.gpx');
    expect(savedGpx, '<gpx label="Phone A" />');
  });

  testWidgets('passes raw filter text to Rust without parsing it in Dart', (
    tester,
  ) async {
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();

    await tester.enterText(find.byKey(const Key('location-track-from')), '2h');
    await tester.enterText(
      find.byKey(const Key('location-track-to')),
      '2026-11-27T12:00:00',
    );
    await tester.tap(find.byKey(const Key('location-track-load')));
    await tester.pumpAndSettle();

    expect(loadedFrom, '2h');
    expect(loadedTo, '2026-11-27T12:00:00');
    expect(loadedPeer, 'sender-device');
  });

  testWidgets('shows a load error without losing the screen', (tester) async {
    api.loadLocationTrackOverride = (peerId, fromText, toText) =>
        throw StateError('bad filter');
    await tester.pumpWidget(screen());
    await tester.pumpAndSettle();

    expect(find.byKey(const Key('location-track-error')), findsOneWidget);
    expect(find.textContaining('bad filter'), findsOneWidget);
  });
}
