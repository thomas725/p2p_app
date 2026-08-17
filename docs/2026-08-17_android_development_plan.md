# Cross-Platform Frontend Strategy

## Current State

- **No Android code exists** in the repository
- Three existing frontends: CLI, TUI (ratatui), Dioxus desktop (WebView)
- Rust core (libp2p, SQLite, business logic) is well-structured for reuse
- `sqlite_bundled` feature and size-optimized release profile already configured
- Cross-compilation for ARM Linux exists

## The Core Problem

The Rust networking layer must run on all targets. The UI layer needs to cover **desktop + Android** (and ideally iOS/web later) with minimal separate codebases. The Android app must also be shareable (APK sharing).

The key tradeoff is not just "one frontend or many." For this app, the hard Android work is mostly outside the screen rendering layer:
- long-running libp2p lifecycle
- foreground service requirements
- mDNS / multicast lock behavior
- Android storage paths for SQLite
- share intents for APK sharing
- runtime permissions for networking and nearby-device features

A single cross-platform UI can reduce screen code, but it does not remove most of those platform-specific obligations.

---

## Approach Options

### Option A: Dioxus Mobile (WebView)

Dioxus supports Android/iOS using the same Wry/Tao/WebView renderer family as desktop. The existing `dioxus_app.rs` could be adapted, and current Dioxus tooling has a mobile path through `dx`.

**Platforms:** Web, Desktop, Mobile (all WebView-based)

**Pros:**
- Reuses existing Dioxus code (CSS styling, component structure)
- Single codebase for desktop + mobile
- Familiar HTML/CSS mental model
- Already partially implemented in this repo
- Lowest UI rewrite if the current Dioxus desktop app becomes the main GUI

**Cons:**
- WebView overhead and Android WebView behavior become part of the app's support matrix
- Real-time chat rendering is probably fine; the bigger issue is lifecycle, permissions, and native integration
- Android WebView quirks (fragmentation across devices)
- libp2p background service is tricky with WebView lifecycle
- No native Android feel
- Dioxus mobile is still less mature than desktop and less proven than native Android/Flutter

**Verdict:** Good prototype path if we want to reuse the existing GUI quickly. Not my first choice for an Android app whose hard requirements are background networking and platform integration.

---

### Option B: Native Android UI + JNI/Rust Core

Write a native Android UI (Kotlin/Jetpack Compose) and call into Rust via JNI.

**Platforms:** Android only (desktop would remain separate)

**Pros:**
- Native Android performance and feel
- Full access to Android APIs (share intents, Bluetooth, notifications)
- Smallest APK (Rust .so is ~3-5 MB, Android shell is small)
- Best battery life (no WebView overhead)
- Can run Rust networking as a foreground service
- Easiest place to handle Android-only problems: multicast lock, notification channels, battery optimization prompts, storage directories

**Cons:**
- JNI bridge code to maintain
- Kotlin UI code is a separate codebase from desktop
- Two languages, two build systems
- Desktop frontends remain separate (TUI, Dioxus)
- Raw JNI is easy to get wrong unless the exported Rust API is deliberately small

**Verdict:** Best Android shell. Prefer pairing it with UniFFI rather than maintaining raw JNI by hand.

---

### Option C: Cargo-Android (Experimental)

Use `cargo-apk` or `ndk-glue` to build a native Rust Android app without Kotlin.

**Platforms:** Android (experimental)

**Pros:**
- Pure Rust, no JNI bridge

**Cons:**
- Very experimental ecosystem
- No access to Android Java APIs (share intents, Bluetooth, etc.)
- Can't use Jetpack Compose or standard Android UI toolkit
- Poor documentation and community support
- APK sharing requires Android APIs

**Verdict:** Not recommended. Too immature, blocks APK sharing feature.

---

### Option D: UniFFI (Mozilla)

Mozilla's UniFFI generates bindings from Rust interface definitions. Kotlin and Swift are the mature mobile targets; React Native tooling also exists through `uniffi-bindgen-react-native`.

**Platforms:** Android, iOS (binding generation only, not a UI framework)

**Pros:**
- Type-safe bindings auto-generated from Rust
- Used by Mozilla (Firefox) in production
- Clean interface definitions
- Lets the Rust boundary stay intentionally small: commands in, events/status snapshots out
- Better fit than raw JNI for this repo because the current Rust core already exposes `SwarmCommand`, `SwarmEvent`, `PeerRecord`, and message types

