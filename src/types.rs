use libp2p::Multiaddr;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_event_broadcast_message() {
        let event = SwarmEvent::BroadcastMessage {
            content: "hello".to_string(),
            peer_id: "peer1".to_string(),
            latency: Some("10ms".to_string()),
            nickname: Some("Alice".to_string()),
            msg_id: Some("msg-1".to_string()),
        };
        match event {
            SwarmEvent::BroadcastMessage {
                content, peer_id, ..
            } => {
                assert_eq!(content, "hello");
                assert_eq!(peer_id, "peer1");
            }
            _ => panic!("expected BroadcastMessage"),
        }
    }

    #[test]
    fn test_swarm_event_direct_message() {
        let event = SwarmEvent::DirectMessage {
            content: "hi".to_string(),
            peer_id: "peer2".to_string(),
            latency: None,
            nickname: None,
            msg_id: None,
        };
        match event {
            SwarmEvent::DirectMessage {
                content, peer_id, ..
            } => {
                assert_eq!(content, "hi");
                assert_eq!(peer_id, "peer2");
            }
            _ => panic!("expected DirectMessage"),
        }
    }

    #[test]
    fn test_swarm_event_receipt() {
        let event = SwarmEvent::Receipt {
            peer_id: "peer1".to_string(),
            ack_for: "msg-1".to_string(),
            received_at: Some(123456.0),
        };
        match event {
            SwarmEvent::Receipt {
                peer_id, ack_for, ..
            } => {
                assert_eq!(peer_id, "peer1");
                assert_eq!(ack_for, "msg-1");
            }
            _ => panic!("expected Receipt"),
        }
    }

    #[test]
    fn test_swarm_event_peer_connected() {
        let event = SwarmEvent::PeerConnected("peer1".to_string());
        match event {
            SwarmEvent::PeerConnected(id) => assert_eq!(id, "peer1"),
            _ => panic!("expected PeerConnected"),
        }
    }

    #[test]
    fn test_swarm_event_peer_disconnected() {
        let event = SwarmEvent::PeerDisconnected("peer1".to_string());
        match event {
            SwarmEvent::PeerDisconnected(id) => assert_eq!(id, "peer1"),
            _ => panic!("expected PeerDisconnected"),
        }
    }

    #[test]
    fn test_swarm_event_listen_addr() {
        let event = SwarmEvent::ListenAddrEstablished("/ip4/127.0.0.1/tcp/1234".to_string());
        match event {
            SwarmEvent::ListenAddrEstablished(addr) => assert!(addr.contains("127.0.0.1")),
            _ => panic!("expected ListenAddrEstablished"),
        }
    }

    #[test]
    fn test_swarm_command_publish() {
        let cmd = SwarmCommand::Publish {
            content: "hello".to_string(),
            nickname: Some("Alice".to_string()),
            msg_id: Some("msg-1".to_string()),
        };
        match cmd {
            SwarmCommand::Publish {
                content,
                nickname,
                msg_id: _,
            } => {
                assert_eq!(content, "hello");
                assert_eq!(nickname, Some("Alice".to_string()));
            }
            _ => panic!("expected Publish"),
        }
    }

    #[test]
    fn test_swarm_command_send_dm() {
        let cmd = SwarmCommand::SendDm {
            peer_id: "peer1".to_string(),
            content: "hi".to_string(),
            nickname: Some("Bob".to_string()),
            msg_id: Some("dm-1".to_string()),
            ack_for: Some("orig-msg".to_string()),
        };
        match cmd {
            SwarmCommand::SendDm {
                peer_id,
                content,
                ack_for,
                ..
            } => {
                assert_eq!(peer_id, "peer1");
                assert_eq!(content, "hi");
                assert_eq!(ack_for, Some("orig-msg".to_string()));
            }
            _ => panic!("expected SendDm"),
        }
    }

    #[test]
    fn test_swarm_event_clone() {
        let event = SwarmEvent::PeerConnected("peer1".to_string());
        let cloned = event.clone();
        match cloned {
            SwarmEvent::PeerConnected(id) => assert_eq!(id, "peer1"),
            _ => panic!("expected PeerConnected"),
        }
    }

    #[test]
    fn test_swarm_command_clone() {
        let cmd = SwarmCommand::Publish {
            content: "test".to_string(),
            nickname: None,
            msg_id: None,
        };
        let cloned = cmd.clone();
        match cloned {
            SwarmCommand::Publish { content, .. } => assert_eq!(content, "test"),
            _ => panic!("expected Publish"),
        }
    }
}

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
