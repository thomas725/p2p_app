// One sending device's received location track.
//
// The track is always scoped to exactly one sender peer id: two devices bound
// into the same account must never mix their samples. Rust owns the relative /
// ISO time filter grammar, the GPX document and the capped map URL; this screen
// passes the raw field text straight through and renders what it gets back.

import 'dart:async' show unawaited;

import 'package:flutter/material.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/location.dart' show LocationTrackPoint;
import 'package:p2p_app_flutter/src/util/attachments.dart'
    show openRemoteUrlOverride;
import 'package:p2p_app_flutter/src/util/location_export.dart' show saveGpxText;

/// Full-screen track view for one sending device.
class LocationTrackScreen extends StatefulWidget {
  const LocationTrackScreen({
    super.key,
    required this.peerId,
    this.deviceLabel,
  });

  /// Peer id of the device whose DMs carried the samples.
  final String peerId;

  /// Display label for that device (Rust's canonical name when available).
  final String? deviceLabel;

  @override
  State<LocationTrackScreen> createState() => _LocationTrackScreenState();
}

class _LocationTrackScreenState extends State<LocationTrackScreen> {
  final _from = TextEditingController();
  final _to = TextEditingController();
  List<LocationTrackPoint> _points = const [];
  String? _url;
  String? _error;
  bool _loading = false;

  String get _label => widget.deviceLabel ?? widget.peerId;

  @override
  void initState() {
    super.initState();
    unawaited(_load());
  }

  @override
  void dispose() {
    _from.dispose();
    _to.dispose();
    super.dispose();
  }

  /// Load through Rust, explicitly passing the raw filter text. No Dart-side
  /// date parsing or coordinate math: Rust decides what the text means.
  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final points = await nodeApi.loadLocationTrack(
        peerId: widget.peerId,
        fromText: _from.text.trim().isEmpty ? null : _from.text.trim(),
        toText: _to.text.trim().isEmpty ? null : _to.text.trim(),
      );
      String? url;
      if (points.isNotEmpty) {
        try {
          url = await nodeApi.locationTrackUrl(points: points);
        } catch (_) {
          // A track is still usable without its optional map link (and test
          // doubles need not configure every seam).
        }
      }
      if (!mounted) return;
      setState(() {
        _points = points;
        _url = url;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _loading = false;
        _error = 'Could not load location track: $e';
      });
    }
  }

  Future<void> _openMap() async {
    final url = _url;
    if (url == null) return;
    final error = await openRemoteUrlOverride(url);
    if (!mounted) return;
    _showMessage(error ?? 'Opening map for $_label');
  }

  /// Rust builds the GPX; Dart only encodes it and calls the save seam.
  Future<void> _exportGpx() async {
    if (_points.isEmpty) return;
    try {
      final gpx = await nodeApi.locationGpx(
        points: _points,
        deviceLabel: _label,
      );
      final saved = await saveGpxText(
        fileName: 'location-${widget.peerId}.gpx',
        gpx: gpx,
      );
      if (!mounted || saved == null) return;
      _showMessage('Saved $saved');
    } catch (e) {
      if (!mounted) return;
      _showMessage('Could not export GPX: $e');
    }
  }

  void _showMessage(String message) {
    ScaffoldMessenger.maybeOf(context)
      ?..clearSnackBars()
      ..showSnackBar(
        SnackBar(content: Text(message), duration: const Duration(seconds: 2)),
      );
  }

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Scaffold(
      appBar: AppBar(
        title: const Text('Location track'),
        actions: [
          IconButton(
            key: const Key('location-track-export'),
            onPressed: _points.isEmpty ? null : () => unawaited(_exportGpx()),
            icon: const Icon(Icons.download_outlined),
            tooltip: 'Export GPX',
          ),
        ],
      ),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(
                'Sender: $_label',
                style: TextStyle(color: cs.onSurfaceVariant, fontSize: 12),
              ),
              Row(
                children: [
                  Expanded(
                    child: TextField(
                      key: const Key('location-track-from'),
                      controller: _from,
                      decoration: const InputDecoration(
                        labelText: 'From (Rust-parsed)',
                        isDense: true,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: TextField(
                      key: const Key('location-track-to'),
                      controller: _to,
                      decoration: const InputDecoration(
                        labelText: 'To (Rust-parsed)',
                        isDense: true,
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  FilledButton(
                    key: const Key('location-track-load'),
                    onPressed: _loading ? null : () => unawaited(_load()),
                    child: const Text('Load'),
                  ),
                ],
              ),
              const SizedBox(height: 10),
              Text(
                _loading
                    ? 'Loading…'
                    : '${_points.length} point'
                          '${_points.length == 1 ? '' : 's'} from $_label',
                key: const Key('location-track-count'),
                style: const TextStyle(fontWeight: FontWeight.bold),
              ),
              if (_url != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: OutlinedButton.icon(
                    key: const Key('location-map-link'),
                    onPressed: () => unawaited(_openMap()),
                    icon: const Icon(Icons.map_outlined),
                    label: const Text('Open map'),
                  ),
                ),
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    _error!,
                    key: const Key('location-track-error'),
                    style: TextStyle(color: cs.error),
                  ),
                ),
              const SizedBox(height: 8),
              Expanded(
                child: ListView.builder(
                  itemCount: _points.length,
                  itemBuilder: (context, i) {
                    final point = _points[i];
                    return ListTile(
                      dense: true,
                      title: Text(
                        '${point.latitude.toStringAsFixed(5)}, '
                        '${point.longitude.toStringAsFixed(5)}',
                      ),
                      subtitle: Text(
                        '±${point.accuracy.toStringAsFixed(0)} m · '
                        '#${point.sequence}${point.isFinal ? ' · final' : ''}',
                      ),
                    );
                  },
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
