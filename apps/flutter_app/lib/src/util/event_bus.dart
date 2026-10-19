import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// Single event sink for node events. Exactly one screen owns it at a time:
/// the home broadcast view by default, or the DM chat screen while it is
/// open (so DMs route there instead).
void Function(SwarmEventJson)? _onEvent;

/// Install the callback that receives each node [SwarmEventJson].
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

/// Start delivering node events to the installed sink. The in-process
/// transport polls `pollEvent()` every 200ms; the remote transport receives
/// server-pushed frames — both funnel through [NodeApi.startEventPump], so
/// screens never care which one is active.
void startEventPolling() {
  nodeApi.startEventPump((event) => _onEvent?.call(event));
}

/// Stop the event pump (e.g. before `stopNode`, so no frames are polled
/// after the node is gone).
void stopEventPolling() {
  nodeApi.stopEventPump();
}