**Cons:**
- Not a UI framework — just a binding generator
- Still need native UI code on each platform
- Adds build complexity (UDL files, proc macros)
- JNI bridge still exists (just auto-generated)
- Long-running Rust tasks still need explicit lifecycle ownership from the Android shell

**Verdict:** Not a standalone app strategy, but likely the best bridge for a native Android shell.

---

### Option E: Flutter + flutter_rust_bridge (NEW)

Flutter is Google's cross-platform UI toolkit. `flutter_rust_bridge` (FRB) generates type-safe Dart bindings from Rust code automatically.

**Platforms:** Android, iOS, Windows, macOS, Linux, Web — **single codebase**

**Pros:**
- **One UI codebase for all platforms** (Android, iOS, Desktop, Web)
- Flutter is a mature, widely used cross-platform mobile SDK
- `flutter_rust_bridge` v2 is a mature Rust/Dart binding generator
- Auto-generates Dart bindings from Rust `api.rs` — no manual JNI/FFI
- Supports async Rust, arbitrary types, two-way calls (Rust ↔ Dart)
- Hot reload for rapid development
- Native Android/iOS feel (Material Design / Cupertino widgets)
- Smaller APK than WebView solutions (~15-25 MB with Rust .so)
- Large ecosystem of packages and plugins
- FRB supports all 6 platforms (Android, iOS, Windows, macOS, Linux, Web)
- Strong option if future iOS is a near-term requirement and native Android look/feel is less important than one UI team/codebase

**Cons:**
- **Dart is a new language** for this project (learning curve)
- Flutter rendering is Skia/Impeller-based (not truly native widgets, but looks native)
- APK sharing still needs Android platform APIs (accessible via Flutter plugins)
- FRB code generation adds a build step
- Flutter desktop is less mature than mobile (but usable)
- Two languages (Rust + Dart) in the project
- Background networking (libp2p) still needs platform-specific service management
- Flutter Web uses WASM, which adds complexity
- The hardest Android pieces still require Android-specific plugin or platform-channel work
- A Flutter desktop rewrite would duplicate the existing Dioxus desktop before it replaces it

**Architecture:**
```
Flutter App (Dart - single codebase)
  └── flutter_rust_bridge
       └── Rust Core (lib.rs → .so/.dylib/.dll)
            ├── libp2p networking
            ├── SQLite (diesel, sqlite_bundled)
            └── All business logic
```

**FRB Integration:**
```rust
// src/api.rs — Rust functions exposed to Dart
#[frb]
pub fn start_p2p(db_path: String) { ... }

#[frb]
pub fn send_broadcast(content: String) { ... }

#[frb]
pub fn send_dm(peer_id: String, content: String) { ... }

#[frb]
pub fn get_peers() -> Vec<PeerRecord> { ... }
```

FRB auto-generates:
- Dart types from Rust structs/enums
- Async Dart functions from Rust async functions
- Stream handlers for Rust → Dart callbacks (for events)

**Verdict:** Strong contender for unified desktop + mobile. Best code sharing ratio, but not necessarily the lowest-risk Android path for a P2P app.

---

### Option F: Tauri 2 Mobile (NEW)

Tauri 2 supports iOS and Android. It uses a web frontend with native Rust backend commands; mobile plugins can include Android Kotlin code.

**Platforms:** Windows, macOS, Linux, Android, iOS — **single codebase**

**Pros:**
- Single codebase (Rust + web frontend)
- Tiny bundles (~3 MB desktop, mobile comparable to Dioxus WebView)
- Mature desktop support (used in production by 1Password, Spacedrive, AppFlowy)
- Built-in capability-based security model
- Rust backend runs natively — perfect for our libp2p core
- Active development and documented Android/iOS setup
- Android plugin model can reach Kotlin APIs when needed

**Cons:**
- **Mobile is less mature than desktop** — plugin support lags, rough edges
- WebView-based on mobile (same concerns as Dioxus for native feel)
- Requires Node.js/npm toolchain for the web frontend
- Mobile-specific functionality may require writing custom plugins anyway
- Background service story on Android similar to WebView options

**Verdict:** Good if desktop is primary and mobile is secondary. Not ideal for mobile-first.

---

