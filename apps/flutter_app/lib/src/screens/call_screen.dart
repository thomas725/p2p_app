import 'dart:async';

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';
import 'package:p2p_app_flutter/src/screens/nav_bar.dart';
import 'package:p2p_app_flutter/src/util/call_audio_device.dart';
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

/// Test/platform seams for the Rust-owned media session (calls lane, US-044).
///
/// The commands normally go through [NodeApi] to Rust. The `audio` callback is
/// an extra observer for decoded peer PCM: the real [CallAudioAdapter] always
/// receives it too, so widget tests can assert the frames without a device.
/// Tests install these so they never load the Rust library.
@visibleForTesting
class CallMediaOverrides {
  const CallMediaOverrides({
    this.start,
    this.stop,
    this.sendPcm,
    this.mute,
    this.audio,
  });

  final FutureOr<void> Function(String callId, String peerId)? start;
  final FutureOr<void> Function()? stop;
  final FutureOr<void> Function(String callId, List<int> pcm)? sendPcm;
  final FutureOr<void> Function(bool muted)? mute;

  /// Receives decoded 16 kHz mono i16 PCM from the peer.
  final void Function(List<int> pcm)? audio;
}

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
    this.mediaOverrides,
    this.audioAdapter,
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

  /// Test-only seams for the media session and its audio callbacks.
  @visibleForTesting
  final CallMediaOverrides? mediaOverrides;

  /// Device microphone/speaker adapter. Defaults to the platform
  /// implementation (browser Web Audio on web, a no-op on native); tests
  /// inject a fake so no audio device is touched.
  @visibleForTesting
  final CallAudioAdapter? audioAdapter;

  @override
  State<CallScreen> createState() => CallScreenState();
}

class CallScreenState extends State<CallScreen> {
  // Sink installed while this screen is open; restored on dispose so the view
  // underneath keeps receiving events (e.g. an open DM chat).
  void Function(SwarmEventJson)? _previousSink;

  late CallScreenPhase _phase;

  // Media transport is opened only once the call is live and closed on every
  // exit path. The counter mirrors the TUI overlay and is the visible proof
  // that peer audio is flowing even on platforms with no device adapter.
  bool _mediaStarted = false;
  bool _muted = false;
  int _audioFrames = 0;

  /// Microphone/speaker half of the call. Rust still owns framing, jitter and
  /// mute; this only moves samples to and from the OS/browser.
  late final CallAudioAdapter _audioAdapter =
      widget.audioAdapter ?? createCallAudioAdapter();

  @override
  void initState() {
    super.initState();
    _phase = widget.initialPhase;
    _previousSink = pushEventSink(_handleSwarmEvent);
    if (_phase == CallScreenPhase.inCall) unawaited(_startMedia());
  }

  @override
  void dispose() {
    unawaited(_stopMedia());
    setEventSink(_previousSink);
    super.dispose();
  }

  /// Run one media command through its test seam, or the Rust API. A failed
  /// command never takes the call UI down: signaling remains usable and the
  /// swarm loop tears the session down when the call ends.
  Future<void> _media(FutureOr<void> Function() action) async {
    try {
      await action();
    } catch (e) {
      debugPrint('Call media command failed: $e');
    }
  }

  Future<void> _startMedia() async {
    if (_mediaStarted) return;
    _mediaStarted = true;
    final override = widget.mediaOverrides?.start;
    await _media(() => override != null
        ? override(widget.callId, widget.peerId)
        : nodeApi.startCallMedia(callId: widget.callId, peerId: widget.peerId));
    await _media(
      () => _audioAdapter.start(onPcm: (pcm) => unawaited(sendPcm(pcm))),
    );
    // Capture can fail for reasons the user can act on (permission, HTTPS);
    // surface that in the status line once the adapter has settled.
    if (mounted) setState(() {});
  }

  Future<void> _stopMedia() async {
    if (!_mediaStarted) return;
    _mediaStarted = false;
    final override = widget.mediaOverrides?.stop;
    await _media(
      () => override != null ? override() : nodeApi.stopCallMedia(),
    );
    await _media(_audioAdapter.stop);
  }

  /// Hand one 16 kHz mono PCM block from the microphone adapter to Rust.
  /// The session filter lives in Rust, so a stale call's audio is ignored
  /// there; this guard just avoids pointless FFI traffic.
  Future<void> sendPcm(List<int> pcm) {
    if (_phase != CallScreenPhase.inCall || pcm.isEmpty) {
      return Future<void>.value();
    }
    final override = widget.mediaOverrides?.sendPcm;
    return _media(() => override != null
        ? override(widget.callId, pcm)
        : nodeApi.sendCallMedia(callId: widget.callId, pcm: pcm));
  }

  Future<void> _toggleMute() async {
    final muted = !_muted;
    if (mounted) setState(() => _muted = muted);
    final override = widget.mediaOverrides?.mute;
    await _media(
      () => override != null ? override(muted) : nodeApi.setCallMediaMute(muted: muted),
    );
  }

  /// Send a call signal over the calls lane (accept/reject/end), using the
  /// test seam when one is installed.
  Future<void> _sendSignal(String kind) {
    final override = widget.sendSignalOverride;
    if (override != null) {
      return Future.sync(() => override(widget.peerId, widget.callId, kind));
    }
    return nodeApi.sendCallSignal(
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
      'call_invite' ||
      'call_accepted' ||
      'call_rejected' ||
      'call_ended' ||
      'call_media' =>
        true,
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
    if (event.eventType == 'call_media') {
      _acceptPeerAudio(event.audio);
      return;
    }
    _applySessionEvent(event.eventType);
  }

  /// Deliver decoded peer PCM to the speaker (and the test seam) and refresh
  /// the status line at most once per second — not once per 20 ms frame.
  void _acceptPeerAudio(List<int>? pcm) {
    if (pcm == null || pcm.isEmpty) return;
    widget.mediaOverrides?.audio?.call(pcm);
    _audioAdapter.play(pcm);
    _audioFrames++;
    if (_audioFrames % 50 == 1 && mounted) setState(() {});
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
        await nodeApi.sendCallSignal(peerId: peerId, callId: callId, kind: 'reject');
      }
    } catch (e) {
      debugPrint('Failed to auto-decline second call: $e');
    }
  }

  void _applySessionEvent(String eventType) {
    switch (eventType) {
      case 'call_accepted':
        setState(() => _phase = CallScreenPhase.inCall);
        unawaited(_startMedia());
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
    unawaited(_stopMedia());
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
    unawaited(_startMedia());
  }

  void _leaveTab(int index) {
    unawaited(_sendSignal('end'));
    unawaited(_stopMedia());
    Navigator.of(context).pop();
    widget.onNavigate?.call(index);
  }

  @override
  Widget build(BuildContext context) {
    final captureError = _audioAdapter.captureError;
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
            key: const Key('mute-toggle'),
            label: _muted ? 'Unmute' : 'Mute',
            icon: _muted ? Icons.mic_off : Icons.mic,
            color: Colors.blue,
            onPressed: () => unawaited(_toggleMute()),
          ),
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
                if (_phase == CallScreenPhase.inCall)
                  Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: Column(
                      children: [
                        Text(
                          'Mic ${_muted ? 'muted' : 'on'} · peer frames $_audioFrames',
                          style: Theme.of(context).textTheme.bodySmall,
                          textAlign: TextAlign.center,
                        ),
                        if (captureError != null)
                          Text(
                            captureError,
                            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                                  color: Theme.of(context).colorScheme.error,
                                ),
                            textAlign: TextAlign.center,
                          ),
                      ],
                    ),
                  ),
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