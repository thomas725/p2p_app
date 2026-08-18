#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")"

exec cargo run --bin p2p_chat_tui --features "tui mdns tracing quic"
