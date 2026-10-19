import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

// PlatformInt64Util is frb's int<->i64 adapter (int on IO, BigInt on web);
// same ignore the generated files carry.
// ignore_for_file: invalid_use_of_internal_member

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart'
    show PlatformInt64Util;

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/data/node_transport.dart';
import 'package:p2p_app_flutter/src/rust/mobile_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// [NodeApi] over a running `p2p_chat_server`: every call is one JSON
/// request frame on the WebSocket (`{id, method, params}` answered by
/// `{id, ok|err}`), attachment bytes go over `POST /api/upload`, and the
/// server pushes swarm events as `{"event": ...}` frames — this replaces
/// the client-side poll loop the in-process transport uses.
///
/// A dropped connection fails all in-flight calls; the next call lazily
/// reconnects (events pushed while disconnected are not replayed — screens
/// refresh on reconnect-worthy changes).
class HttpNodeApi implements NodeApi {
  HttpNodeApi({required Future<RemoteTransport> Function() connect})
    : _connect = connect;

  /// Connect to the node server serving this page (or [origin]), carrying
  /// [token] when the deployment requires one.
  factory HttpNodeApi.toServer({Uri? origin, String? token}) => HttpNodeApi(
    connect: () => WsRemoteTransport.connect(origin: origin, token: token),
  );

  final Future<RemoteTransport> Function() _connect;
  RemoteTransport? _transport;

  /// The single in-flight connection attempt, shared by concurrent callers
  /// (`startEventPump` + the first `_call`) so the transport's frames stream
  /// is listened to exactly once. Cleared on drop so reconnects re-run it.
  Future<void>? _connecting;
  StreamSubscription<String>? _framesSub;
  final Map<int, Completer<dynamic>> _pending = {};
  int _nextId = 0;
  void Function(SwarmEventJson event)? _onEvent;

  /// Rust-computed `formatTimeHhmm` results by input string. The function is
  /// pure, so a memo hit is exactly Rust's answer (see
  /// [formatTimeHhmmSync]).
  final Map<String, String> _timeFormatCache = {};

  @override
  bool get isRemoteTransport => true;

  // ── Event pump ────────────────────────────────────────────────────────────

  @override
  void startEventPump(void Function(SwarmEventJson event) onEvent) {
    _onEvent = onEvent;
    // Eagerly (best-effort) establish the connection so pushed events flow
    // even before the first request; failures surface on the next _call.
    unawaited(_ensureConnected().catchError((Object _) {}));
  }

  @override
  void stopEventPump() {
    _onEvent = null;
  }

  // ── Wire plumbing ─────────────────────────────────────────────────────────

  Future<void> _ensureConnected() {
    if (_transport != null) return Future<void>.value();
    final pending = _connecting;
    if (pending != null) return pending;
    final attempt = _connectAndListen();
    _connecting = attempt;
    // A failed attempt must not poison future calls: clear the shared future
    // (the original still delivers the error to whoever awaited it).
    unawaited(
      attempt.catchError((Object _) {
        if (identical(_connecting, attempt)) _connecting = null;
      }),
    );
    return attempt;
  }

  Future<void> _connectAndListen() async {
    final transport = await _connect();
    _transport = transport;
    _framesSub = transport.frames.listen(
      _onFrame,
      onError: (Object _) => _dropConnection(transport),
      onDone: () => _dropConnection(transport),
    );
    await transport.ready;
  }

  void _onFrame(String raw) {
    dynamic decoded;
    try {
      decoded = jsonDecode(raw);
    } on FormatException {
      return; // Non-protocol frame: ignore rather than kill the stream.
    }
    if (decoded is! Map<String, dynamic>) return;
    final event = decoded['event'];
    if (event is Map<String, dynamic>) {
      _onEvent?.call(eventFromJson(event));
      return;
    }
    final completer = _pending.remove(decoded['id']);
    if (completer == null) return;
    final error = decoded['err'];
    if (error != null) {
      completer.completeError(Exception('node call failed: $error'));
    } else {
      completer.complete(decoded['ok']);
    }
  }

  void _dropConnection(RemoteTransport transport) {
    if (!identical(_transport, transport)) return;
    unawaited(_framesSub?.cancel());
    _framesSub = null;
    _transport = null;
    _connecting = null;
    for (final completer in _pending.values) {
      completer.completeError(Exception('connection to the node closed'));
    }
    _pending.clear();
  }

