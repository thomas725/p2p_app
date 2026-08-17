#!/usr/bin/env bash
set -euo pipefail

missing=0

check() {
  if command -v "$1" >/dev/null 2>&1; then
    printf "ok: %s\n" "$1"
  else
    printf "missing: %s\n" "$1"
    missing=1
  fi
}

check cargo
check flutter
check java
check sdkmanager
check adb

if command -v cargo >/dev/null 2>&1; then
  if cargo ndk --version >/dev/null 2>&1; then
    printf "ok: cargo ndk\n"
  else
    printf "missing: cargo-ndk (install with: cargo install cargo-ndk)\n"
    missing=1
  fi
fi

exit "$missing"
