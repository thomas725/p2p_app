#!/usr/bin/env bash
set -euo pipefail

# bash run_flutter_desktop.sh 2>&1 | tee "target/flutter_desktop_$(date +%F_%H%M-%S).log"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building Rust library (release, mobile feature)..."
cargo build --release --features mobile

echo "Building Flutter Linux desktop (debug)..."
cd apps/flutter_app
# Clean stale CMake cache (breaks when symlink paths change)
rm -rf build/linux
# Ensure all Dart dependencies are fetched
flutter pub get
flutter build linux --debug

# Flutter can reuse a cached Rust core, so force the freshly built library into
# the bundle. This guarantees the launched app always runs the latest rust core.
cd "$SCRIPT_DIR"
BUNDLE_LIB="apps/flutter_app/build/linux/x64/debug/bundle/lib"
if [ -f "target/release/libp2p_app.so" ] && [ -d "$BUNDLE_LIB" ]; then
    cp -f "target/release/libp2p_app.so" "$BUNDLE_LIB/libp2p_app.so"
    echo "Rust core updated in bundle: $BUNDLE_LIB/libp2p_app.so"
else
    echo "WARNING: could not locate built Rust core or bundle lib dir" >&2
fi

echo "Launching desktop app..."
exec env LD_LIBRARY_PATH="$BUNDLE_LIB" \
  apps/flutter_app/build/linux/x64/debug/bundle/p2p_app_flutter
