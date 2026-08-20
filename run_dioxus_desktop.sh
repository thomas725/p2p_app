#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

exec cargo run --no-default-features --features "dioxus-desktop basic mdns tracing quic" --bin p2p_chat_dioxus
