use libp2p::Multiaddr;

/// High-level application events from the swarm
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Broadcast message received from peer
    BroadcastMessage {
        /// The message text
        content: String,
        /// Sender's peer ID
        peer_id: String,
        /// Round-trip latency string, if known
        latency: Option<String>,
        /// Sender's nickname, if provided
        nickname: Option<String>,
        /// Unique message ID, if present
        msg_id: Option<String>,
    },
    /// Direct message received from peer
    DirectMessage {
        /// The message text
        content: String,
        /// Sender's peer ID
        peer_id: String,
        /// Round-trip latency string, if known
        latency: Option<String>,
        /// Sender's nickname, if provided
        nickname: Option<String>,
        /// Unique message ID, if present
        msg_id: Option<String>,
    },
    /// Receipt confirmation received from a peer (for either broadcast or direct messages).
    Receipt {
        /// Peer ID of the sender who acknowledged the message
        peer_id: String,
        /// Message ID this receipt is acknowledging
        ack_for: String,
        /// Timestamp (seconds since epoch) when the peer received the message
        received_at: Option<f64>,
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
        /// Discovered peer's ID
        peer_id: String,
        /// Addresses at which the peer can be reached
        addresses: Vec<Multiaddr>,
    },
    /// Peer expired via mDNS
    #[cfg(feature = "mdns")]
    PeerExpired {
        /// Expired peer's ID
        peer_id: String,
    },
}

/// Commands sent to the swarm task
#[derive(Debug, Clone)]
pub enum SwarmCommand {
    /// Publish a message to the broadcast topic
    Publish {
        /// The message text to broadcast
        content: String,
        /// Optional sender nickname to include in the message
        nickname: Option<String>,
        /// Optional unique message ID
        msg_id: Option<String>,
    },
    /// Send a direct message to a peer
    SendDm {
        /// Recipient's peer ID
        peer_id: String,
        /// The message text
        content: String,
        /// Optional sender nickname
        nickname: Option<String>,
        /// Optional unique message ID
        msg_id: Option<String>,
        /// Optional ID of the message this is acknowledging
        ack_for: Option<String>,
    },
}
