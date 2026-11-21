import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/data/http_node_api.dart';
import 'src/data/node_api.dart';
import 'src/rust/frb_generated.dart';
import 'src/screens/home.dart';
import 'src/util/event_bus.dart';
import 'src/util/notifications.dart' show setAppInForeground;
import 'src/util/theme.dart';
import 'src/util/ui_utils.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    // Browser build: the node lives in a native `p2p_chat_server` process;
    // this app is a pure display client over its WebSocket/HTTP API. The
    // transport is installed before runApp so the first frame can already
    // reach the node. Auth is HTTP Basic handled by the browser: the session
    // cookie it stores travels with both the WebSocket handshake and the
    // attachment routes, so no credential ever reaches Dart or the URL.
    nodeApi = HttpNodeApi.toServer();
  } else {
    await RustLib.init();
    if (isAndroid) {
      serviceChannel.setMethodCallHandler(_handleServiceCall);
    }
  }
  await loadTheme();
  runApp(const P2pApp());
}

Future<dynamic> _handleServiceCall(MethodCall call) async {
  switch (call.method) {
    case 'startNetworking':
      final dbPath = call.arguments as String? ?? defaultDbPath;
      try {
        await nodeApi.startNode(dbPath: dbPath);
        startEventPolling();
        return true;
      } catch (_) {
        return false;
      }
    case 'stopNetworking':
      stopEventPolling();
      await nodeApi.stopNode();
      return true;
    default:
      return null;
  }
}

/// Root application widget: theme + home screen.
///
/// Rebuilds on theme changes so a Settings toggle repaints the whole app. The
/// seed colour is Rust's canonical accent ARGB; Dart never defines a palette.
class P2pApp extends StatefulWidget {
  const P2pApp({super.key});

  @override
  State<P2pApp> createState() => _P2pAppState();
}

class _P2pAppState extends State<P2pApp> with WidgetsBindingObserver {
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    themeNotifier.addListener(_onTheme);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    themeNotifier.removeListener(_onTheme);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    setAppInForeground(state == AppLifecycleState.resumed);
  }

  void _onTheme() => setState(() {});

  @override
  Widget build(BuildContext context) {
    final theme = themeNotifier.value;
    final seed = Color(theme.accentArgb);
    return MaterialApp(
      title: 'p2p_app',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: seed),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: seed,
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      themeMode: theme.dark ? ThemeMode.dark : ThemeMode.light,
      home: const HomeScreen(),
    );
  }
}