  /// One request/response round trip over the WebSocket. Method and param
  /// names mirror the FRB function names (see `src/web/methods.rs`).
  Future<dynamic> _call(String method, [Map<String, dynamic>? params]) async {
    await _ensureConnected();
    final transport = _transport;
    if (transport == null) {
      throw Exception('not connected to the node');
    }
    final id = ++_nextId;
    final completer = Completer<dynamic>();
    _pending[id] = completer;
    await transport.send(
      jsonEncode({'id': id, 'method': method, 'params': params ?? const {}}),
    );
    try {
      // Generous bound: local-node calls are fast, but a wedged server must
      // not hang a screen forever.
      return await completer.future.timeout(const Duration(seconds: 60));
    } on TimeoutException {
      _pending.remove(id);
      rethrow;
    }
  }

  Future<Map<String, dynamic>> _upload(
    RemoteTransport transport,
    Uint8List bytes,
    String? name,
  ) async {
    final body = await transport.upload(
      name: name ?? 'attachment',
      bytes: bytes,
    );
    return body['attachment'] as Map<String, dynamic>;
  }

  // ── JSON -> generated model conversion (camelCase wire per the Rust
  //    serde rename_all; `attachment` is the snake_case persisted shape of
  //    files::MessageAttachment) ────────────────────────────────────────────

  static List<T> _list<T>(dynamic raw, T Function(dynamic) convert) =>
      (raw as List<dynamic>).map(convert).toList();

