# CI Build Error Analysis - Deep Investigation

**Error**: `gdk-sys v0.18.2` build failure
**Status**: STILL FAILING despite our fix

---

## The Problem

### What Happened
The build script for `gdk-sys` (GDK 3.0 bindings) failed when trying to find the system library `gdk-3.0.pc`.

### Error Message
```
error: failed to run custom build command for `gdk-sys v0.18.2`

Caused by:
  process didn't exit successfully: `/home/runner/.../build-script-build` (exit code: 1)

The system library `gdk-3.0` required by crate `gdk-sys` was not found.
The file `gdk-3.0.pc` needs to be installed and the PKG_CONFIG_PATH 
environment variable must contain its parent directory.
The PKG_CONFIG_PATH environment variable is not set.
```

### Key Detail
The error says:
```
Package gdk-3.0 was not found in the pkg-config search path.
Perhaps you should add the directory containing `gdk-3.0.pc'
```

This is the CRITICAL CLUE: even though we installed `libgtk-3-dev`, the system can't find `gdk-3.0.pc`.

---

## Root Cause Analysis

###  The Dependency Chain

```
p2p_app
  └─> Something depends on GTK/GDK functionality
      └─> notify-rust (most likely culprit)
          └─> Requires GTK 3 for desktop notifications
              └─> Pulls in gtk-rs ecosystem
                  └─> gdk-sys crate (Rust bindings for GDK 3.0)
                      └─> Needs gdk-3.0.pc pkg-config file
```

### Why Our Fix Didn't Work

We added:
- ✅ libglib2.0-dev
- ✅ libgtk-3-dev  
- ✅ libgdk-pixbuf2.0-dev
- ✅ libcairo2-dev
- ✅ libpango1.0-dev

**But** `gdk-3.0.pc` is STILL not found. This means:

1. **The packages are installed** - they compiled successfully (glib-sys, gobject-sys, gio-sys all passed)
2. **But their pkg-config files are missing or in non-standard locations** - pkg-config can't find them
3. **GDK 3.0 might not be installed at all** - libgtk-3-dev may not include the GDK 3.0 bits in all Ubuntu versions

---

## The Real Issue: What We're Missing

###  Ubuntu 24.04.4 LTS Context
The CI is running on Ubuntu 24.04.4. This version may have:
- Different GTK 3 package structure
- Different pkg-config installation paths
- Missing or relocated GDK 3 development files

### Package Investigation

The command `pkg-config --list-all` shows what's available, but we can't run it in CI.

**Most likely missing packages:**
- `libgdk-3-dev` - Specifically the GDK 3.0 development headers (NOT the Pixbuf one)
- `libgdk-3-0` - GDK 3.0 library runtime (separate from libgtk-3-0)

---

## Why This Project Has GTK Dependency

### Tracing the Dependency

Looking at the compilation output:
- ✅ glib-sys v0.18.1 compiled successfully
- ✅ gobject-sys v0.18.0 compiled successfully  
- ✅ gio-sys v0.18.1 compiled successfully
- ❌ gdk-sys v0.18.2 **FAILED** - this is where it breaks

**gdk-sys is being pulled in by something**, but it's not directly in Cargo.toml.

### Hypothesis: notify-rust

The most common crate that pulls in GTK on Linux is `notify-rust` for desktop notifications. This is a common indirect dependency in GUI or TUI applications that want to show system notifications.

**We need to search for what's actually pulling in gdk-sys** - it's a transitive dependency somewhere.

---

## Solutions to Try (In Order)

### Option 1: Install GDK 3.0 Development Package Specifically
```bash
sudo apt-get install -y \
  libgdk-3-dev \
  libgdk-3-0
```

### Option 2: Set PKG_CONFIG_PATH Manually
If the `.pc` files exist but aren't in the default path:
```bash
export PKG_CONFIG_PATH="/usr/lib/x86_64-linux-gnu/pkgconfig:/usr/lib/pkgconfig"
```

### Option 3: Disable the GUI Notification Feature
If the project has an optional feature that pulls in notifications:
```bash
cargo build --no-default-features --features "basic,mdns,sqlite_bundled"
```

### Option 4: Use --no-default-features for Build
The CI build-minimal already does this:
```bash
cargo build --no-default-features --features "basic,mdns,sqlite_bundled"
```

Let's check if this job passed or failed.

---

## What We DON'T Know Yet

1. **Which crate is pulling in gdk-sys?** 
   - Need to run: `cargo tree | grep gdk` to find it
   - This would tell us if it's notify-rust or something else

2. **Does the build-minimal job pass?**
   - If yes → the issue is with full-featured builds
   - If no → the issue is more fundamental

3. **Where is gdk-3.0.pc actually located?**
   - CI runner output doesn't show package search path
   - May be in non-standard location

4. **Is GDK 3.0 even available in Ubuntu 24.04?**
   - GTK 3 is aging, Ubuntu 24.04 might prioritize GTK 4
   - We might need to check available versions

---

## The Path Forward

### Immediate: Update CI Workflow  

Add GDK 3.0 specifically:
```yaml
sudo apt-get install -y \
  libsqlite3-dev \
  pkg-config \
  libglib2.0-dev \
  libgtk-3-dev \
  libgdk-3-dev \
  libgdk-3-0 \
  libgdk-pixbuf2.0-dev \
  libcairo2-dev \
  libpango1.0-dev
```

### Alternative: Debug the Dependency

Run this locally or in CI:
```bash
cargo tree --depth 5 | grep -i gdk
cargo tree --depth 5 | grep -i gtk
cargo tree --depth 5 | grep -i notify
```

This would show exactly what's pulling in GDK.

### Best Solution: Optional Feature

If `gdk-sys` is only needed for an optional feature (notifications), make it optional:
- Modify Cargo.toml to make the feature optional
- CI can then build without it using `--no-default-features`

---

## Conclusion

The GTK/GDK dependency is NOT in the project code itself - it's a **transitive dependency** from some crate (likely `notify-rust` for desktop notifications).

Our previous fix (`libgtk-3-dev` + pixel/cairo/pango libs) was on the right track but incomplete. **GDK 3.0 is a separate package** that wasn't included.

The fix needs to be one of:
1. Add `libgdk-3-dev` and `libgdk-3-0` to apt-get
2. Find and disable the optional feature pulling in GTK
3. Set PKG_CONFIG_PATH environment variable explicitly

We should investigate which dependency is pulling in GDK before applying the next fix.

