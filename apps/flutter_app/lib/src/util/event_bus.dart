import 'dart:async';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// Single event sink for polled node events. Exactly one screen owns it at a
/// time: the home broadcast view by default, or the DM chat screen while it is
/// open (so DMs route there instead).
void Function(SwarmEventJson)? _onEvent;
Timer? _pollTimer;

/// Install the callback that receives each polled [SwarmEventJson].
void setEventSink(void Function(SwarmEventJson)? onEvent) {
  _onEvent = onEvent;
}

/// Poll `pollEvent()` every 200ms and forward non-null events to the sink.
void startEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
    try {
      final event = await pollEvent();
      if (event != null) _onEvent?.call(event);
    } catch (_) {}
  });
}

/// Stop the polling timer started by [startEventPolling].
void stopEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = null;
}