### Option G: Crux (Shared Rust Core + Native Shells) (NEW)

Crux by Red Badger is a framework for sharing Rust business logic across platforms, with **native UI on each platform** (SwiftUI, Jetpack Compose, React, etc.).

**Platforms:** iOS, Android, Web, Desktop (via native shells)

**Pros:**
- **Production-proven**: Used by Proton (Mail, Calendar, Drive, VPN — millions of users) and Photoroom
- Strict separation: Rust core (business logic, side-effect-free) + thin native shell (UI)
- Core is fully testable without any platform
- Native UI on each platform (best possible feel)
- Event-sourced architecture (similar to Elm/Redux) — clean state management
- Type-safe FFI with code generation (Kotlin, Swift, TypeScript)
- Modular: teams can own independent Crux cores, compose them into apps
- Pre-1.0 but production-ready (used by Proton at scale)

**Cons:**
- **Still need native UI code on each platform** (Kotlin for Android, Swift for iOS, etc.)
- Pre-1.0 — API may have breaking changes
- Steeper learning curve (event sourcing, effects system, Commands)
- Doesn't reduce the number of frontend codebases — reduces *business logic* duplication
- More architectural overhead than FRB or direct JNI
- Dart/Flutter shell not yet supported (planned)
- Smaller community than Flutter

**Architecture:**
```
Rust Core (Crux App)
  ├── Model (state)
  ├── Event (actions)
  ├── Effect (side effects → Shell executes)
  └── ViewModel (prepared for UI)

Platform Shells:
  ├── Android: Kotlin + Jetpack Compose
  ├── iOS: Swift + SwiftUI
  ├── Desktop: TypeScript + React (or other)
  └── Web: TypeScript + React/Leptos
```

**Verdict:** Best for large teams/products needing maximum code reuse with native UI. Overkill for this project's current scale.

---

### Option H: Native Android + UniFFI + Thin Rust Service API (NEW, Favorite)

This is Option B with the bridge improved by Option D. Build a Kotlin/Jetpack Compose Android app, but expose the Rust core through a small UniFFI API instead of hand-written JNI. Keep existing CLI/TUI/Dioxus frontends rather than trying to replace them immediately.

**Platforms:** Android first; iOS later is possible with Swift bindings; desktop remains existing Rust frontends

**Pros:**
- Best Android integration for the hard parts: foreground service, notification channels, share intents, WiFi multicast lock, runtime permissions
- Type-safe generated bridge avoids most raw JNI maintenance
- Small UI surface can move fast in Compose without forcing a desktop rewrite
- Fits the current repo shape: Rust already owns networking, persistence, identity, and message types
- Keeps Android-specific service lifecycle in Android code, where it belongs
- Future iOS can reuse the same Rust boundary if the API is kept platform-neutral

**Cons:**
- Not one UI codebase
- Requires Kotlin/Gradle/Android project setup
- UniFFI adds code generation and binding packaging
- The Rust core probably needs a facade crate/module so the mobile API is smaller than the internal app API

**Architecture:**
```
Android App (Kotlin + Jetpack Compose)
  ├── ForegroundService owns process/lifecycle
  ├── Android APIs: share intent, permissions, multicast lock
  └── UniFFI-generated bindings
       └── Rust mobile facade
            ├── start_node(db_path, config) -> event stream
            ├── send_broadcast(content)
            ├── send_dm(peer_id, content)
            ├── list_messages() / list_peers()
            └── stop_node()
```

**Verdict:** My favorite path for this repo. It optimizes for the actual hard constraints of an Android P2P app instead of optimizing only for frontend code reuse.

---

## Comparison Matrix

