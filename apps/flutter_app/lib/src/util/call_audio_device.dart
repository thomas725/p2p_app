// Selects the platform device adapter without pulling browser JS into native
// builds — the same conditional-import pattern the notification adapters use.

export 'call_audio.dart';

import 'call_audio.dart';
import 'call_audio_platform.dart'
    if (dart.library.js_interop) 'call_audio_platform_web.dart';

/// A fresh device adapter for one live call.
CallAudioAdapter createCallAudioAdapter() => createPlatformCallAudioAdapter();
