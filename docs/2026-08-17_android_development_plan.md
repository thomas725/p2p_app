# Cross-Platform Frontend Strategy

## Current State

- **No Android code exists** in the repository
- Three existing frontends: CLI, TUI (ratatui), Dioxus desktop (WebView)
- Rust core (libp2p, SQLite, business logic) is well-structured for reuse
- `sqlite_bundled` feature and size-optimized release profile already configured
- Cross-compilation for ARM Linux exists

## The Core Problem

The Rust networking layer must run on all targets. The UI layer needs to cover **desktop + Android** (and ideally iOS/web later) with minimal separate codebases. The Android app must also be shareable (APK sharing).

---

## Approach Options

### Option A: Dioxus Mobile (WebView)

Dioxus supports Android via WebView. The existing `dioxus_app.rs` could be adapted.

**Platforms:** Web, Desktop, Mobile (all WebView-based)

**Pros:**
- Reuses existing Dioxus code (CSS styling, component structure)
- Single codebase for desktop + mobile
- Familiar HTML/CSS mental model
- Already partially implemented in this repo

**Cons:**
- WebView adds ~15-30 MB to APK size
- WebView performance is suboptimal for real-time chat
- Android WebView quirks (fragmentation across devices)
- libp2p background service is tricky with WebView lifecycle
- No native Android feel
- Dioxus mobile is less mature than desktop

**Verdict:** Keep desktop Dioxus as-is. Not recommended as primary mobile target.

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

**Cons:**
- JNI bridge code to maintain
- Kotlin UI code is a separate codebase from desktop
- Two languages, two build systems
- Desktop frontends remain separate (TUI, Dioxus)

**Verdict:** Best for Android-only. Doesn't help reduce desktop frontend count.

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

Mozilla's UniFFI generates Kotlin/Swift bindings from Rust interface definitions.

**Platforms:** Android, iOS (binding generation only, not a UI framework)

**Pros:**
- Type-safe bindings auto-generated from Rust
- Used by Mozilla (Firefox) in production
- Clean interface definitions

**Cons:**
- Not a UI framework — just a binding generator
- Still need native UI code on each platform
- Adds build complexity (UDL files, proc macros)
- JNI bridge still exists (just auto-generated)

**Verdict:** Useful as a complement to other options (e.g., Option B or G), not a standalone solution.

---

### Option E: Flutter + flutter_rust_bridge (NEW)

Flutter is Google's cross-platform UI toolkit. `flutter_rust_bridge` (FRB) generates type-safe Dart bindings from Rust code automatically.

**Platforms:** Android, iOS, Windows, macOS, Linux, Web — **single codebase**

**Pros:**
- **One UI codebase for all platforms** (Android, iOS, Desktop, Web)
- Flutter is the most popular cross-platform mobile SDK (StackOverflow surveys)
- `flutter_rust_bridge` v2 is mature (Flutter Favorite, 5K+ stars, active CI with sanitizers)
- Auto-generates Dart bindings from Rust `api.rs` — no manual JNI/FFI
- Supports async Rust, arbitrary types, two-way calls (Rust ↔ Dart)
- Hot reload for rapid development
- Native Android/iOS feel (Material Design / Cupertino widgets)
- Smaller APK than WebView solutions (~15-25 MB with Rust .so)
- Large ecosystem of packages and plugins
- FRB supports all 6 platforms (Android, iOS, Windows, macOS, Linux, Web)

**Cons:**
- **Dart is a new language** for this project (learning curve)
- Flutter rendering is Skia/Impeller-based (not truly native widgets, but looks native)
- APK sharing still needs Android platform APIs (accessible via Flutter plugins)
- FRB code generation adds a build step
- Flutter desktop is less mature than mobile (but usable)
- Two languages (Rust + Dart) in the project
- Background networking (libp2p) still needs platform-specific service management
- Flutter Web uses WASM, which adds complexity

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

**Verdict:** Strong contender for unified desktop + mobile. Best code sharing ratio.

---

### Option F: Tauri 2 Mobile (NEW)

Tauri 2 added iOS and Android support in late 2024. Uses system WebView on mobile, Rust for backend.

**Platforms:** Windows, macOS, Linux, Android, iOS — **single codebase**

**Pros:**
- Single codebase (Rust + web frontend)
- Tiny bundles (~3 MB desktop, mobile comparable to Dioxus WebView)
- Mature desktop support (used in production by 1Password, Spacedrive, AppFlowy)
- Built-in capability-based security model
- Rust backend runs natively — perfect for our libp2p core
- Active development (v2.11, April 2026)
- Android build variants now supported

