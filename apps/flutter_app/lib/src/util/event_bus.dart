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

/// Take over the sink while a screen is open, returning the callback it
/// replaced. The overriding screen must pass the returned value back to
/// [setEventSink] in its `dispose`, so the underlying screen (e.g. the home
/// view reached through Peer info → Direct message) resumes receiving events.
void Function(SwarmEventJson)? pushEventSink(
  void Function(SwarmEventJson) onEvent,
) {
  final previous = _onEvent;
  _onEvent = onEvent;
  return previous;
}

/// Poll `pollEvent()` every 200ms and forward all pending events to the sink.
///
/// A burst (e.g. many peers connecting at once) is drained to completion within
/// one tick instead of stalling at one event per 200ms.
void startEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = Timer.periodic(const Duration(milliseconds: 200), (_) async {
    try {
      for (var i = 0; i < 100; i++) {
        final event = await pollEvent();
        if (event == null) break;
        _onEvent?.call(event);
      }
    } catch (_) {}
  });
}

/// Stop the polling timer started by [startEventPolling].
void stopEventPolling() {
  _pollTimer?.cancel();
  _pollTimer = null;
}
