import 'dart:io' show Platform;

import 'package:flutter/services.dart';

/// Android service method channel used by the app shell and settings screen.
const serviceChannel = MethodChannel('com.example.p2p_app_flutter/service');

final bool isAndroid = Platform.isAndroid;

/// Network/topic name the node joins. Mirrors CHAT_TOPIC in the Rust core.
const String kNetworkName = 'test-net';

/// SQLite database path passed to the Rust node (Android app-private dir on
/// Android, a local file elsewhere).
String get defaultDbPath => isAndroid
    ? '/data/data/com.example.p2p_app_flutter/databases/p2p.db'
    : 'p2p.db';
