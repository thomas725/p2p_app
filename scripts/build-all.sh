#!/usr/bin/env bash
# Build every desktop frontend in one go: the TUI binary AND the Flutter web
# bundle (served by the Rust node server).
#
# Architecture (docs/2026-10-18_web_frontend.md): the browser app is a pure
# display client. The node stays native inside the Rust web server
# (`p2p_chat_server`), and the browser talks JSON-over-WebSocket to it. Nothing
# is compiled to WASM, so the web half is just a normal Rust build plus
# `flutter build web`.
#
# Both Rust binaries come out of ONE cargo invocation, so the shared dependency
# graph is compiled once and the two frontends get an identical feature set
# (same migrations, same attachment GC, same node behaviour).
#
# Usage:
#   scripts/build-all.sh                  # release build of both frontends
#   scripts/build-all.sh --debug          # fast debug build
#   scripts/build-all.sh --tui-only       # only the TUI binary
#   scripts/build-all.sh --web-only       # only the server binary + web bundle
#
# Then run (from the repo root):
#   ./target/release/p2p_chat_tui
#   ./target/release/p2p_chat_server --assets apps/flutter_app/build/web
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

profile=release
cargo_flags=(--release)
flutter_flags=()
build_tui=1
build_web=1

usage() {
    # Print the leading comment block (everything after the shebang line),
    # stripping the comment markers. No hard-coded line numbers to drift.
    awk 'NR > 1 && /^#/ { sub(/^# ?/, ""); print; next } NR > 1 { exit }' "${BASH_SOURCE[0]}"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --debug)
            profile=debug
            cargo_flags=()
            flutter_flags=(--debug)
            ;;
        --release)
            profile=release
            cargo_flags=(--release)
            flutter_flags=()
            ;;
        --tui-only)
            build_web=0
            ;;
        --web-only)
            build_tui=0
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            printf 'unknown option: %s\n\n' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

command -v cargo >/dev/null 2>&1 || {
    printf 'error: cargo not found in PATH\n' >&2
    exit 1
}
if [[ $build_web -eq 1 ]]; then
    command -v flutter >/dev/null 2>&1 || {
        printf 'error: flutter not found in PATH (needed for the web bundle)\n' >&2
        exit 1
    }
fi

# ---- Rust: the TUI binary and/or the node server binary --------------------
# `web` is in the default feature set (Cargo.toml), so no --features needed.
# One invocation for both bins keeps a single feature resolution.
if [[ $build_tui -eq 1 && $build_web -eq 1 ]]; then
    printf '==> cargo build --bin p2p_chat_tui --bin p2p_chat_server (%s)\n' "$profile"
    cargo build "${cargo_flags[@]}" --bin p2p_chat_tui --bin p2p_chat_server
elif [[ $build_tui -eq 1 ]]; then
    printf '==> cargo build --bin p2p_chat_tui (%s)\n' "$profile"
    cargo build "${cargo_flags[@]}" --bin p2p_chat_tui
else
    printf '==> cargo build --bin p2p_chat_server (%s)\n' "$profile"
    cargo build "${cargo_flags[@]}" --bin p2p_chat_server
fi

# ---- Flutter: the web bundle ----------------------------------------------
if [[ $build_web -eq 1 ]]; then
    printf '==> flutter build web (%s)\n' "$profile"
    # --no-wasm-dry-run silences the benign "Wasm dry run failed" warning that
    # flutter_rust_bridge 2.13.0 itself emits on a JS-only build.
    (cd apps/flutter_app && flutter build web "${flutter_flags[@]}" --no-wasm-dry-run)
fi

# ---- Summary ---------------------------------------------------------------
printf '\nBuilt:\n'
if [[ $build_tui -eq 1 ]]; then
    printf '  tui    : %s/target/%s/p2p_chat_tui\n' "$repo_root" "$profile"
fi
if [[ $build_web -eq 1 ]]; then
    printf '  node   : %s/target/%s/p2p_chat_server\n' "$repo_root" "$profile"
    printf '  assets : %s/apps/flutter_app/build/web\n' "$repo_root"
fi

if [[ $build_tui -eq 1 ]]; then
    printf '\nRun the TUI:\n'
    printf '  ./target/%s/p2p_chat_tui\n' "$profile"
fi

if [[ $build_web -eq 1 ]]; then
    printf '\nRun the web frontend (loopback only, no token — the TUI trust model):\n'
    printf '  ./target/%s/p2p_chat_server --assets apps/flutter_app/build/web\n' "$profile"
    printf '\nFor LAN exposure add a token (required off loopback):\n'
    printf '  ./target/%s/p2p_chat_server --bind 0.0.0.0 --token <secret> --assets apps/flutter_app/build/web\n' "$profile"
fi
