# p2p_app Flutter Frontend

Shared Flutter UI for Android first, with desktop/iOS/Windows as later targets.

This app is intentionally thin:
- Flutter owns screens, navigation, and visual state.
- Native Android owns foreground service lifecycle, permissions, notifications, multicast lock, storage paths, and share intents.
- Rust owns libp2p, SQLite, identity, messages, and peer state.

## First Milestone

Target vertical slice:
1. Flutter starts and asks Android for mobile status.
2. Android supplies an app-specific SQLite path.
3. Rust initializes the database and identity through the mobile facade.
4. Flutter displays the local peer ID and service status.
5. Flutter can request start/stop of the Android foreground service.

## Current State

Flutter is not installed in the current environment, so this directory is a checked-in scaffold, not yet a generated Flutter project.

Once Flutter is installed, run:

```bash
cd apps/flutter_app
flutter create --project-name p2p_app_flutter --platforms android,linux,macos,windows,ios .
flutter pub get
```

Keep the `lib/` API shape intact unless the native host contract changes.

## Host Channel Contract

Flutter talks to the native host on:

```text
app.p2p/host
```

Methods:
- `getStatus` -> `{ databaseUrl, localPeerId, selfNickname, serviceRunning }`
- `startService` -> `{ serviceRunning }`
- `stopService` -> `{ serviceRunning }`

Android will implement this channel in Kotlin. Desktop can later implement an equivalent host directly in Flutter/FRB or through platform-specific plugins.
