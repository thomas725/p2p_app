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

echo "Launching desktop app..."
cd "$SCRIPT_DIR"
exec env LD_LIBRARY_PATH=apps/flutter_app/build/linux/x64/debug/bundle/lib \
  apps/flutter_app/build/linux/x64/debug/bundle/p2p_app_flutter