| Criterion | A: Dioxus Mobile | B: Native JNI | C: Cargo-Android | D: UniFFI | E: Flutter+FRB | F: Tauri 2 | G: Crux | H: Native+UniFFI |
|-----------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| **Single UI codebase (desktop+mobile)** | Yes | No | Partial | No | **Yes** | **Yes** | No | No |
| **Android support** | Yes | **Yes** | Experimental | Yes | **Yes** | Yes | Yes | **Yes** |
| **Desktop support** | **Yes** | No | No | No | **Yes** | **Yes** | Yes (native) | Existing only |
| **Native feel on Android** | No | **Yes** | N/A | **Yes** | Mostly | No | **Yes** | **Yes** |
| **APK size** | Large | **Smallest** | Small | Small | Medium | Large | Small | **Small** |
| **Maturity** | Medium | **High** | Low | **High** | **High** | Medium | Medium | **High** |
| **Rust integration ease** | Existing UI | Manual JNI | Manual | Auto-gen | **Auto-gen** | Built-in | Auto-gen | **Auto-gen** |
| **Background service** | Hard | **Easy** | Hard | **Easy** | Medium | Hard | **Easy** | **Easy** |
| **Community/ecosystem** | Medium | **High** | Low | Medium | **Very High** | High | Medium | **High** |
| **Learning curve (new tech)** | Low | Medium | Low | Medium | Dart | JS/CSS | High | Kotlin + UniFFI |
| **APK sharing possible** | Yes | **Yes** | No | **Yes** | **Yes** | Yes | **Yes** | **Yes** |
| **Lowest-risk Android P2P lifecycle** | Low | High | Low | Medium | Medium | Low | High | **Highest** |

---

## Recommendation

### My favorite for this repo: Option H

**Native Android + UniFFI + a thin Rust service API** is the best default choice.

Why:
- The existing Rust core is already the valuable shared asset; rewriting desktop UI now is not necessary.
- Android-specific behavior is central to success: foreground service, multicast lock, share intents, runtime permissions, and battery restrictions.
- Compose gives native UI without WebView lifecycle ambiguity.
- UniFFI keeps the bridge maintainable and makes later Swift/iOS possible.
- The mobile Rust API can be deliberately small and testable: `start_node`, `stop_node`, `send_broadcast`, `send_dm`, `list_peers`, `list_messages`, and an event stream.

This does not maximize UI code sharing. It maximizes the chance of producing a reliable Android APK without destabilizing the working CLI/TUI/Dioxus frontends.

### If you want ONE codebase for desktop + mobile:

**Option E: Flutter + flutter_rust_bridge** is still the strongest choice.

- Single Dart/Flutter UI runs on Android, iOS, Windows, macOS, Linux, Web
- FRB auto-generates bindings — no manual JNI boilerplate
- Native-feeling UI (Material Design on Android, Cupertino on iOS)
- Mature ecosystem and large package/plugin community
- APK sharing via Flutter's `share_plus` or `url_launcher` plugins (wraps Android Share Intent)
- Background service via `flutter_background_service` plugin

The tradeoff is learning Dart and accepting that Android service/plugin work still exists. Pick this if the product direction is "one visual app across mobile and desktop," not just "ship Android well."

### If you want native Android feel + maximum control:

**Option B: Native Android + JNI** is the best Android-only choice.

- Best performance, smallest APK, full Android API access
- But you still need separate desktop frontends (TUI + Dioxus)

### If you want shared business logic but native UI everywhere:

**Option G: Crux** is the enterprise-grade choice.

- Proton and Photoroom prove it works at scale
- But you still write platform-specific UI (Kotlin, Swift, etc.)
- Overkill unless you plan to support iOS + Web + Desktop + Android all with native UI

---

## Hybrid Approach (Recommended)

The most pragmatic path uses **multiple options together**:

1. **Keep TUI** for terminal users (already works, small binary)
2. **Keep Dioxus desktop** for desktop users who want a GUI (already works)
3. **Add Native Android + UniFFI** for Android
4. Reconsider Flutter only if iOS/desktop UI unification becomes a real near-term requirement

This gives:
- **3 platforms covered** with 3 small, focused frontends
- The Rust core remains the single source of truth
- No need to rewrite working TUI/Dioxus code
- Android gets native lifecycle/service integration
- Future iOS can reuse the same UniFFI Rust API with a Swift shell

```
p2p_app
├── src/                    # Shared Rust core (lib.rs)
│   ├── libp2p networking
│   ├── SQLite persistence
│   └── Business logic
├── src/bin/
│   ├── p2p_chat.rs         # CLI (headless)
│   ├── p2p_chat_tui.rs     # TUI (ratatui)
│   └── p2p_chat_dioxus.rs  # Desktop GUI (Dioxus)
├── src/mobile_api.rs       # NEW: narrow UniFFI-facing facade
└── android_app/            # NEW: Kotlin + Jetpack Compose app
    ├── app/src/main/
    │   ├── AndroidManifest.xml
    │   ├── java/.../MainActivity.kt
    │   └── java/.../P2pForegroundService.kt
    └── generated/           # UniFFI-generated Kotlin bindings
```

