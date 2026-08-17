import 'package:flutter/services.dart';

class MobileHostStatus {
  const MobileHostStatus({
    required this.databaseUrl,
    required this.localPeerId,
    required this.serviceRunning,
    this.selfNickname,
  });

  final String databaseUrl;
  final String localPeerId;
  final bool serviceRunning;
  final String? selfNickname;

  factory MobileHostStatus.fromMap(Map<Object?, Object?> map) {
    return MobileHostStatus(
      databaseUrl: map['databaseUrl'] as String? ?? '',
      localPeerId: map['localPeerId'] as String? ?? '',
      serviceRunning: map['serviceRunning'] as bool? ?? false,
      selfNickname: map['selfNickname'] as String?,
    );
  }
}

class MobileHostApi {
  const MobileHostApi();

  static const _channel = MethodChannel('app.p2p/host');

  Future<MobileHostStatus> getStatus() async {
    final result = await _channel.invokeMapMethod<Object?, Object?>('getStatus');
    return MobileHostStatus.fromMap(result ?? const {});
  }

  Future<MobileHostStatus> startService() async {
    final result =
        await _channel.invokeMapMethod<Object?, Object?>('startService');
    return MobileHostStatus.fromMap(result ?? const {});
  }

  Future<MobileHostStatus> stopService() async {
    final result = await _channel.invokeMapMethod<Object?, Object?>('stopService');
    return MobileHostStatus.fromMap(result ?? const {});
  }
}
