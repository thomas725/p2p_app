//! Core type definitions for application events and commands

use libp2p::Multiaddr;

#[cfg(test)]
#[path = "../tests/unit/unit_types.rs"]
mod tests;

/// A formatted display message for the UI layer.
///
/// Carries the rendered text and the sender's peer ID (`None` = sent by local user).
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub text: String,
    pub sender_peer_id: Option<String>,
}

/// Peer record with identification and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerRecord {
    pub peer_id: String,
    pub first_seen: String,
    pub last_seen: String,
}

impl std::fmt::Display for PeerRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.peer_id, self.last_seen)
    }
}

/// Data common to both broadcast and direct messages from the swarm
#[derive(Debug, Clone)]
pub struct MessageEvent {
    /// The message text
    pub content: String,
    /// Sender's peer ID
    pub peer_id: String,
    /// Round-trip latency string, if known
    pub latency: Option<String>,
    /// Sender's nickname, if provided
    pub nickname: Option<String>,
    /// Unique message ID, if present
    pub msg_id: Option<String>,
}

/// High-level application events from the swarm
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Broadcast message received from peer
    BroadcastMessage(MessageEvent),
    /// Direct message received from peer
    DirectMessage(MessageEvent),
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
