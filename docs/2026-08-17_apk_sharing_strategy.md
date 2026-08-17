# APK Sharing Strategy

## Requirement

The Android app must be able to share its own APK with other Android phones via whatever means possible: WiFi, Bluetooth, Android's share feature (which also handles generic file sharing).

This is critical for a P2P app: users should be able to onboard new users without requiring the Play Store or internet access.

---

## How Android APK Sharing Works

An APK is just a file. Android can share any file via:
1. **Share Intent** (`ACTION_SEND`) - opens Android's share sheet
2. **Bluetooth** - direct device-to-device transfer
3. **WiFi Direct** - high-speed local transfer
4. **Nearby Share** (Google's API) - uses Bluetooth + WiFi
5. **NFC** - tap to transfer (limited size, impractical for APKs)

The APK file is located at `context.packageCodePath` or can be obtained from `ApplicationInfo.sourceDir`.

---

## Implementation Strategy

### 1. Share Intent (Simplest, Covers Most Cases)

Android's built-in share sheet lets users share the APK via:
- Bluetooth
- WiFi Direct (via nearby devices)
- Email/messaging apps
- Any installed app that handles `ACTION_SEND`

```kotlin
// ShareManager.kt
class ShareManager(private val context: Context) {

    fun shareApk(activity: Activity) {
        val apkFile = File(context.packageCodePath)
        val uri = FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            apkFile
        )

        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "application/vnd.android.package-archive"
            putExtra(Intent.EXTRA_STREAM, uri)
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        activity.startActivity(Intent.createChooser(intent, "Share P2P Chat"))
    }
}
```

**Requirements in AndroidManifest.xml:**
```xml
<provider
    android:name="androidx.core.content.FileProvider"
    android:authorities="${applicationId}.fileprovider"
    android:exported="false"
    android:grantUriPermissions="true">
    <meta-data
        android:name="android.support.FILE_PROVIDER_PATHS"
        android:resource="@xml/file_paths" />
</provider>
```

**FileProvider paths (res/xml/file_paths.xml):**
```xml
<paths>
    <root-path name="root" path="/" />
</paths>
```

### 2. Bluetooth Transfer

Two options:

**Option A: Via Share Intent (recommended)**
The Share Intent already supports Bluetooth if the device has Bluetooth. No extra code needed.

**Option B: Direct Bluetooth (for automation)**
Use Android's Bluetooth APIs to programmatically send the APK to a paired device:
- `BluetoothSocket` for RFCOMM
- Requires pairing beforehand
- More complex, but allows app-to-app automation

For a P2P chat app, Option A is sufficient. Users tap "Share", pick Bluetooth, pick device.

### 3. WiFi Direct

WiFi Direct is exposed through Android's Share Sheet on most devices (it appears as "Nearby Share" or "Quick Share"). No extra code needed if using Share Intent.

For programmatic WiFi Direct (e.g., auto-discover and send):
- Use `WifiP2pManager` API
- More complex, requires both devices to opt in
- Not recommended for initial implementation

### 4. In-App Receiving

The receiving phone needs to accept and install the APK. Options:

**Option A: User handles it**
- Sender shares APK via any method
- Receiver downloads/opens the APK file
- Android prompts to install
- Simplest, works today

**Option B: In-app receiver (advanced)**
- App runs a small HTTP server or Bluetooth listener
- Receives the APK and prompts installation
- More seamless but complex

**Recommendation:** Start with Option A. The Android package installer handles APK installation well.

---

## Complete Sharing Flow

```
User A (has app)                    User B (needs app)
      │                                    │
      ├── Open app                         │
      ├── Tap "Share App"                  │
      ├── Android share sheet opens        │
      ├── Pick method (BT/WiFi/etc)  ────> │
      │                                    ├── Receive APK file
      │                                    ├── Tap to open
      │                                    ├── Android: "Install from unknown source?"
      │                                    ├── Allow, install
      │                                    ├── Open app
      │                                    ├── mDNS discovers User A
      │   <──── P2P connection ────────────┤
      │                                    │
```

---

## Security Considerations

### For Sender
- The APK is signed with the developer's key
- Users should verify they're sending the correct app
- No sensitive data in the APK itself

### For Receiver
- Android warns about installing from unknown sources
- APK signature verification happens automatically
- Users should only install APKs they trust

### Recommended: In-App Version Check
```kotlin
// When receiving a shared APK, verify before installing
fun verifyAndInstall(apkFile: File) {
    val packageInfo = packageManager.getPackageArchiveInfo(
        apkFile.absolutePath, PackageManager.GET_SIGNING_CERTIFICATES
    )
    // Compare signing certificate with known-good certificate
    if (isTrustedSignature(packageInfo)) {
        promptInstall(apkFile)
    } else {
        showWarning("APK signature doesn't match expected app")
    }
}
```

---

## Implementation Checklist

### Phase 1: Basic Share Intent
- [ ] Add FileProvider to AndroidManifest.xml
- [ ] Create `file_paths.xml` resource
- [ ] Write `ShareManager.shareApk()` method
- [ ] Add "Share App" button to UI
- [ ] Test: share via Bluetooth, verify APK installs

### Phase 2: In-App APK Receiver
- [ ] Add intent filter for `application/vnd.android.package-archive`
- [ ] Handle received APK in `onCreate`/`onNewIntent`
- [ ] Verify APK signature before prompting install
- [ ] Test: receive APK from another phone, install

### Phase 3: P2P APK Distribution (Advanced)
- [ ] Add APK metadata to peer discovery (version, size, signature)
- [ ] Use libp2p request-response to transfer APK between peers
- [ ] Auto-update check: "Newer version available from peer X"
- [ ] This is optional - Share Intent covers the core requirement

---

## Edge Cases

| Case | Handling |
|------|----------|
| App not installed yet (fresh phone) | Share APK via BT/USB/cloud manually |
| Different architectures (arm64 vs arm) | Build universal APK or split APKs by ABI |
| Android version differences | `FileProvider` works on API 14+, share intent is universal |
| APK too large for Bluetooth | Use WiFi Direct via share sheet, or split APK |
| User declines unknown source install | Show instructions to enable in settings |
| App is running in background | Share Intent works even when app is backgrounded |

---

## Build Considerations for Universal APK

To share between different Android architectures:

**Option A: Universal APK**
```gradle
android {
    splits {
        abi {
            isEnable = false  // Single APK with all ABIs
        }
    }
}
```
Larger APK (~15-20 MB with Rust .so for all ABIs) but works everywhere.

**Option B: Split APKs (AAB)**
Google Play handles this automatically, but for direct sharing, a single universal APK is simpler.

**Recommendation:** Single universal APK for the sharing use case. Size is acceptable for a P2P app.

---

## Files to Create

```
android/app/src/main/
├── java/com/p2pchat/app/
│   └── share/
│       └── ShareManager.kt          # APK sharing logic
├── AndroidManifest.xml              # FileProvider + intent filters
└── res/xml/
    └── file_paths.xml               # FileProvider paths
```

This is a small, self-contained feature. ~100 lines of Kotlin total.
