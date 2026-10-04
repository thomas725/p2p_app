// File-attachment helpers for the chat rows (files lane §12).
//
// The Rust side owns the bytes (content-addressed store under `files_dir`); a
// message only carries a `MobileAttachment` descriptor. Sending means picking a
// local path for `saveOutgoing*` to ingest; opening a received file resolves its
// blob path via `attachmentFilePath` and hands it to the system opener. Both
// plugin calls sit behind injectable seams so widget tests never touch the
// platform channel.
library;

import 'package:file_picker/file_picker.dart';
import 'package:open_filex/open_filex.dart';

import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// Injectable seam over the platform file picker. Returns the picked path, or
/// `null` when the user cancels. Widget tests replace this to avoid the plugin.
typedef PickFileOverride = Future<String?> Function();

PickFileOverride pickFileOverride = _pickViaPlugin;

Future<String?> _pickViaPlugin() async {
  final file = await FilePicker.pickFile();
  return file?.path;
}

/// Resolve and open a received attachment's stored blob. Returns an error
/// message on failure (or when the blob hasn't landed yet), `null` on success.
Future<String?> openAttachedFile(ChatMessage msg) async {
  final attachment = msg.attachment;
  if (attachment == null) return null;
  try {
    final path = await attachmentFilePath(fileId: attachment.fileId);
    return await openFileOverride(path);
  } catch (e) {
    return '$e';
  }
}

/// Injectable seam over the system file opener. Returns an error message on
/// failure, `null` on success.
typedef OpenFileOverride = Future<String?> Function(String path);

OpenFileOverride openFileOverride = _openViaPlugin;

Future<String?> _openViaPlugin(String path) async {
  final result = await OpenFilex.open(path);
  return result.type == ResultType.done ? null : result.message;
}