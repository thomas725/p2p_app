#!/usr/bin/env bash
# Launch the Flutter p2p_app on a local Waydroid instance.
#
# Assumes a standard Waydroid setup:
#   - waydroid-container systemd service running (sudo systemctl start waydroid-container)
#   - adbd listening on TCP 5555 inside the container (default in Waydroid images)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$SCRIPT_DIR/apps/flutter_app"
ADB_PORT="${ADB_PORT:-5555}"

for cmd in waydroid adb flutter; do
  command -v "$cmd" >/dev/null 2>&1 || { echo "error: '$cmd' not found in PATH" >&2; exit 1; }
done

# 1. Ensure the Waydroid container and session are up.
if ! waydroid status 2>/dev/null | grep -q "Container:.*RUNNING"; then
  echo "error: Waydroid container is not running."
  echo "Start it with: sudo systemctl start waydroid-container"
  exit 1
fi

if ! waydroid status 2>/dev/null | grep -q "Session:.*RUNNING"; then
  echo "==> Starting Waydroid session..."
  nohup waydroid session start >/dev/null 2>&1 &
fi

# 2. Determine the container IP from the host ARP table on the waydroid0
#    bridge (populated as soon as the container does DHCP) and connect adb.
BRIDGE="${WAYDROID_BRIDGE:-waydroid0}"
echo "==> Waiting for Waydroid network on $BRIDGE..."
CANDIDATES=""
for _ in $(seq 1 15); do
  CANDIDATES="$(ip -o neigh show dev "$BRIDGE" 2>/dev/null | awk '/lladdr/ {print $1}')"
  [ -n "$CANDIDATES" ] && break
  sleep 1
done
[ -n "$CANDIDATES" ] || {
  echo "error: no Waydroid container found on bridge $BRIDGE (is the session running?)" >&2
  exit 1
}

SERIAL=""
for ip in $CANDIDATES; do
  s="$ip:$ADB_PORT"
  echo "==> Connecting adb to $s..."
  adb connect "$s" >/dev/null 2>&1 || true
  STATE="$(adb devices | awk -v x="$s" '$1 == x {print $2}')"
  if [ "$STATE" = "device" ]; then SERIAL="$s"; break; fi
  if [ "$STATE" = "unauthorized" ]; then
    echo "error: adb unauthorized for $s - accept the debugging prompt inside Waydroid" >&2
    exit 1
  fi
done
[ -n "$SERIAL" ] || { echo "error: could not connect adb to any of: $CANDIDATES" >&2; exit 1; }

# 3. Build the Rust native library for Android (copies .so files into jniLibs).
#    Comment out to reuse a previous build.
bash "$APP_DIR/build_rust_android.sh"

# 4. Build and launch the Flutter app on the Waydroid device.
#    `flutter clean` forces a fresh build so a stale cached APK can't keep
#    shipping the old Rust .so. Leave it enabled when chasing native changes;
#    comment it out to reuse the previous Flutter build for faster launches.
cd "$APP_DIR"
flutter clean

# The Rust `mobile` build mirrors the in-app Log-tab lines to stderr
# (src/logging.rs, `#[cfg(all(feature = "mobile", not(test)))]`). On Android
# that stderr is captured by logcat under the `stderr` tag. Echo those lines to
# this script's stdout (so they show up in the terminal and the outer `tee`)
# by filtering logcat for our `[YYYY-MM-DD HH:MM:SS` timestamp prefix that
# `push_log` always emits. Duplicates any lines `flutter run` already forwards.
APLOG_PID=""
cleanup_applog() {
  [ -n "$APLOG_PID" ] && kill "$APLOG_PID" 2>/dev/null || true
}
trap cleanup_applog EXIT

adb -s "$SERIAL" logcat -c 2>/dev/null || true
adb -s "$SERIAL" logcat 2>/dev/null \
  | grep --line-buffered -E '\[20[0-9]{2}-[0-9]{2}-[0-9]{2} ' &
APLOG_PID=$!

# `flutter run` also forwards the app's logcat, so drop our own lines from its
# stream to avoid showing them twice; the dedicated `adb logcat` above is the
# single source for the in-app Log-tab lines on this script's stdout.
flutter run -d "$SERIAL" 2>&1 \
  | grep -v --line-buffered -E '\[20[0-9]{2}-[0-9]{2}-[0-9]{2} '
