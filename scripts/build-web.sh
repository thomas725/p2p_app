#!/usr/bin/env bash
# Build the web frontend: the node server binary plus the Flutter web bundle.
#
# The browser app is a pure display client over the node's JSON-over-WebSocket
# API (docs/2026-10-18_web_frontend.md) — nothing is compiled to WASM, so this
# is just the normal Rust build plus `flutter build web`.
#
# Usage:
#   scripts/build-web.sh [--debug]     # debug builds are much faster
#
# Then run (from the repo root):
#   ./target/release/p2p_chat_server --assets apps/flutter_app/build/web
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

profile=release
cargo_flags=(--release)
flutter_flags=()
if [[ "${1:-}" == "--debug" ]]; then
  profile=debug
  cargo_flags=()
  flutter_flags=(--debug)
fi

printf '==> cargo build --bin p2p_chat_server (%s)\n' "$profile"
# `web` is in the default feature set (Cargo.toml), so no --features needed.
cargo build "${cargo_flags[@]}" --bin p2p_chat_server

printf '==> flutter build web (%s)\n' "$profile"
(cd apps/flutter_app && flutter build web "${flutter_flags[@]}")

printf '\nBuilt:\n'
printf '  node   : %s/target/%s/p2p_chat_server\n' "$repo_root" "$profile"
printf '  assets : %s/apps/flutter_app/build/web\n' "$repo_root"
printf '\nRun (loopback only, no token — the TUI trust model):\n'
printf '  ./target/%s/p2p_chat_server --assets apps/flutter_app/build/web\n' "$profile"
printf '\nFor LAN exposure add a token (required off loopback):\n'
printf '  ./target/%s/p2p_chat_server --bind 0.0.0.0 --token <secret> --assets apps/flutter_app/build/web\n' "$profile"
