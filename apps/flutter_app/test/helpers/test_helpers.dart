// Test helpers for p2p_app_flutter
//
// Provides common utilities for unit and widget tests.

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';

// Re-export FRB types for test convenience
export 'package:p2p_app_flutter/src/rust/mobile_node.dart'
    show ChatMessage, MobilePeerRecord, SwarmEventJson;

// Extension for easier widget finding
extension WidgetFinderExtensions on CommonFinders {
  Finder byTooltip(String tooltip) => find.byWidgetPredicate(
    (widget) => widget is Tooltip && widget.message == tooltip,
  );
}

// Pump a widget and wait for animations
Future<void> pumpAndSettle(WidgetTester tester, {Duration? duration}) async {
  await tester.pump(duration ?? const Duration(milliseconds: 100));
  await tester.pumpAndSettle();
}

// Create a minimal test app wrapper
Widget createTestApp({required Widget child}) {
  return MaterialApp(
    home: Scaffold(body: child),
    theme: ThemeData(
      colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff2f5d50)),
      useMaterial3: true,
    ),
    darkTheme: ThemeData(
      colorScheme: ColorScheme.fromSeed(
        seedColor: const Color(0xff2f5d50),
        brightness: Brightness.dark,
      ),
      useMaterial3: true,
    ),
    themeMode: ThemeMode.system,
  );
}

// Test constants
const Duration testTimeout = Duration(seconds: 30);
const Duration shortDelay = Duration(milliseconds: 100);