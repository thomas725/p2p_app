// Location-sharing dialog for a DM.
//
// Rust owns the defaults, their validation, recipient eligibility, cadence and
// the canonical message body. This screen only renders the Rust-supplied
// values, persists edits through `setLocationDefaults` and reports the
// user's choices back to the DM screen, which owns the live session. The DM
// partner is deliberately not added to the recipient list: only devices bound
// into this install's user (returned by `listLocationRecipients`) may receive
// a location.

import 'dart:async' show unawaited;

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/rust/location.dart' show LocationRecipient;
import 'package:p2p_app_flutter/src/rust/settings.dart' show LocationDefaults;
import 'package:p2p_app_flutter/src/util/location.dart'
    show LocationAdapter, locationDeviceUnavailableReason;
import 'package:p2p_app_flutter/src/util/location_session.dart'
    show LocationShareConfig;

/// Dialog asking where, how often and to which bound devices to share.
class LocationShareDialog extends StatefulWidget {
  const LocationShareDialog({
    super.key,
    required this.adapter,
    required this.loadDefaults,
    required this.saveDefaults,
    required this.loadRecipients,
    required this.onOnce,
    required this.onStart,
    required this.onStop,
    this.active = false,
    this.activeLabel,
  });

  /// The same adapter the session will use, so permission/unavailable reasons
  /// shown here describe the actual share.
  final LocationAdapter adapter;
  final Future<LocationDefaults> Function() loadDefaults;
  final Future<void> Function(LocationDefaults input) saveDefaults;
  final Future<List<LocationRecipient>> Function() loadRecipients;

  /// Each callback starts (or stops) the session in the owning DM screen and
  /// throws on failure so the dialog can keep the form open for correction.
  final Future<void> Function(LocationShareConfig config) onOnce;
  final Future<void> Function(LocationShareConfig config) onStart;
  final Future<void> Function() onStop;

  final bool active;
  final String? activeLabel;

  @override
  State<LocationShareDialog> createState() => _LocationShareDialogState();
}

class _LocationShareDialogState extends State<LocationShareDialog> {
  LocationDefaults? _defaults;
  List<LocationRecipient> _recipients = const [];
  final Set<String> _selected = <String>{};
  bool _allDevices = true;
  bool _busy = false;
  String? _error;

  late final TextEditingController _duration;
  late final TextEditingController _sensorInterval;
  late final TextEditingController _sendInterval;

  static const _modes = ['once', 'limited', 'unlimited'];

  @override
  void initState() {
    super.initState();
    _duration = TextEditingController();
    _sensorInterval = TextEditingController();
    _sendInterval = TextEditingController();
    unawaited(_load());
  }

