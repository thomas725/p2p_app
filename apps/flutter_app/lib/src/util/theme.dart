import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/theme.dart';

/// Default palette before the persisted one loads (and in widget tests with
/// no node). Matches Rust's `Accent::TEAL` and dark-by-default settings.
const defaultTheme = AppTheme(
  dark: true,
  accentName: 'teal',
  accentArgb: 0xff2f5d50,
  accents: [
    AccentOption(name: 'teal', argb: 0xff2f5d50),
    AccentOption(name: 'blue', argb: 0xff2f5d9e),
    AccentOption(name: 'violet', argb: 0xff6a3fa0),
    AccentOption(name: 'rose', argb: 0xffa03050),
    AccentOption(name: 'amber', argb: 0xff9a6b12),
    AccentOption(name: 'green', argb: 0xff2f7a3c),
  ],
);

/// Live theme shared by the root `MaterialApp` and the Settings screen. Rust
/// owns the palette; this notifier only carries the resolved snapshot so a
/// toggle repaints the whole app without a manual rebuild chain.
final ValueNotifier<AppTheme> themeNotifier = ValueNotifier<AppTheme>(defaultTheme);

/// Load the persisted theme from Rust through the active [nodeApi]. Best-effort:
/// a transport error leaves the default palette in place.
Future<void> loadTheme() async {
  try {
    themeNotifier.value = await nodeApi.getTheme();
  } catch (e) {
    debugPrint('Failed to load theme: $e');
  }
}

/// Persist a theme choice and refresh the notifier. The accent name is
/// validated in Rust, so an invalid swatch never reaches the renderer.
///
/// Returns an error string from Rust/Dart on failure, or `null` on success.
Future<String?> setTheme({required bool dark, required String accentName}) async {
  try {
    await nodeApi.setTheme(dark: dark, accentName: accentName);
    await loadTheme();
    return null;
  } catch (e) {
    return '$e'.replaceFirst('Exception: ', '');
  }
}
