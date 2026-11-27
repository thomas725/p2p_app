// Selects the platform device adapter without pulling browser JS into native
// builds — the same conditional-import pattern the notification and call-audio
// adapters use. Web resolves to `location_device_web.dart`; every other target
// resolves to `location_device_io.dart`.

export 'location.dart';

import 'location.dart';
import 'location_device_io.dart'
    if (dart.library.js_interop) 'location_device_web.dart';

/// A fresh device-location adapter for this platform.
///
/// The UI calls this once per sharing session (and closes the result when the
/// session ends); [NoopLocationAdapter] is returned on targets without a real
/// sensor binding so callers can surface [locationDeviceUnavailableReason].
LocationAdapter createLocationAdapter() => createPlatformLocationAdapter();