**Cons:**
- **Mobile is less mature than desktop** — plugin support lags, rough edges
- WebView-based on mobile (same concerns as Dioxus for native feel)
- Requires Node.js/npm toolchain for the web frontend
- iOS has Xcode 26 compatibility issues (Swift linking bugs, open as of March 2026)
- Community reports mobile plugin gaps vs desktop
- "If your product is mobile first, I'd still lean toward React Native or Flutter" — common advice
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

## Comparison Matrix

| Criterion | A: Dioxus Mobile | B: Native JNI | C: Cargo-Android | D: UniFFI | E: Flutter+FRB | F: Tauri 2 | G: Crux |
|-----------|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| **Single UI codebase (desktop+mobile)** | Yes | No | Partial | No | **Yes** | **Yes** | No |
| **Android support** | Yes | **Yes** | Experimental | Yes | **Yes** | Yes | Yes |
| **Desktop support** | **Yes** | No | No | No | **Yes** | **Yes** | Yes (native) |
| **Native feel on Android** | No | **Yes** | N/A | **Yes** | Mostly | No | **Yes** |
| **APK size** | Large | **Smallest** | Small | Small | Medium | Large | Small |
| **Maturity** | Medium | **High** | Low | **High** | **High** | Medium | Medium |
| **Rust integration ease** | N/A | Manual JNI | Manual | Auto-gen | **Auto-gen** | Built-in | Auto-gen |
| **Background service** | Hard | **Easy** | Hard | **Easy** | Medium | Hard | **Easy** |
| **Community/ecosystem** | Medium | **High** | Low | Medium | **Very High** | High | Medium |
| **Learning curve (new tech)** | Low | Medium | Low | Medium | **Dart** | **JS/CSS** | High |
| **APK sharing possible** | Yes | **Yes** | No | **Yes** | **Yes** | Yes | **Yes** |

---

## Recommendation

### If you want ONE codebase for desktop + mobile:

**Option E: Flutter + flutter_rust_bridge** is the strongest choice.

- Single Dart/Flutter UI runs on Android, iOS, Windows, macOS, Linux, Web
- FRB auto-generates bindings — no manual JNI boilerplate
- Native-feeling UI (Material Design on Android, Cupertino on iOS)
- Mature ecosystem, large community, Flutter Favorite status for FRB
- APK sharing via Flutter's `share_plus` or `url_launcher` plugins (wraps Android Share Intent)
- Background service via `flutter_background_service` plugin

The tradeoff is learning Dart. But Dart is easy to pick up (similar to Java/Kotlin/TypeScript), and the payoff is one UI codebase across all platforms.

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
3. **Add Flutter + FRB** for Android (and potentially iOS/web later)

This gives:
- **3 platforms covered** with 3 small, focused frontends
- The Rust core remains the single source of truth
- No need to rewrite working TUI/Dioxus code
- Android gets the best mobile experience via Flutter
- Future iOS/web can reuse the same Flutter codebase

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
└── flutter_app/            # NEW: Flutter mobile/desktop app
    ├── lib/
    │   ├── main.dart
    │   ├── screens/
    │   └── src/rust/        # FRB-generated bindings
    ├── rust/                # FRB api.rs entry point
    └── pubspec.yaml
```

---

## Migration Path (Flutter + FRB)

1. **Phase 1:** Install Flutter SDK, create `flutter_app/` project
2. **Phase 2:** Add `flutter_rust_bridge`, create `rust/api.rs` with core functions
3. **Phase 3:** Cross-compile Rust core for Android (FRB handles this via cargo-ndk)
4. **Phase 4:** Build chat screen (broadcast messages, peer list)
5. **Phase 5:** Add DM screen, nickname editing
6. **Phase 6:** Implement foreground service for background networking
7. **Phase 7:** Add APK sharing (Flutter plugin wrapping Share Intent)
8. **Phase 8:** Test on real devices, polish UI

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

---

## Risks and Unknowns

| Risk | Mitigation |
|------|-----------|
| libp2p mDNS broken on Android | Use Android NsdManager as fallback |
| Android kills networking service | Foreground service + battery exemption prompt |
| Flutter + FRB code generation issues | FRB has solid CI (valgrind, sanitizers), large user base |
| Rust binary size too large | `sqlite_bundled` + `opt-level='z'` + LTO already configured |
| Cross-compilation issues | FRB handles cargo-ndk integration, well-tested path |
| Dart learning curve | Dart is simple (Java/TypeScript-like), 1-2 weeks to productive |
| Tauri iOS Xcode 26 bug | Affects Option F only; not recommended as primary mobile target |
| Crux pre-1.0 API changes | Affects Option G only; Proton manages this at scale |
