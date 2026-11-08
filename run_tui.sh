#!/usr/bin/env bash
# Three ways to build/run the desktop frontends:
#
# | Mode            | Build                                                        | Run                                                                       |
# |-----------------|--------------------------------------------------------------|---------------------------------------------------------------------------|
# | TUI only        | cargo build --bin p2p_chat_tui                               | ./target/debug/p2p_chat_tui                                               |
# | Web server only | cargo build --bin p2p_chat_server                            | ./target/debug/p2p_chat_server --assets apps/flutter_app/build/web        |
# | Both, one node  | cargo build --bin p2p_chat_tui  (default features include web) | ./target/debug/p2p_chat_tui --web --web-assets apps/flutter_app/build/web |
#
# "Both, one node" is the replacement for the removed `p2p_chat_combined`
# selector binary: the TUI and the browser drive one swarm in one process.
#
# For a web-free TUI binary (no axum/web server code):
#   cargo build --no-default-features \
#     --features "tui,mobile,mdns,kad,relay,nat,quic,tracing,attachment-gc" \
#     --bin p2p_chat_tui
#
# For a TUI-free server binary:
#   cargo build --no-default-features \
#     --features "web,mdns,kad,relay,nat,quic,tracing,attachment-gc" \
#     --bin p2p_chat_server
#
# This script runs the TUI with the default feature set, so `--web` (and
# `--web-assets <dir>`) work through it too:
#   ./run_tui.sh
#   ./run_tui.sh --web --web-assets apps/flutter_app/build/web
set -euo pipefail

cd "$(dirname "$0")"

exec cargo run --bin p2p_chat_tui -- "$@"
