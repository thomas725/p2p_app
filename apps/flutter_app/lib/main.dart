import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/data/http_node_api.dart';
import 'src/data/node_api.dart';
import 'src/rust/frb_generated.dart';
import 'src/screens/home.dart';
import 'src/util/event_bus.dart';
import 'src/util/ui_utils.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  if (kIsWeb) {
    // Browser build: the node lives in a native `p2p_chat_server` process;
    // this app is a pure display client over its WebSocket/HTTP API. The
    // transport is installed before runApp so the first frame can already
    // reach the node. Deployments that require auth carry the token in the
    // page URL (`?token=...`).
    nodeApi = HttpNodeApi.toServer(token: webAuthToken);
  } else {
    await RustLib.init();
    if (isAndroid) {
      serviceChannel.setMethodCallHandler(_handleServiceCall);
    }
  }
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
class P2pApp extends StatelessWidget {
  const P2pApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'p2p_app',
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
      home: const HomeScreen(),
    );
  }
}
