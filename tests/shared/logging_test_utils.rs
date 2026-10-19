use tracing_subscriber::filter::{LevelFilter, Targets};

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
