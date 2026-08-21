#!/usr/bin/env bash
# Build the Rust native library for Android and copy to jniLibs.
# Run this BEFORE `flutter build apk`.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CARGO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
JNLIBS_DIR="$SCRIPT_DIR/android/app/src/main/jniLibs"
RUST_PROFILE="${RUST_PROFILE:-release}"

declare -A TARGETS=(
    ["arm64-v8a"]="aarch64-linux-android"
    ["armeabi-v7a"]="armv7-linux-androideabi"
    ["x86_64"]="x86_64-linux-android"
)

# Find NDK clang if ANDROID_HOME is set
NDK_HOME="${ANDROID_HOME:-$HOME/.android-sdk}/ndk/29.0.14206865"
NDK_BIN="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"

CARGO_ARGS=(
    build --lib
    --features mobile
    --profile "$RUST_PROFILE"
)

# Set cross-compilation env vars
# NOTE: bash identifiers cannot contain hyphens, so use the underscore form
# (the `cc` crate looks up CC_<target> with '-' replaced by '_').
export CC_aarch64_linux_android="$NDK_BIN/aarch64-linux-android35-clang"
export CC_armv7_linux_androideabi="$NDK_BIN/armv7a-linux-androideabi35-clang"
export CC_x86_64_linux_android="$NDK_BIN/x86_64-linux-android35-clang"
export AR_aarch64_linux_android="$NDK_BIN/llvm-ar"
export AR_armv7_linux_androideabi="$NDK_BIN/llvm-ar"
export AR_x86_64_linux_android="$NDK_BIN/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_BIN/aarch64-linux-android35-clang"
export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$NDK_BIN/armv7a-linux-androideabi35-clang"
export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK_BIN/x86_64-linux-android35-clang"
# Clear host env that pollutes cross-compilation
unset CC CFLAGS LDFLAGS PKG_CONFIG_PATH BINDGEN_EXTRA_CLANG_ARGS 2>/dev/null || true
export PKG_CONFIG_ALLOW_CROSS=1

echo "==> Building Rust for Android..."
cd "$CARGO_ROOT"

for abi in "${!TARGETS[@]}"; do
    target="${TARGETS[$abi]}"
    CARGO_ARGS+=(--target "$target")
done

cargo "${CARGO_ARGS[@]}"

echo "==> Copying .so files to jniLibs..."
for abi in "${!TARGETS[@]}"; do
    target="${TARGETS[$abi]}"
    so_file="$CARGO_ROOT/target/$target/$RUST_PROFILE/libp2p_app.so"
    dest="$JNLIBS_DIR/$abi"
    mkdir -p "$dest"
    cp "$so_file" "$dest/libp2p_app.so"
    echo "  $abi -> $(ls -lh "$dest/libp2p_app.so" | awk '{print $5}')"
done

echo "==> Done."