  /// `attachment` travels in both directions: messages carry the descriptor
  /// (snake_case keys, the persisted `attachment_json` shape). Uploads hand
  /// back the server's own descriptor map, so no Dart-side serializer exists.
  static MobileAttachment attachmentFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileAttachment(
      fileId: m['fileId'] as String,
      name: m['name'] as String,
      size: PlatformInt64Util.from(m['size'] as int),
      mime: m['mime'] as String?,
    );
  }

  static ChatMessage chatMessageFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return ChatMessage(
      id: m['id'] as int,
      content: m['content'] as String,
      peerId: m['peerId'] as String?,
      isBroadcast: m['isBroadcast'] as bool,
      targetPeer: m['targetPeer'] as String?,
      sent: m['sent'] as bool,
      msgId: m['msgId'] as String?,
      sentAt: m['sentAt'] as String?,
      createdAt: m['createdAt'] as String,
      senderNickname: m['senderNickname'] as String?,
      attachment: m['attachment'] == null
          ? null
          : attachmentFromJson(m['attachment']),
    );
  }

  static MobileGroup groupFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileGroup(
      groupId: m['groupId'] as String,
      displayName: m['displayName'] as String,
      memberCount: PlatformInt64Util.from(m['memberCount'] as int),
      isPrivate: m['isPrivate'] as bool,
    );
  }

  static MobileGroupMessage groupMessageFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileGroupMessage(
      id: m['id'] as int,
      content: m['content'] as String,
      peerId: m['peerId'] as String?,
      sent: m['sent'] as bool,
      msgId: m['msgId'] as String?,
      sentAt: m['sentAt'] as String?,
      createdAt: m['createdAt'] as String,
      senderNickname: m['senderNickname'] as String?,
      attachment: m['attachment'] == null
          ? null
          : attachmentFromJson(m['attachment']),
    );
  }

  static MobilePeerStatus peerStatusFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobilePeerStatus(
      databaseUrl: m['databaseUrl'] as String,
      localPeerId: m['localPeerId'] as String,
      selfNickname: m['selfNickname'] as String?,
    );
  }

  static MobileDeviceInfo deviceInfoFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileDeviceInfo(
      claimId: m['claimId'] as String,
      userId: m['userId'] as String,
      peerId: m['peerId'] as String,
      devicePubkey: Uint8List.fromList(
        (m['devicePubkey'] as List<dynamic>).cast<int>(),
      ),
      boundAt: m['boundAt'] as String,
    );
  }

  static PeerWithStats peerWithStatsFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return PeerWithStats(
      peerId: m['peerId'] as String,
      displayName: m['displayName'] as String,
      firstSeen: m['firstSeen'] as String,
      lastSeen: m['lastSeen'] as String,
      nickname: m['nickname'] as String?,
      localNickname: m['localNickname'] as String?,
      dmCount: PlatformInt64Util.from(m['dmCount'] as int),
      broadcastSentToPeer: PlatformInt64Util.from(
        m['broadcastSentToPeer'] as int,
      ),
    );
  }

  static MobileContactDevice contactDeviceFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileContactDevice(
      peerId: m['peerId'] as String,
      displayName: m['displayName'] as String,
      firstSeen: m['firstSeen'] as String,
      lastSeen: m['lastSeen'] as String,
      receivedNickname: m['receivedNickname'] as String?,
      localNickname: m['localNickname'] as String?,
    );
  }

  static MobileContactRow contactRowFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return MobileContactRow(
      contactId: m['contactId'] as String,
      isUser: m['isUser'] as bool,
      displayName: m['displayName'] as String,
      deviceCount: BigInt.from(m['deviceCount'] as int),
      deviceRows: _list(m['deviceRows'], contactDeviceFromJson),
      firstSeen: m['firstSeen'] as String,
      lastSeen: m['lastSeen'] as String,
      dmCount: PlatformInt64Util.from(m['dmCount'] as int),
      broadcastSentToPeer: PlatformInt64Util.from(
        m['broadcastSentToPeer'] as int,
      ),
    );
  }

  static ContactSortInput contactSortFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    return ContactSortInput(
      contactId: m['contactId'] as String,
      displayName: m['displayName'] as String,
      lastSeen: m['lastSeen'] as String,
      firstSeen: m['firstSeen'] as String,
      dmCount: PlatformInt64Util.from(m['dmCount'] as int),
      broadcastSentToPeer: PlatformInt64Util.from(
        m['broadcastSentToPeer'] as int,
      ),
    );
  }

  static Map<String, dynamic> contactSortToJson(ContactSortInput row) => {
    'contactId': row.contactId,
    'displayName': row.displayName,
    'lastSeen': row.lastSeen,
    'firstSeen': row.firstSeen,
    'dmCount': row.dmCount.toInt(),
    'broadcastSentToPeer': row.broadcastSentToPeer.toInt(),
  };

  static SwarmEventJson eventFromJson(dynamic raw) {
    final m = raw as Map<String, dynamic>;
    final pubkey = m['devicePubkey'];
    return SwarmEventJson(
      eventType: m['eventType'] as String,
      peerId: m['peerId'] as String?,
      content: m['content'] as String?,
      latency: m['latency'] as String?,
      nickname: m['nickname'] as String?,
      msgId: m['msgId'] as String?,
      address: m['address'] as String?,
      groupId: m['groupId'] as String?,
      devicePubkey: pubkey == null
          ? null
          : Uint8List.fromList((pubkey as List<dynamic>).cast<int>()),
    );
  }

  // ── Node lifecycle ────────────────────────────────────────────────────────

  @override
  Future<MobileInitStatus> initMobileDatabase({required String dbPath}) async {
    // The server owns its database; mirror the boot handshake without
    // touching its configuration.
    final status = await getMobilePeerStatus();
    return MobileInitStatus(
      databaseUrl: status.databaseUrl,
      localPeerId: status.localPeerId,
    );
  }

  @override
  Future<MobilePeerStatus> getMobilePeerStatus() async =>
      peerStatusFromJson(await _call('getMobilePeerStatus'));

  @override
  Future<String> startNode({required String dbPath}) async =>
      await _call('startNode', {'dbPath': dbPath}) as String;

  @override
  Future<String> startNodeAuto() async =>
      await _call('startNodeAuto') as String;

  @override
  Future<void> stopNode() => _call('stopNode');

  // ── Logs ──────────────────────────────────────────────────────────────────

  @override
  Future<List<String>> getLogs() async =>
      _list(await _call('getLogs'), (raw) => raw as String);

  @override
  Future<void> clearLogs() => _call('clearLogs');

  // ── Messages ──────────────────────────────────────────────────────────────

  @override
  Future<List<ChatMessage>> loadBroadcastMessages({required int limit}) async =>
      _list(
        await _call('loadBroadcastMessages', {'limit': limit}),
        chatMessageFromJson,
      );

  @override
  Future<List<ChatMessage>> loadDmMessages({
    required String peerId,
    required int limit,
  }) async => _list(
    await _call('loadDmMessages', {'peerId': peerId, 'limit': limit}),
    chatMessageFromJson,
  );

  @override
  Future<ChatMessage> saveOutgoingBroadcast({
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) async {
    Map<String, dynamic>? attachment;
    if (attachmentBytes != null) {
      await _ensureConnected();
      attachment = await _upload(_transport!, attachmentBytes, attachmentName);
    }
    return chatMessageFromJson(
      await _call('saveOutgoingBroadcast', {
        'content': content,
        'attachmentPath': ?attachmentPath,
        'attachment': ?attachment,
      }),
    );
  }

  @override
  Future<ChatMessage> saveOutgoingDm({
    required String peerId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) async {
    Map<String, dynamic>? attachment;
    if (attachmentBytes != null) {
      await _ensureConnected();
      attachment = await _upload(_transport!, attachmentBytes, attachmentName);
    }
    return chatMessageFromJson(
      await _call('saveOutgoingDm', {
        'peerId': peerId,
        'content': content,
        'attachmentPath': ?attachmentPath,
        'attachment': ?attachment,
      }),
    );
  }

  @override
  Future<ChatMessage> saveIncomingMessage({
    required String content,
    required String peerId,
    required bool isDirect,
    String? nickname,
  }) async => chatMessageFromJson(
    await _call('saveIncomingMessage', {
      'content': content,
      'peerId': peerId,
      'isDirect': isDirect,
      'nickname': ?nickname,
    }),
  );

  // ── Groups ────────────────────────────────────────────────────────────────

  @override
  Future<MobileGroup> createGroup({required String name}) async =>
      groupFromJson(await _call('createGroup', {'name': name}));

  @override
  Future<List<MobileGroup>> listGroups() async =>
      _list(await _call('listGroups'), groupFromJson);

  @override
  Future<List<MobileGroupMessage>> loadGroupMessages({
    required String groupId,
    required int limit,
  }) async => _list(
    await _call('loadGroupMessages', {'groupId': groupId, 'limit': limit}),
    groupMessageFromJson,
  );

  @override
  Future<MobileGroupMessage> saveOutgoingGroup({
    required String groupId,
    required String content,
    String? attachmentPath,
    Uint8List? attachmentBytes,
    String? attachmentName,
  }) async {
    Map<String, dynamic>? attachment;
    if (attachmentBytes != null) {
      await _ensureConnected();
      attachment = await _upload(_transport!, attachmentBytes, attachmentName);
    }
    return groupMessageFromJson(
      await _call('saveOutgoingGroup', {
        'groupId': groupId,
        'content': content,
        'attachmentPath': ?attachmentPath,
        'attachment': ?attachment,
      }),
    );
  }

  @override
  Future<MobileGroup> acceptGroupInvite({required String groupId}) async =>
      groupFromJson(await _call('acceptGroupInvite', {'groupId': groupId}));

  // ── Nickname / self ───────────────────────────────────────────────────────

  @override
  Future<void> setSelfNickname({required String nickname}) =>
      _call('setSelfNickname', {'name': nickname});

  @override
  Future<bool> validateNickname({required String nickname}) async =>
      await _call('validateNickname', {'name': nickname}) as bool;

  @override
  Future<String> networkSizeLabel({required int peerCount}) async =>
      await _call('networkSizeLabel', {'peerCount': peerCount}) as String;

  // ── Identity / device binding ─────────────────────────────────────────────

  @override
  Future<String?> getMyUserId() async =>
      await _call('getMyUserId') as String?;

  @override
  Future<List<MobileDeviceInfo>> getUserDevices({required String userId}) async =>
      _list(await _call('getUserDevices', {'userId': userId}), deviceInfoFromJson);

  @override
  Future<void> sendDeviceBindingIntro({required String peerId}) =>
      _call('sendDeviceBindingIntro', {'peerId': peerId});

  @override
  Future<void> acceptDeviceBinding({
    required String peerId,
    required Uint8List devicePubkey,
  }) => _call('acceptDeviceBinding', {
    'peerId': peerId,
    'devicePubkey': devicePubkey,
  });

  // ── Peers + contacts ──────────────────────────────────────────────────────

  @override
  Future<List<PeerWithStats>> getPeersWithStats() async => _list(
    await _call('getPeersWithStats'),
    peerWithStatsFromJson,
  );

  @override
  Future<List<MobileContactRow>> getContactsWithStats() async => _list(
    await _call('getContactsWithStats'),
    contactRowFromJson,
  );

  @override
  Future<List<ContactSortInput>> sortContacts({
    required List<ContactSortInput> contacts,
    required int sortColumn,
    required bool ascending,
  }) async => _list(
    await _call('sortContacts', {
      'contacts': [for (final row in contacts) contactSortToJson(row)],
      'sortColumn': sortColumn,
      'ascending': ascending,
    }),
    contactSortFromJson,
  );

  // ── Calls ─────────────────────────────────────────────────────────────────

  @override
  Future<String> startCall({required String peerId}) async =>
      await _call('startCall', {'peerId': peerId}) as String;

  @override
  Future<void> sendCallSignal({
    required String peerId,
    required String callId,
    required String kind,
  }) => _call('sendCallSignal', {'peerId': peerId, 'callId': callId, 'kind': kind});

  // ── Pure UI helpers (Rust is the single source) ───────────────────────────

  @override
  String? formatTimeHhmmSync({required String dt}) => _timeFormatCache[dt];

  @override
  Future<String> formatTimeHhmm({required String dt}) async {
    final value = await _call('formatTimeHhmm', {'dt': dt}) as String;
    _timeFormatCache[dt] = value;
    return value;
  }

  @override
  Future<bool> isAtBottom({
    required int scrollOffset,
    required int total,
    required int visible,
  }) async => await _call('isAtBottom', {
    'scrollOffset': scrollOffset,
    'total': total,
    'visible': visible,
  }) as bool;

  // ── Attachments ───────────────────────────────────────────────────────────

  @override
  Future<String> resolveAttachmentLocation(String fileId) async {
    await _ensureConnected();
    final transport = _transport;
    if (transport == null) {
      throw Exception('not connected to the node');
    }
    return transport.downloadUrl(fileId);
  }
}
