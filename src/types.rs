use libp2p::Multiaddr;

/// High-level application events from the swarm
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Broadcast message received from peer
    BroadcastMessage {
        content: String,
        peer_id: String,
        latency: Option<String>,
    },
    /// Direct message received from peer
    DirectMessage {
        content: String,
        peer_id: String,
        latency: Option<String>,
    },
    /// Peer connected to the network
    PeerConnected(String),
    /// Peer disconnected from the network
    PeerDisconnected(String),
    /// Local address established for listening
    ListenAddrEstablished(String),
    /// Peer discovered via mDNS
    #[cfg(feature = "mdns")]
    PeerDiscovered {
        peer_id: String,
        addresses: Vec<Multiaddr>,
    },
    /// Peer expired via mDNS
    #[cfg(feature = "mdns")]
    PeerExpired { peer_id: String },
}

/// Commands sent to the swarm task
#[derive(Debug, Clone)]
pub enum SwarmCommand {
    /// Publish a message to the broadcast topic
    Publish(String),
    /// Send a direct message to a peer
    SendDm { peer_id: String, content: String },
}
