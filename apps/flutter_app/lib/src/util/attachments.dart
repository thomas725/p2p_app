// File-attachment helpers for the chat rows (files lane §12).
//
// The Rust side owns the bytes (content-addressed store under `files_dir`); a
// message only carries a `MobileAttachment` descriptor. Sending means either
// handing `saveOutgoing*` a local path (mobile/native) or uploading raw bytes
// over the web transport's HTTP route; opening a received file resolves its
// location via `NodeApi.resolveAttachmentLocation` — a filesystem path for the
// in-process transport (system opener), a download URL for the remote one
// (browser). Both plugin calls sit behind injectable seams so widget tests
// never touch a platform channel.
library;

import 'dart:typed_data';

import 'package:file_picker/file_picker.dart';
import 'package:open_filex/open_filex.dart';
import 'package:url_launcher/url_launcher.dart';

import 'package:p2p_app_flutter/src/data/node_api.dart';
import 'package:p2p_app_flutter/src/rust/mobile_node.dart';

/// A picked outgoing file: a filesystem [path] on native targets, or [bytes]
/// on web (where the picker yields an in-memory blob with no local path).
class PickedAttachment {
  const PickedAttachment({this.path, this.bytes, required this.name})
    : assert(path != null || bytes != null, 'path or bytes required');

  final String? path;
  final Uint8List? bytes;
  final String name;
}

/// Injectable seam over the platform file picker. Returns the picked file,
/// or `null` when the user cancels. Widget tests replace this to avoid the
/// plugin.
typedef PickFileOverride = Future<PickedAttachment?> Function();

PickFileOverride pickFileOverride = _pickViaPlugin;

Future<PickedAttachment?> _pickViaPlugin() async {
  final file = await FilePicker.pickFile();
  if (file == null) return null;
  final path = file.path;
  if (path != null) return PickedAttachment(path: path, name: file.name);
  // Web: the picker result has no local filesystem entry — carry the bytes
  // so the remote transport can upload them.
  return PickedAttachment(bytes: await file.readAsBytes(), name: file.name);
}

/// Resolve and open a received attachment's stored blob. Returns an error
/// message on failure (or when the blob hasn't landed yet), `null` on success.
Future<String?> openAttachedFile(ChatMessage msg) async {
  final attachment = msg.attachment;
  if (attachment == null) return null;
  try {
    final location = await nodeApi.resolveAttachmentLocation(attachment.fileId);
    return nodeApi.isRemoteTransport
        ? await openRemoteUrlOverride(location)
        : await openFileOverride(location);
  } catch (e) {
    return '$e';
  }
}

/// Injectable seam over the system file opener (native). Returns an error
/// message on failure, `null` on success.
typedef OpenFileOverride = Future<String?> Function(String path);

OpenFileOverride openFileOverride = _openViaPlugin;

Future<String?> _openViaPlugin(String path) async {
  final result = await OpenFilex.open(path);
  return result.type == ResultType.done ? null : result.message;
}

/// Injectable seam over the browser download (remote transport). The node
/// serves the blob with an attachment disposition, so `launchUrl` downloads
/// or displays it in a new context. Returns an error message on failure,
/// `null` on success.
typedef OpenRemoteUrlOverride = Future<String?> Function(String url);

OpenRemoteUrlOverride openRemoteUrlOverride = _openViaUrlLauncher;

Future<String?> _openViaUrlLauncher(String url) async {
  final uri = Uri.parse(url);
  try {
    final opened = await launchUrl(uri, mode: LaunchMode.platformDefault);
    return opened ? null : 'the browser refused to open $url';
  } catch (e) {
    return '$e';
  }
}