  @override
  void dispose() {
    _duration.dispose();
    _sensorInterval.dispose();
    _sendInterval.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final defaults = await widget.loadDefaults();
      final recipients = await widget.loadRecipients();
      if (!mounted) return;
      setState(() {
        _defaults = defaults;
        _recipients = recipients;
        _fillFrom(defaults);
      });
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = 'Could not load location settings: $e');
    }
  }

  void _fillFrom(LocationDefaults defaults) {
    _duration.text = defaults.durationSecs?.toString() ?? '';
    _sensorInterval.text = defaults.sensorIntervalSecs.toString();
    _sendInterval.text = defaults.sendIntervalSecs.toString();
  }

  /// Bounds and grammar are Rust's. A value we cannot parse is sent to Rust as
  /// an out-of-range sentinel so it gets the same rejection a TUI user sees.
  int _interval(String text) => int.tryParse(text.trim()) ?? -1;

  String get _mode => _defaults?.mode ?? 'once';

  bool get _motionGate => _defaults?.motionGate ?? true;

  LocationDefaults _candidate() => LocationDefaults(
    mode: _mode,
    durationSecs: _mode == 'limited' ? _interval(_duration.text) : null,
    sensorIntervalSecs: _interval(_sensorInterval.text),
    sendIntervalSecs: _interval(_sendInterval.text),
    motionGate: _motionGate,
  );

  /// Persist every user edit through Rust; the returned defaults are not
  /// synthesised in Dart, only the candidate values are echoed on failure.
  Future<bool> _persist() async {
    final candidate = _candidate();
    try {
      await widget.saveDefaults(candidate);
      if (!mounted) return false;
      setState(() {
        _defaults = candidate;
        _error = null;
      });
      return true;
    } catch (e) {
      if (!mounted) return false;
      setState(() => _error = 'Location settings not saved: $e');
      return false;
    }
  }

  void _commitSettings() => unawaited(_persist());

  void _setMode(String? mode) {
    if (mode == null) return;
    setState(() {
      _defaults = _candidate().mode == mode
          ? _defaults
          : LocationDefaults(
              mode: mode,
              durationSecs: mode == 'limited'
                  ? _interval(_duration.text)
                  : null,
              sensorIntervalSecs: _interval(_sensorInterval.text),
              sendIntervalSecs: _interval(_sendInterval.text),
              motionGate: _motionGate,
            );
    });
    _commitSettings();
  }

  void _setMotionGate(bool value) {
    setState(() {
      _defaults = LocationDefaults(
        mode: _mode,
        durationSecs: _candidate().durationSecs,
        sensorIntervalSecs: _interval(_sensorInterval.text),
        sendIntervalSecs: _interval(_sendInterval.text),
        motionGate: value,
      );
    });
    _commitSettings();
  }

  void _select(String peerId, bool? selected) {
    setState(() {
      if (selected == true) {
        _selected.add(peerId);
        _allDevices = false;
      } else {
        _selected.remove(peerId);
      }
    });
  }

  LocationShareConfig _config({required bool once}) =>
      LocationShareConfig.fromDefaults(
        _candidate(),
        selectedPeerIds: _selected.toList(),
        allDevices: _allDevices,
        modeOverride: once ? 'once' : null,
      );

  Future<void> _submit({required bool once}) async {
    if (_busy || _defaults == null || (widget.active && !once)) return;
    setState(() => _busy = true);
    try {
      if (!await _persist()) return;
      final config = _config(once: once);
      if (once) {
        await widget.onOnce(config);
      } else {
        await widget.onStart(config);
      }
      if (!mounted) return;
      Navigator.pop(context, config);
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = 'Could not share location: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  Future<void> _stop() async {
    if (_busy) return;
    setState(() => _busy = true);
    try {
      await widget.onStop();
      if (!mounted) return;
      Navigator.pop(context);
    } catch (e) {
      if (!mounted) return;
      setState(() => _error = 'Could not stop sharing: $e');
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    final unavailable = locationDeviceUnavailableReason(widget.adapter);
    final loading = _defaults == null;

    return AlertDialog(
      title: const Text('Share location'),
      content: SizedBox(
        width: double.maxFinite,
        child: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              if (widget.active)
                Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Text(
                    widget.activeLabel ?? 'Location sharing is active.',
                    key: const Key('location-share-active'),
                    style: TextStyle(color: cs.primary),
                  ),
                ),
              if (loading)
                const Padding(
                  padding: EdgeInsets.symmetric(vertical: 24),
                  child: Center(child: CircularProgressIndicator()),
                )
              else ...[
                DropdownButtonFormField<String>(
                  key: const Key('location-share-mode'),
                  initialValue: _mode,
                  decoration: const InputDecoration(labelText: 'Mode'),
                  items: [
                    for (final mode in _modes)
                      DropdownMenuItem(value: mode, child: Text(mode)),
                  ],
                  onChanged: widget.active ? null : _setMode,
                ),
                if (_mode == 'limited')
                  Padding(
                    padding: const EdgeInsets.only(top: 8),
                    child: TextField(
                      key: const Key('location-share-duration'),
                      controller: _duration,
                      keyboardType: TextInputType.number,
                      enabled: !widget.active,
                      decoration: const InputDecoration(
                        labelText: 'Duration (seconds)',
                        border: OutlineInputBorder(),
                      ),
                      onSubmitted: (_) => _commitSettings(),
                    ),
                  ),
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: TextField(
                    key: const Key('location-share-sensor-interval'),
                    controller: _sensorInterval,
                    keyboardType: TextInputType.number,
                    enabled: !widget.active,
                    decoration: const InputDecoration(
                      labelText: 'Sensor interval (seconds)',
                      border: OutlineInputBorder(),
                    ),
                    onSubmitted: (_) => _commitSettings(),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: TextField(
                    key: const Key('location-share-send-interval'),
                    controller: _sendInterval,
                    keyboardType: TextInputType.number,
                    enabled: !widget.active,
                    decoration: const InputDecoration(
                      labelText: 'Send interval (seconds)',
                      border: OutlineInputBorder(),
                    ),
                    onSubmitted: (_) => _commitSettings(),
                  ),
                ),
                SwitchListTile(
                  key: const Key('location-share-motion-gate'),
                  value: _motionGate,
                  title: const Text('Low-power motion gate'),
                  subtitle: const Text('Query the sensor only while moving'),
                  onChanged: widget.active
                      ? null
                      : (_) => _setMotionGate(!_motionGate),
                ),
                const SizedBox(height: 8),
                Text(
                  'Destinations are your other bound devices, not this chat partner.',
                  style: TextStyle(fontSize: 12, color: cs.onSurfaceVariant),
                ),
                SwitchListTile(
                  key: const Key('location-share-all-devices'),
                  value: _allDevices,
                  title: const Text('All bound devices'),
                  onChanged: widget.active
                      ? null
                      : (value) => setState(() {
                          _allDevices = value;
                          if (value) _selected.clear();
                        }),
                ),
                for (final recipient in _recipients)
                  CheckboxListTile(
                    key: Key('location-share-recipient-${recipient.peerId}'),
                    value: _allDevices || _selected.contains(recipient.peerId),
                    title: Text(recipient.displayName),
                    subtitle: Text(recipient.peerId),
                    onChanged: widget.active
                        ? null
                        : (value) => _select(recipient.peerId, value),
                  ),
                if (_recipients.isEmpty)
                  const Text(
                    'No other device is bound into your account yet.',
                    key: Key('location-share-no-recipients'),
                  ),
              ],
              if (unavailable != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    unavailable,
                    key: const Key('location-share-device-error'),
                    style: TextStyle(color: cs.error),
                  ),
                ),
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    _error!,
                    key: const Key('location-share-error'),
                    style: TextStyle(color: cs.error),
                  ),
                ),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: const Text('Cancel'),
        ),
        FilledButton.tonal(
          key: const Key('location-share-once'),
          onPressed: _busy || loading || widget.active
              ? null
              : () => unawaited(_submit(once: true)),
          child: const Text('Share once'),
        ),
        if (widget.active)
          FilledButton(
            key: const Key('location-share-stop'),
            style: FilledButton.styleFrom(backgroundColor: cs.error),
            onPressed: _busy ? null : () => unawaited(_stop()),
            child: const Text('Stop sharing'),
          )
        else
          FilledButton(
            key: const Key('location-share-start'),
            onPressed: _busy || loading || _mode == 'once'
                ? null
                : () => unawaited(_submit(once: false)),
            child: const Text('Start sharing'),
          ),
      ],
    );
  }
}
