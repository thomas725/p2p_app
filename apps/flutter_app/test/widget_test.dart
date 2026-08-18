// Smoke test for the p2p_app_flutter application
//
// This test verifies the app can be built and rendered without errors.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

import 'package:p2p_app_flutter/main.dart';

void main() {
  testWidgets('App builds and shows loading screen', (WidgetTester tester) async {
    await tester.pumpWidget(const P2pApp());
    await tester.pumpAndSettle();

    // The app starts with a loading indicator while initializing
    // We just verify it builds without crashing
    expect(find.byType(MaterialApp), findsOneWidget);
  });
}
