// Timestamp / date formatting helpers shared by the UI screens.

/// Format a local [dt] as `YYYY-MM-DD HH:MM:SS`; `Never` when null.
String formatTimestamp(DateTime? dt) {
  if (dt == null) return 'Never';
  final d = dt.toLocal();
  String pad2(int n) => n.toString().padLeft(2, '0');
  return '${d.year}-${pad2(d.month)}-${pad2(d.day)} ${pad2(d.hour)}:${pad2(d.minute)}:${pad2(d.second)}';
}

/// Format a "YYYY-MM-DDTHH:MM:SS..." last/first-seen string from Rust into a
/// display form: truncate to the second, T→space, drop a trailing Z.
String formatSeenTimestamp(String value) {
  if (value.isEmpty) return 'unknown';
  final trimmed = value.length >= 19 ? value.substring(0, 19) : value;
  return trimmed.replaceAll('T', ' ').replaceAll('Z', '').trim();
}
