// JSON mapping for the Rust location DTOs over the web transport.
//
// FRB 2.13 generates no `fromJson`/`toJson`, so the HTTP transport keeps the
// conversion here. Field names mirror Rust's `serde(rename_all = "camelCase")`
// wire shape exactly; numbers cross as JSON numbers on both platforms.

import 'package:p2p_app_flutter/src/rust/location.dart'
    show LocationRecipient, LocationTrackPoint;
import 'package:p2p_app_flutter/src/rust/settings.dart' show LocationDefaults;

LocationDefaults locationDefaultsFromJson(Map<String, dynamic> m) =>
    LocationDefaults(
      mode: m['mode'] as String,
      durationSecs: m['durationSecs'] == null
          ? null
          : (m['durationSecs'] as num).toInt(),
      sensorIntervalSecs: (m['sensorIntervalSecs'] as num).toInt(),
      sendIntervalSecs: (m['sendIntervalSecs'] as num).toInt(),
      motionGate: m['motionGate'] as bool,
    );

Map<String, dynamic> locationDefaultsToJson(LocationDefaults defaults) => {
  'mode': defaults.mode,
  'durationSecs': ?defaults.durationSecs,
  'sensorIntervalSecs': defaults.sensorIntervalSecs,
  'sendIntervalSecs': defaults.sendIntervalSecs,
  'motionGate': defaults.motionGate,
};

LocationTrackPoint locationTrackPointFromJson(Map<String, dynamic> m) =>
    LocationTrackPoint(
      latitude: (m['latitude'] as num).toDouble(),
      longitude: (m['longitude'] as num).toDouble(),
      accuracy: (m['accuracy'] as num).toDouble(),
      timestampSecs: (m['timestampSecs'] as num).toDouble(),
      sequence: (m['sequence'] as num).toInt(),
      isFinal: m['isFinal'] as bool,
    );

Map<String, dynamic> locationTrackPointToJson(LocationTrackPoint point) => {
  'latitude': point.latitude,
  'longitude': point.longitude,
  'accuracy': point.accuracy,
  'timestampSecs': point.timestampSecs,
  'sequence': point.sequence,
  'isFinal': point.isFinal,
};

LocationRecipient locationRecipientFromJson(Map<String, dynamic> m) =>
    LocationRecipient(
      peerId: m['peerId'] as String,
      displayName: m['displayName'] as String,
    );
