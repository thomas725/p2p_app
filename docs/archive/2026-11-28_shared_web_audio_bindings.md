> **ARCHIVED** — historical record, not current behaviour.

# Shared Web Audio bindings for the web adapters (2026-11-28)

The notification beep and the call-media device adapter are both web-only and
both declared their own `@JS('AudioContext')` extension type plus an identical
`isSuspended` check. Two copies of the same browser binding meant a fix to
context detection (or a new node factory) had to be made twice.

## What shipped

- `apps/flutter_app/lib/src/util/web_audio.dart` owns the one `WebAudioContext`
  binding (the union of the members both adapters used), `WebAudioBuffer`, and
  the shared `isSuspended` extension. It is imported only by the two web
  adapters, so native builds still never compile `dart:js_interop` for audio.
- `notifications_platform_web.dart` keeps its beep-only `_AudioNodeOps`
  extension; `call_audio_platform_web.dart` keeps its `getUserMedia` binding
  and capture/playback logic. Neither copies the context type any more.
- The `dart format` rewrap exposed an old unbraced `if` in the call adapter;
  braces were added so `curly_braces_in_flow_control_structures` stays clean.

No runtime behaviour changed. `flutter analyze`, the full 292-test suite and a
real `flutter build web` (which compiles the extracted bindings) all pass.
