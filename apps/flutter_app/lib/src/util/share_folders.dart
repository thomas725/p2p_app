// Native-only directory picker for file shares.
//
// Rust owns every policy decision here: `bindShare` canonicalises the root and
// `snapshotShare` walks it into content-addressed blobs with a revision bump.
// Dart only asks the operating system which directory the user chose, so the
// plugin call sits behind an injectable seam for FFI-free widget tests.
library;

import 'package:file_picker/file_picker.dart';

/// Returns the chosen directory, or `null` when the user cancels.
typedef PickDirectoryOverride = Future<String?> Function();

PickDirectoryOverride pickDirectoryOverride = _pickViaPlugin;

Future<String?> _pickViaPlugin() => FilePicker.getDirectoryPath();
