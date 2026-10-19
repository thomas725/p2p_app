import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:http/http.dart' as http;
import 'package:web_socket_channel/web_socket_channel.dart';

/// Duplex transport to a `p2p_chat_server` (see
/// `docs/2026-10-18_web_frontend.md`): text frames for the JSON
/// request/response + push-event protocol, plus HTTP routes for attachment
/// bytes. Production is [WsRemoteTransport]; Dart tests inject a fake so the
/// [HttpNodeApi] logic runs without a network.
abstract class RemoteTransport {
  /// Incoming text frames (responses and pushed swarm events).
  Stream<String> get frames;

  /// Completes once the connection is usable (immediately for lazy fakes).
  Future<void> get ready;

  /// Send one outgoing text frame.
  Future<void> send(String frame);

  /// POST attachment bytes to `/api/upload`; returns the parsed response
  /// body (`{"attachment": {...}}`).
  Future<Map<String, dynamic>> upload({
    required String name,
    required Uint8List bytes,
  });

  /// Browser-openable URL that downloads the stored blob
  /// (`/api/files/{fileId}`).
  String downloadUrl(String fileId);

  void close();
}

/// Production transport: a WebSocket for the JSON protocol and HTTP for
/// attachments, both against a single node-server origin.
class WsRemoteTransport implements RemoteTransport {
  WsRemoteTransport._(this._channel, this._httpBase, this._token);

  /// Connect to the node server at [origin] — the page's own origin by
  /// default, so the SPA and its API share one host. [token] is carried on
  /// the WS handshake and the HTTP routes when the deployment requires one.
  static Future<WsRemoteTransport> connect({
    Uri? origin,
    String? token,
  }) async {
    final base = origin ?? _pageOrigin();
    final wsBase = base.replace(scheme: base.scheme == 'https' ? 'wss' : 'ws');
    final channel = WebSocketChannel.connect(
      wsBase.replace(
        path: '/ws',
        queryParameters: token == null ? null : {'token': token},
      ),
    );
    await channel.ready;
    return WsRemoteTransport._(channel, base, token);
  }

  /// Scheme/host/port of the page currently displayed (SPA assets and the
  /// API share one origin; the SPA route path itself is irrelevant).
  static Uri _pageOrigin() =>
      Uri(scheme: Uri.base.scheme, host: Uri.base.host, port: Uri.base.port);

  final WebSocketChannel _channel;
  final Uri _httpBase;
  final String? _token;

  @override
  Stream<String> get frames =>
      _channel.stream.map((event) => event.toString());

  @override
  Future<void> get ready => _channel.ready;

  @override
  Future<void> send(String frame) async {
    _channel.sink.add(frame);
  }

  @override
  Future<Map<String, dynamic>> upload({
    required String name,
    required Uint8List bytes,
  }) async {
    final uri = _httpBase.replace(
      path: '/api/upload',
      queryParameters: {'name': name, 'token': ?_token},
    );
    final response = await http.post(
      uri,
      headers: const {'content-type': 'application/octet-stream'},
      body: bytes,
    );
    if (response.statusCode != 200) {
      throw http.ClientException(
        'upload failed with HTTP ${response.statusCode}: ${response.body}',
        uri,
      );
    }
    return jsonDecode(response.body) as Map<String, dynamic>;
  }

  @override
  String downloadUrl(String fileId) => _httpBase
      .replace(
        path: '/api/files/$fileId',
        queryParameters: {'token': ?_token},
      )
      .toString();

  @override
  void close() {
    unawaited(_channel.sink.close());
  }
}
