import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/event_bus.dart';

/// How far along a call session is, from the local side's perspective.
enum CallScreenPhase {
  /// An incoming invite is ringing; the user can accept or decline.
  ringing,

  /// We placed an outgoing invite; the user can cancel while it rings.
  calling,

  /// Both sides accepted; the user can hang up.
  inCall,
}

/// Test-only seam for [CallScreen]: replaces the FFI-backed `sendCallSignal`
/// call so widget tests can exercise the accept/decline/cancel/hang-up actions
/// without a loaded Rust library.
@visibleForTesting
typedef SendCallSignalOverride =
    FutureOr<void> Function(String peerId, String callId, String kind);

/// Full-screen call session UI. Takes over the event sink while open (like the
/// DM chat screen) so call-signaling events route here, filtering session
/// events on `peer_id` + the echoed `call_id` (`content`) and forwarding
/// everything else to the sink it replaced. One session at a time: a second
/// incoming invite is auto-declined while a call is already active.
class CallScreen extends StatefulWidget {
  const CallScreen({
    super.key,
    required this.peerId,
    required this.displayName,
    required this.callId,
    required this.initialPhase,
    this.onNavigate,
    this.currentTab = 1,
    this.sendSignalOverride,
  });

  /// The remote peer on the other side of this session.
  final String peerId;

  /// Display name for the remote peer (header/copy).
  final String displayName;

  /// End-to-end session id, echoed by every call signal and event.
  final String callId;

  /// Where this session starts (ringing for an incoming invite, calling for an
  /// outgoing one).
  final CallScreenPhase initialPhase;

  /// Bottom-tab navigation, mirroring the other full-screen routes. Calls are
  /// transient; selecting a tab hangs up and pops.
  final void Function(int)? onNavigate;

  /// The main-tab index to mark active in the bottom nav.
  final int currentTab;

  /// Test-only seam replacing the FFI-backed `sendCallSignal` call.
  @visibleForTesting
  final SendCallSignalOverride? sendSignalOverride;

  @override
  State<CallScreen> createState() => CallScreenState();
}

class CallScreenState extends State<CallScreen> with WidgetsBindingObserver {
  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events (e.g. an open DM chat).
  void Function(SwarmEventJson)? _previousSink;

  late CallScreenPhase _phase;

  @override
  void initState() {
    super.initState();
    _phase = widget.initialPhase;
    _previousSink = pushEventSink(_handleSwarmEvent);
  }

  @override
  void dispose() {
    setEventSink(_previousSink);
    super.dispose();
  }

  /// Send a call signal over the calls lane (accept/reject/end), using the
  /// test seam when one is installed.
  Future<void> _sendSignal(String kind) {
    final override = widget.sendSignalOverride;
    if (override != null) {
      return Future.sync(() => override(widget.peerId, widget.callId, kind));
    }
    return sendCallSignal(
      peerId: widget.peerId,
      callId: widget.callId,
      kind: kind,
    );
  }

  /// Route a polled swarm event. Accepts this session's call events and
  /// forwards everything else (other call sessions included) downstream.
  void _handleSwarmEvent(SwarmEventJson event) {
    if (!mounted) return;
    final isCallEvent = switch (event.eventType) {
      'call_invite' || 'call_accepted' || 'call_rejected' || 'call_ended' => true,
      _ => false,
    };
    if (!isCallEvent) {
      _previousSink?.call(event);
      return;
    }
    // Only events addressed to *this* session's call_id (and peer) mutate
    // state; anything else is a second, concurrent session we don't own.
    final ours =
        event.peerId == widget.peerId && event.content == widget.callId;
    if (!ours) {
      // Auto-decline a second incoming call: one session at a time.
      if (event.eventType == 'call_invite' &&
          event.peerId != null &&
          event.content != null) {
        unawaited(_declineOtherCall(event.peerId!, event.content!));
      }
      return;
    }
    _applySessionEvent(event.eventType);
  }

  /// Accept this session's call events and forward everything else downstream.
  ///
  /// Test-only entry: widget tests deliver synthetic [SwarmEventJson]s through
  /// the exact same handler the poll loop installs as the event sink.
  @visibleForTesting
  void deliverEvent(SwarmEventJson event) => _handleSwarmEvent(event);

