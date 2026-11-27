// GPX export seam for location tracks.
//
// Rust renders the canonical GPX document; Dart only turns the returned UTF-8
// text into bytes and hands it to the platform save dialog. Keeping the picker
// behind a seam lets widget tests exercise the export without a platform
// channel, and mirrors the `pickFileOverride` pattern in `attachments.dart`.

import 'dart:convert';
import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';

/// Result of a GPX save: the URI chosen by the user, or `null` on cancellation.
typedef SaveGpxOverride =
    Future<Uri?> Function({required String fileName, required Uint8List bytes});

/// Injectable platform seam. Replace in tests; production uses `file_picker`.
SaveGpxOverride saveGpxOverride = _saveGpxViaFilePicker;

Future<Uri?> _saveGpxViaFilePicker({
  required String fileName,
  required Uint8List bytes,
}) => FilePicker.saveFile(
  fileName: fileName,
  bytes: bytes,
  mimeType: 'application/gpx+xml',
);

/// Rust-owned GPX text → UTF-8 bytes through the save seam.
Future<Uri?> saveGpxText({
  required String fileName,
  required String gpx,
  SaveGpxOverride? override,
}) =>
    (override ?? saveGpxOverride)(fileName: fileName, bytes: utf8.encode(gpx));
