use super::TUI_LOGS;
use tracing_subscriber::filter::{LevelFilter, Targets};

/// Test utility: Clears the TUI log storage for test isolation.
pub fn clear_tui_logs() {
    if let Some(logs) = TUI_LOGS.get()
        && let Ok(mut guard) = logs.lock()
    {
        guard.clear();
    }
}

/// Build a tracing `Targets` filter that denies noisy internal modules
/// and keeps useful networking events at DEBUG level.
#[must_use]
pub fn tracing_filter() -> Targets {
    Targets::new()
        .with_target("multistream_select", LevelFilter::OFF)
        .with_target("yamux::connection", LevelFilter::OFF)
        .with_target("libp2p_core::transport::choice", LevelFilter::OFF)
        .with_target("libp2p_mdns::behaviour::iface", LevelFilter::OFF)
        .with_target("libp2p_gossipsub::behaviour", LevelFilter::OFF)
        .with_target("libp2p_swarm", LevelFilter::DEBUG)
        .with_target("libp2p_tcp", LevelFilter::DEBUG)
        .with_target("libp2p_quic::transport", LevelFilter::DEBUG)
        .with_target("libp2p_mdns::behaviour", LevelFilter::DEBUG)
        .with_default(LevelFilter::WARN)
}
