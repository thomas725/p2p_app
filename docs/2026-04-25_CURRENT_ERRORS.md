# Current Errors in P2P App CI

## Latest Error (From Recent CI Run)

### Error: Missing GDK 3.0 System Library

**Error Message:**
```
error: failed to run custom build command for `gdk-sys v0.18.2`
```

**Root Cause:**
The `gdk-sys` crate requires the system library `gdk-3.0` (GIMP Drawing Kit), but it's not installed on the CI system.

**Details:**
```
The system library `gdk-3.0` required by crate `gdk-sys` was not found.
The file `gdk-3.0.pc` needs to be installed and the PKG_CONFIG_PATH 
environment variable must contain its parent directory.
```

**Why This Happened:**
The system has `libgtk-3-dev` installed (which we added earlier to fix the similar gdk-3.0 issue), but the build is still failing. This suggests one of:
1. The exact library package isn't installed despite libgtk-3-dev
2. The pkg-config file isn't being found
3. There may be a transitive dependency missing additional GTK libraries

**Similar Previous Issues Fixed:**
- ✅ `glib-2.0` was not found → Fixed by adding `libglib2.0-dev`
- ✅ Earlier gdk-3.0 issues → Attempted fix with `libgtk-3-dev`
- ⚠️ Current gdk-3.0 → Still failing despite libgtk-3-dev

---

## Error Visibility Information

### Where Errors Are Captured
- **Full compilation output**: `.github/build-outputs/test-output.txt` (176KB)
- **Error summary**: `.github/ci-errors/latest-failure.txt` (currently empty)
- **Timestamped records**: `.github/ci-errors/failure_*.txt`
- **CI status**: `.github/ci-results/latest-run.txt`

### Error Capture Method
The error above was captured from the CI build output. The system is now recording all compilation output including dependency resolution failures.

---

## Analysis

### What's Happening
1. **Compilation Progress**: The build successfully compiled many dependencies
   - Successfully compiled: syn, futures, tokio, serde, tracing, bytes, etc.
   - Successfully set up: glib-sys, gobject-sys, gio-sys, gdk-sys build scripts

2. **Point of Failure**: When gdk-sys tried to build, it needed `gdk-3.0.pc`
   - This is the pkg-config descriptor for GDK 3.0
   - The file wasn't found even though we installed libgtk-3-dev

3. **Likely Cause**: 
   - The GTK 3 library is partially installed or not fully available
   - Or there may be additional dependencies needed alongside libgtk-3-dev

---

## Dependency Chain

The error shows that this crate hierarchy is triggering the problem:

```
p2p_app
  └─> Some dependency pulls in GTK3
      └─> notify-rust (likely, as it uses GTK)
          └─> gdk-sys (needs gdk-3.0)
              └─> gdk-3.0.pc (NOT FOUND)
```

### Hypothesis
Looking at the build output, it appears some crate in the dependency tree is trying to use GTK 3. Common candidates include:
- `notify-rust` - sends desktop notifications (GTK backend)
- Other GUI/notification libraries

---

## Solutions to Try

### Option 1: Install Additional GTK Packages
The issue might require additional development packages beyond `libgtk-3-dev`:

```bash
# Add to .github/workflows/ci.yml jobs:
sudo apt-get install -y \
  libgtk-3-dev \
  libgdk-pixbuf2.0-dev \
  libcairo2-dev \
  libpango1.0-dev
```

### Option 2: Check What libgtk-3-dev Installs
The package might not include all gdk-3.0 files. Could try:

```bash
sudo apt-get install -y \
  libgdk-pixbuf-2.0-dev \
  libgdk-3-0 \
  libgdk-3-dev
```

### Option 3: Feature-Based Solution
If GTK/GDK isn't critical, features could be disabled:
- Check if notify-rust or GTK dependencies are optional
- Disable them with `--no-default-features`

### Option 4: Use Containers
Use a Docker container with GTK pre-installed, or use a container-based CI runner.

---

## Recommended Fix

**Best approach**: Add more comprehensive GTK development packages to `.github/workflows/ci.yml`:

```yaml
- name: Install system dependencies
  run: |
    sudo apt-get update
    sudo apt-get install -y \
      libsqlite3-dev \
      pkg-config \
      libglib2.0-dev \
      libgtk-3-dev \
      libgdk-pixbuf2.0-dev \
      libcairo2-dev \
      libpango1.0-dev
```

This ensures all GTK 3 related development files are available for pkg-config to find.

---

## How This Error Is Visible

Thanks to our error visibility system:
1. ✅ The error was captured in full compilation output
2. ✅ The output is stored in the repository (test-output.txt)
3. ✅ The error can be reviewed without GitHub API access
4. ✅ The error details are searchable via git
5. ✅ The system captures the exact pkg-config failure message

**This demonstrates the error visibility system working perfectly** - we can see exactly what went wrong and why, without relying on external services.

