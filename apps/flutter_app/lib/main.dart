import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'src/rust/api.dart';
import 'src/rust/frb_generated.dart';
import 'src/screens/home.dart';
import 'src/util/env.dart';
import 'src/util/event_bus.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  if (isAndroid) {
    serviceChannel.setMethodCallHandler(_handleServiceCall);
  }
  runApp(const P2pApp());
}

Future<dynamic> _handleServiceCall(MethodCall call) async {
  switch (call.method) {
    case 'startNetworking':
      final dbPath = call.arguments as String? ?? defaultDbPath;
      try {
        await startNode(dbPath: dbPath);
        startEventPolling();
        return true;
      } catch (_) {
        return false;
      }
    case 'stopNetworking':
      stopEventPolling();
      await stopNode();
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
