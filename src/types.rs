//! Core type definitions for application events and commands

use crate::reply::ReplyRef;

#[cfg(feature = "mdns")]
use libp2p::Multiaddr;

/// String-keyed map with the standard [`RandomState`] hasher.
///
/// The app-wide map convention: every keyed lookup in this codebase is keyed
/// by a peer id, group id, or display name (all `String`), so the few dozen
/// signatures that take a `HashMap` name the hasher once here instead of
/// spelling `RandomState` at every call site. This is what the `implicit_hasher`
/// pedantic lint demands of those signatures.
///
/// [`RandomState`]: std::collections::hash_map::RandomState
pub type StringMap<V> =
    std::collections::HashMap<String, V, std::collections::hash_map::RandomState>;

/// String set with the standard [`RandomState`] hasher.
///
/// [`RandomState`]: std::collections::hash_map::RandomState
pub type StringSet = std::collections::HashSet<String, std::collections::hash_map::RandomState>;

/// What the shared single-line text input is currently capturing.
///
/// The TUI state (`AppState`) and the render-state mirror (`TuiRenderState`)
/// agree on this so input gates and hints behave identically in both layers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InputMode {
    /// Ordinary chat/command input for the active tab.
    #[default]
    Chat,
    /// Capturing a nickname edit (own nickname or a DM partner's).
    EditingNickname,
    /// Capturing the local display name this install shows for a peer
    /// (`peers.peer_local_nickname`), edited from a Peer Info tab.
    EditingLocalNickname,
    /// Capturing a replacement body for a sent message edited locally (US-005).
    EditingMessage,
    /// Capturing an emoji token to react with on the newest message (US-008).
    AddingReaction,
    /// Capturing a group name for create/join on the Groups tab.
    CreatingGroup,
    /// Capturing a Contacts-tab search query (filters the contact table).
    SearchingContacts,
    /// Capturing a peer multiaddr to dial (manual peer address entry, US-053).
    DialingPeer,
    /// Capturing a bootstrap/relay multiaddr to persist in `network_peers`
    /// (Settings tab; the kind is held in `AppState::network_peer_kind`).
    AddingNetworkPeer,
}

/// Per-tab "keep scrolled to the newest message" toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoScrollFlags {
    /// Broadcast chat tab.
    pub chat: bool,
    /// Log tab.
    pub log: bool,
}

/// Clamp a `usize` message count to the signed `i64` domain diesel uses for
/// `LIMIT`.
///
/// Counts are small in practice; an astronomical `usize` (beyond `i64::MAX`)
/// simply clamps to the maximum and loads everything.
#[must_use]
pub fn usize_limit_to_i64(limit: usize) -> i64 {
    i64::try_from(limit).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[path = "../tests/unit/unit_types.rs"]
mod tests;

/// A formatted display message for the UI layer.
///
/// Carries the rendered text and the sender's peer ID (`None` = sent by local user).
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    /// The rendered text to show in the UI.
    pub text: String,
    /// Peer ID of the sender, or `None` if sent by the local user.
    pub sender_peer_id: Option<String>,
}

/// Peer record with identification and timestamps.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PeerRecord {
    /// The peer's libp2p peer ID.
    pub peer_id: String,
    /// Timestamp (RFC 3339) when this peer was first seen.
    pub first_seen: String,
    /// Timestamp (RFC 3339) when this peer was last seen.
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
    /// Attachment descriptor, if the message carries a file (files lane §12)
    pub attachment: Option<crate::files::MessageAttachment>,
    /// True when the body arrived inside an end-to-end envelope (US-042) and
    /// was opened locally; the frontends mark such messages.
    pub encrypted: bool,
    /// Immutable reply snapshot carried on the wire (US-006).
    pub reply_to: Option<ReplyRef>,
}