  Future<void> _declineOtherCall(String peerId, String callId) async {
    try {
      final override = widget.sendSignalOverride;
      if (override != null) {
        await override(peerId, callId, 'reject');
      } else {
        await sendCallSignal(peerId: peerId, callId: callId, kind: 'reject');
      }
    } catch (e) {
      debugPrint('Failed to auto-decline second call: $e');
    }
  }

  void _applySessionEvent(String eventType) {
    switch (eventType) {
      case 'call_accepted':
        setState(() => _phase = CallScreenPhase.inCall);
        break;
      case 'call_rejected':
        _endSession('Call declined');
        break;
      case 'call_ended':
        _endSession('Call ended');
        break;
    }
  }

  /// Hang up / decline / cancel, notifying the peer over the calls lane.
  Future<void> _terminate(String kind, String message) async {
    await _sendSignal(kind);
    _endSession(message);
  }

  /// Pop the screen (with an optional note), clearing the live session.
  void _endSession([String? message]) {
    if (!mounted) return;
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message ?? 'Call ended'),
        duration: const Duration(seconds: 2),
      ),
    );
    Navigator.of(context).pop();
  }

  void _accept() {
    unawaited(_sendSignal('accept'));
    setState(() => _phase = CallScreenPhase.inCall);
  }

  void _leaveTab(int index) {
    unawaited(_sendSignal('end'));
    Navigator.of(context).pop();
    widget.onNavigate?.call(index);
  }

  @override
  Widget build(BuildContext context) {
    final (lines, buttons) = switch (_phase) {
      CallScreenPhase.ringing => (
        (
          'Incoming call',
          '${widget.displayName} is calling',
          Icons.phone,
        ),
        [
          _CallButton(
            key: const Key('decline'),
            label: 'Decline',
            icon: Icons.call_end,
            color: Colors.red,
            onPressed: () => unawaited(_terminate('reject', 'Call declined')),
          ),
          _CallButton(
            key: const Key('accept'),
            label: 'Accept',
            icon: Icons.call,
            color: Colors.green,
            onPressed: _accept,
          ),
        ],
      ),
      CallScreenPhase.calling => (
        (
          'Calling',
          widget.displayName,
          Icons.phone_in_talk,
        ),
        [
          _CallButton(
            key: const Key('cancel'),
            label: 'Cancel',
            icon: Icons.call_end,
            color: Colors.red,
            onPressed: () => unawaited(_terminate('end', 'Call cancelled')),
          ),
        ],
      ),
      CallScreenPhase.inCall => (
        (
          'In call',
          widget.displayName,
          Icons.phone,
        ),
        [
          _CallButton(
            key: const Key('hang-up'),
            label: 'Hang up',
            icon: Icons.call_end,
            color: Colors.red,
            onPressed: () => unawaited(_terminate('end', 'Call ended')),
          ),
        ],
      ),
    };

    return Scaffold(
      appBar: AppBar(title: Text(widget.displayName)),
      body: Column(
        children: [
          Expanded(
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(lines.$3, size: 72, color: Theme.of(context).colorScheme.primary),
                const SizedBox(height: 16),
                Text(lines.$1, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 8),
                Text(lines.$2, style: Theme.of(context).textTheme.bodyMedium),
              ],
            ),
          ),
          Padding(
            padding: const EdgeInsets.only(bottom: 32),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: buttons,
            ),
          ),
        ],
      ),
      bottomNavigationBar: MainTabNavBar(
        currentIndex: widget.currentTab,
        onSelect: _leaveTab,
      ),
    );
  }
}

class _CallButton extends StatelessWidget {
  const _CallButton({
    super.key,
    required this.label,
    required this.icon,
    required this.color,
    required this.onPressed,
  });
  final String label;
  final IconData icon;
  final Color color;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          IconButton(
            icon: Icon(icon, color: color),
            iconSize: 40,
            tooltip: label,
            onPressed: onPressed,
          ),
          Text(label, style: Theme.of(context).textTheme.labelMedium),
        ],
      ),
    );
  }
}