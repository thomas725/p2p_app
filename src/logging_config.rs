//! Tracing and logging configuration

/// Build a tracing `Targets` filter that denies noisy internal modules
/// and keeps useful networking events at DEBUG level.
///
/// This filter reduces log spam from libp2p's verbose internal components while
/// preserving important events for network debugging.
///
/// # Denylist (set to OFF):
/// - `multistream_select` - protocol negotiation internals
/// - `yamux::connection` - stream multiplexing pings/RTT
/// - `libp2p_core::transport::choice` - unreadable type names on dial failure
/// - `libp2p_mdns::behaviour::iface` - startup-only noise
/// - `libp2p_gossipsub::behaviour` - graft/prune spam from direct peers
///
/// # DEBUG level:
/// - `libp2p_swarm` - connection lifecycle, listener addresses
/// - `libp2p_tcp` - dial attempts, listen addresses
/// - `libp2p_quic::transport` - listen addresses
/// - `libp2p_mdns::behaviour` - peer discovery events
///
/// # Default:
/// Everything else defaults to WARN level
#[cfg(feature = "tracing")]
pub fn tracing_filter() -> tracing_subscriber::filter::Targets {
    use tracing_subscriber::filter::{LevelFilter, Targets};
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