---

## Migration Path (Native Android + UniFFI)

1. **Phase 1:** Add a small Rust `mobile_api` facade behind a feature flag.
2. **Phase 2:** Add UniFFI scaffolding and generate Kotlin bindings for simple sync calls (`get_local_peer_id`, nickname, DB init).
3. **Phase 3:** Expose node lifecycle (`start_node`, `stop_node`) and an event stream backed by the existing swarm handler.
4. **Phase 4:** Create a minimal Compose app: peer ID, peer list, broadcast message list, send box.
5. **Phase 5:** Move node ownership into an Android foreground service.
6. **Phase 6:** Add Android-specific networking support: permissions, WiFi multicast lock, optional `NsdManager` discovery fallback.
7. **Phase 7:** Add DM screen, nickname editing, message receipts.
8. **Phase 8:** Add APK sharing via Android share intent.
9. **Phase 9:** Test on at least two real devices on the same WiFi network.

Each phase produces a runnable (if incomplete) APK.

---

## P2P Networking on Android (Applies to All Options)

### Permissions Needed

```xml
<uses-permission android:name="android.permission.INTERNET" />
<uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />
<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
<uses-permission android:name="android.permission.ACCESS_FINE_LOCATION" />
<uses-permission android:name="android.permission.ACCESS_COARSE_LOCATION" />
<uses-permission android:name="android.permission.BLUETOOTH" />
<uses-permission android:name="android.permission.BLUETOOTH_ADMIN" />
<uses-permission android:name="android.permission.BLUETOOTH_SCAN" />
<uses-permission android:name="android.permission.BLUETOOTH_ADVERTISE" />
<uses-permission android:name="android.permission.BLUETOOTH_CONNECT" />
```

### mDNS on Android

libp2p's mDNS uses UDP multicast. On Android:
- Requires `ACCESS_FINE_LOCATION` permission
- May not work on all devices due to WiFi multicast lock
- Fallback: Android's `NsdManager` for mDNS, feed discovered peers into libp2p

### Background Networking

Android kills background services aggressively. Strategies:
1. **Foreground Service** with persistent notification (recommended)
2. **WorkManager** for periodic reconnection
3. **Battery optimization exemption** prompt

---

## APK Sharing Feature

See `2026-08-17_apk_sharing_strategy.md` for detailed strategy.

Works with all options that can access Android APIs:
- Option B (Native): Direct Android API access
- Option E (Flutter): Via `share_plus` or `url_launcher` plugins
- Option F (Tauri): Via Tauri's plugin system or shell commands
- Option G (Crux): Via platform shell (Kotlin)
- Option H (Native + UniFFI): Direct Android API access from the Kotlin shell

---

## Risks and Unknowns

| Risk | Mitigation |
|------|-----------|
| libp2p mDNS broken on Android | Use Android NsdManager as fallback |
| Android kills networking service | Foreground service + battery exemption prompt |
| Flutter + FRB code generation issues | Prototype a vertical slice before committing to Flutter |
| Rust binary size too large | `sqlite_bundled` + `opt-level='z'` + LTO already configured |
| Cross-compilation issues | Validate `cargo-ndk`/Gradle packaging before building full UI |
| Dart learning curve | Only accept it if Flutter becomes the strategic UI direction |
| Crux pre-1.0 API changes | Affects Option G only; Proton manages this at scale |
| Rust API too broad for mobile bindings | Add a small `mobile_api` facade instead of exposing internal modules directly |
| Android service outlives UI incorrectly | Android foreground service owns the Rust node; Compose observes state/events |

---

## Sources Checked

- Dioxus mobile docs: https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/
- `dioxus-mobile` crate docs: https://docs.rs/crate/dioxus-mobile/latest
- Tauri 2 mobile docs: https://v2.tauri.app/
- Tauri mobile plugin docs: https://v2.tauri.app/develop/plugins/develop-mobile/
- flutter_rust_bridge docs: https://cjycode.com/flutter_rust_bridge/
- UniFFI for React Native announcement: https://hacks.mozilla.org/2024/12/introducing-uniffi-for-react-native-rust-powered-turbo-modules/
- Crux docs: https://redbadger.github.io/crux/