/// A group message received from a peer over a per-group gossipsub topic
#[derive(Debug, Clone)]
pub struct GroupMessageEvent {
    /// The group this message belongs to
    pub group_id: String,
    /// The message text
    pub content: String,
    /// Sender's peer ID
    pub peer_id: String,
    /// Sender's nickname, if provided
    pub nickname: Option<String>,
    /// Unique message ID, if present
    pub msg_id: Option<String>,
    /// Attachment descriptor, if the message carries a file (files lane §12)
    pub attachment: Option<crate::files::MessageAttachment>,
    /// Immutable reply snapshot carried on the wire (US-006).
    pub reply_to: Option<ReplyRef>,
}

/// High-level application events from the swarm
#[derive(Debug, Clone)]
pub enum SwarmEvent {
    /// Broadcast message received from peer
    BroadcastMessage(MessageEvent),
    /// Direct message received from peer
    DirectMessage(MessageEvent),
    /// Group message received from a peer
    GroupMessage(GroupMessageEvent),
    /// Invitation to join a private group the local peer does not yet belong to
    GroupInvite {
        /// The group we are being invited to
        group_id: String,
        /// Peer ID of the peer extending the invitation
        inviter_peer: String,
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
    /// Transient DM typing state from a peer (US-009). Frontends apply their
    /// own short TTL; nothing is persisted.
    Typing {
        /// Peer ID of the person typing (or who just stopped).
        peer_id: String,
        /// Whether the peer is currently composing a DM.
        is_typing: bool,
    },
    /// A reaction was added to or removed from a message (US-008). Frontends
    /// fetch the fresh aggregated chips via `get_reactions(msg_id)`; carrying
    /// only the id keeps the event small and avoids duplicating the summary
    /// shape in the flattened Dart event envelope.
    Reaction {
        /// Stable wire id of the message that changed.
        msg_id: String,
    },
    /// Peer connected to the network
    PeerConnected(String),
    /// Peer disconnected from the network
    PeerDisconnected(String),
    /// Local address established for listening
    ListenAddrEstablished(String),
    /// An externally observed address (from `AutoNAT`/identify) — means we may be
    /// reachable from the public internet.
    ExternalAddressAdded(String),
    /// NAT reachability status change (from `AutoNAT`).
    #[cfg(feature = "nat")]
    NatStatusChanged(String),
    /// A hole-punch attempt (`DCUtR`) finished.
    #[cfg(feature = "nat")]
    HolePunchResult {
        /// Peer the hole-punch was attempted with
        peer_id: String,
        /// Whether a direct connection was established
        success: bool,
    },
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
    /// A peer advertised its device (and optionally its account) via the
    /// device-binding protocol (identity lane §11). Surfaced so the operator
    /// can decide whether to mint a signed claim for that device.
    DeviceBindingIntro {
        /// Peer ID of the advertising device
        peer_id: String,
        /// Pubkey (protobuf) of the advertising device's install keypair
        device_pubkey: Vec<u8>,
    },
    /// A signed device-binding claim for **our** device was accepted by the
    /// account holder (identity lane §11): our install's peer ID is now bound
    /// into that user's device-set.
    DeviceBindingReceived {
        /// The account our device was just bound into
        user_id: String,
    },
    /// An incoming call invite (calls lane). The `call_id` is minted by the
    /// caller and must be echoed back in the accept/reject/end signals.
    CallInvite {
        /// End-to-end session id minted by the caller
        call_id: String,
        /// Peer ID of the caller
        peer_id: String,
    },
    /// Our outgoing invite was accepted: the call is live from our side.
    CallAccepted {
        /// Session id echoed back by the callee
        call_id: String,
        /// Peer ID of the callee
        peer_id: String,
    },
    /// Our outgoing invite was declined.
    CallRejected {
        /// Session id echoed back by the callee
        call_id: String,
        /// Peer ID of the callee
        peer_id: String,
    },
    /// A call was torn down (either side sent "end"). Both sides transition
    /// their session to `Ended`.
    CallEnded {
        /// Session id of the ended call
        call_id: String,
        /// Peer ID the call was with
        peer_id: String,
    },
    /// An inbound attachment finished downloading (files lane §12). The message
    /// it rode on was already delivered; this fires when the bytes are stored
    /// and verifiable, so frontends can refresh their chip status/open affordance.
    FileReceived {
        /// Peer the file was pulled from
        peer_id: String,
        /// Content address (SHA-256 hex) of the stored file
        file_id: String,
        /// Original filename
        name: String,
        /// Absolute path of the stored blob
        path: String,
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
        /// Optional attachment descriptor (files lane §12); the sender already
        /// staged the bytes with `files::store_outgoing_file`.
        attachment: Option<crate::files::MessageAttachment>,
        /// Immutable reply snapshot (US-006); `None` for an ordinary message.
        reply_to: Option<ReplyRef>,
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
        /// Optional ID of the message this is acknowledging (receipts only)
        ack_for: Option<String>,
        /// Optional attachment descriptor (files lane §12)
        attachment: Option<crate::files::MessageAttachment>,
        /// Immutable reply snapshot (US-006); `None` for an ordinary message.
        reply_to: Option<ReplyRef>,
    },
    /// Publish a message to a group's per-group gossipsub topic
    PublishGroup {
        /// The group to send to
        group_id: String,
        /// The message text
        content: String,
        /// Optional sender nickname
        nickname: Option<String>,
        /// Optional unique message ID
        msg_id: Option<String>,
        /// Optional attachment descriptor (files lane §12)
        attachment: Option<crate::files::MessageAttachment>,
        /// Immutable reply snapshot (US-006); `None` for an ordinary message.
        reply_to: Option<ReplyRef>,
    },
    /// Subscribe to a group's per-group gossipsub topic
    SubscribeGroup {
        /// The group to join
        group_id: String,
    },
    /// Unsubscribe from a group's per-group gossipsub topic
    UnsubscribeGroup {
        /// The group to leave
        group_id: String,
    },
    /// Dial a peer by multiaddr (manual peer address entry, US-053).
    Dial {
        /// The multiaddr to dial (e.g. /ip4/1.2.3.4/tcp/4001)
        address: String,
    },
    /// Send a device-binding request (intro or signed claim) to a peer over the
    /// dedicated request-response protocol (identity lane §11).
    SendDeviceBinding {
        /// The peer to send the request to
        peer_id: String,
        /// The device-binding payload to transmit (boxed: keeps this variant
        /// near the others so the command channel stays small)
        request: Box<crate::behavior::DeviceBindingRequest>,
    },
    /// Send a call-signaling request to a peer over the dedicated
    /// request-response lane (calls lane: invite/accept/reject/end).
    SendCallSignal {
        /// The peer to send the signal to
        peer_id: String,
        /// The call-signaling payload (boxed: keeps the command channel small)
        signal: Box<crate::behavior::CallSignal>,
    },
    /// Send a transient typing state to a peer over the dedicated typing lane
    /// (US-009). Fire-and-forget from the UI's perspective: no message row,
    /// receipt, or persisted state is created.
    SendTyping {
        /// Recipient's peer ID
        peer_id: String,
        /// Whether the local user started or stopped composing.
        is_typing: bool,
    },
    /// Send a reaction change to a peer over the dedicated reaction lane
    /// (US-008). The command carries full peer lists because a DM reaction goes
    /// only to the partner while a broadcast/group reaction fans out to every
    /// peer that may hold the message.
    SendReaction {
        /// Peers that should receive the reaction change
        peer_ids: Vec<String>,
        /// The reaction change (boxed: keeps the command channel small)
        signal: Box<crate::behavior::ReactionSignal>,
    },
    /// Drop the live connection to a peer (blocklist toggle, US-054). New
    /// connections from a blocked peer are refused by the swarm handler anyway;
    /// this closes the one that existed before the block.
    DisconnectPeer {
        /// The peer whose connection should be closed
        peer_id: String,
    },
}

/// Maximum broadcast messages to keep in memory (older messages are dropped).
pub const MAX_MESSAGE_HISTORY: usize = 1000;
/// Maximum direct messages to keep per peer conversation.
pub const MAX_DM_HISTORY: usize = 1000;
