#!/usr/bin/env bash
# Three ways to build/run the desktop frontends:
#
# | Mode            | Build                                                                                                                       | Run                                                                       |
# |-----------------|-----------------------------------------------------------------------------------------------------------------------------|---------------------------------------------------------------------------|
# | TUI only        | cargo build --bin p2p_chat_tui --no-default-features --features "tui,mobile,mdns,kad,relay,nat,quic,tracing,attachment-gc"  | ./target/debug/p2p_chat_tui                                               |
# | Web server only | cargo build --bin p2p_chat_server                                                                                           | ./target/debug/p2p_chat_server --assets apps/flutter_app/build/web        |
# | Both, one node  | cargo build --bin p2p_chat_tui  (default features include web)                                                              | ./target/debug/p2p_chat_tui --web --web-assets apps/flutter_app/build/web |
#
# TUI-only means web-free: the TUI-only build above leaves out the `web`
# feature (and therefore the axum web server). The default feature set includes
# `web`, so a plain `cargo build --bin p2p_chat_tui` is the "both, one node"
# binary, not the TUI-only one.
#
# "Both, one node" is the replacement for the removed `p2p_chat_combined`
# selector binary: the TUI and the browser drive one swarm in one process.
#
# For a TUI-free server binary:
#   cargo build --no-default-features \
#     --features "web,mdns,kad,relay,nat,quic,tracing,attachment-gc" \
#     --bin p2p_chat_server
set -euo pipefail

cd "$(dirname "$0")"

# Web-free TUI-only build; extra arguments (e.g. --public-dht) are forwarded.
exec cargo run --no-default-features \
  --features "tui,mobile,mdns,kad,relay,nat,quic,tracing,attachment-gc" \
  --bin p2p_chat_tui -- "$@"
