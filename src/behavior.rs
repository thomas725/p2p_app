//! P2P network behavior and protocol definitions

use crate::files::MessageAttachment;
use crate::network::NetworkSize;
use crate::p2plog_debug;
use libp2p::gossipsub;
use libp2p::ping;
use libp2p::request_response;
use libp2p::swarm::NetworkBehaviour;
use serde::{Deserialize, Serialize};

/// Chat protocol name for direct messages
pub const DM_PROTOCOL_NAME: &str = "/p2p-chat/dm/1.0.0";

/// Device-binding protocol name (identity lane §11).
///
/// One-shot signed claims that bind a device (install keypair) into a remote
/// account's device-set. `intro` requests advertise a device+account; `claim`
/// requests carry the account-key-signed claim that the receiver verifies
/// before binding.
pub const DEVICE_BINDING_PROTOCOL_NAME: &str = "/p2p-chat/device-binding/1.0.0";

/// Call-signaling protocol name (calls lane).
///
/// One-shot JSON signals over a dedicated `request_response` lane, mirroring the
/// device-binding pattern. Kinds: `"invite"` (the callee is asked to ring),
/// `"accept"`/`"reject"` (the callee's answer to an invite), and `"end"`
/// (either side tears the call down). The signaling slice carries no media:
/// `CallSignal::descriptor` is reserved for the later offer/answer payload.
pub const CALL_PROTOCOL_NAME: &str = "/p2p-chat/call/1.0.0";

/// File-transfer protocol name (files lane §12).
///
/// A dedicated `request_response` lane for content-addressed attachments. The
/// receiver drives every transfer: it requests the metadata (`"meta"`), then
/// pulls fixed-size chunks (`"chunk"`) sequentially until the file is complete.
/// The sender serves purely from its content-addressed store (prepared by
/// `files::store_outgoing_file` before the message is sent, so the blob is
/// already on disk when the receiver asks).
pub const FILE_TRANSFER_PROTOCOL_NAME: &str = "/p2p-chat/files/1.0.0";

/// Default chat topic
pub const CHAT_TOPIC: &str = "test-net";

/// Direct message content with timestamp and sender info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DirectMessage {
    /// The message content/text
    pub content: String,
    /// Unix timestamp when the message was created
    pub timestamp: i64,
    /// Optional timestamp of when the message was sent to the network
    pub sent_at: Option<f64>,
    /// Optional nickname of the sender
    pub nickname: Option<String>,
    /// Optional unique identifier for this message
    pub msg_id: Option<String>,
    /// Optional ID of the message this is acknowledging/replying to
    pub ack_for: Option<String>,
    /// Optional timestamp of when this message was received
    pub received_at: Option<f64>,
    /// Stable `group_id` a private-group DM is attributed to; `None` = plain DM
    pub group_id: Option<String>,
    /// Optional attachment descriptor (files lane §12); the bytes are fetched
    /// separately over the file lane by the receiver.
    pub attachment: Option<MessageAttachment>,
    /// Optional end-to-end sealed body (US-042). When present the receiver
    /// opens it and uses the decrypted `content`/`nickname`/`attachment`
    /// instead of the (empty) plaintext fields above.
    pub encrypted: Option<crate::e2e::EncryptedDm>,
}

/// Broadcast message content with timestamp and sender info
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct BroadcastMessage {
    /// The message content/text
    pub content: String,
    /// Optional timestamp of when the message was sent
    pub sent_at: Option<f64>,
    /// Optional nickname of the sender
    pub nickname: Option<String>,
    /// Optional unique identifier for this message
    pub msg_id: Option<String>,
    /// Optional attachment descriptor (files lane §12)
    pub attachment: Option<MessageAttachment>,
}

/// Group message content traveling over a per-group gossipsub topic
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GroupMessage {
    /// The group this message belongs to (must match the topic's group id)
    pub group_id: String,
    /// The message content/text
    pub content: String,
    /// Optional timestamp of when the message was sent
    pub sent_at: Option<f64>,
    /// Optional nickname of the sender
    pub nickname: Option<String>,
    /// Optional unique identifier for this message
    pub msg_id: Option<String>,
    /// Optional attachment descriptor (files lane §12)
    pub attachment: Option<MessageAttachment>,
}

/// Backfill request sent on a group's backfill topic on (re)subscribe
/// (backlog §11.5).
///
/// Gossipsub never replays, so a member that was offline — or joined after the
/// group started — misses the gap. On subscribe the member publishes this
/// requesting everything newer than its `after` watermark; **any** online
/// member whose stored history is newer answers by re-publishing the bounded
/// recent messages as ordinary [`GroupMessage`]s on the main group topic, where
/// the `(group_id, msg_id)` unique index (plus the `nil:` synthetic key for
/// messages without an `msg_id`) absorbs every overlap. The request itself is
/// anonymous control traffic — never persisted.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct GroupBackfillRequest {
    /// The group whose gap is being requested (must match the topic's group id).
    pub group_id: String,
    /// Only messages stored with `sent_at` strictly newer than this are asked
    /// for (`0.0` requests the member's full recent history).
    pub after: f64,
}

/// JSON codec for direct message protocol
pub type ChatCodec = libp2p::request_response::json::codec::Codec<DirectMessage, DirectMessage>;

/// File-transfer exchange payload (files lane §12; receiver-driven).
///
/// One wire type, two request `kind`s and one response type:
///
/// * request `"meta"`  — ask for `file_id`'s metadata (name/size/mime/chunking).
/// * request `"chunk"` — pull chunk `chunk_index` of `file_id`.
/// * response         — `kind == "meta"` carries the metadata handshake;
///   `kind == "chunk"` carries `data`; `kind == "error"` carries a `reason`
///   (unknown file, oversized, out-of-range chunk).
///
/// CBOR (not JSON) is used on purpose: chunk payloads are opaque byte strings,
/// which CBOR serializes natively (~1:1) whereas JSON would expand a 1 MiB
/// chunk ~4x into a base64/array text blob.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FileTransfer {
    /// `"meta"` | `"chunk"`
    pub kind: String,
    /// Content address (SHA-256 hex) of the whole file
    pub file_id: String,
    /// Zero-based chunk index (request `"chunk"` only)
    pub chunk_index: Option<u64>,
}

/// Response to a [`FileTransfer`] request.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct FileTransferResponse {
    /// `"meta"` | `"chunk"` | `"error"`
    pub kind: String,
    /// Content address (SHA-256 hex) of the whole file
    pub file_id: String,
    /// Original filename (`"meta"` only)
    pub name: Option<String>,
    /// Total byte length (`"meta"` only)
    pub size: Option<u64>,
    /// Best-effort MIME type hint (`"meta"` only)
    pub mime: Option<String>,
    /// Number of fixed-size chunks (`"meta"` only; `ceil(size / chunk_size)`)
    pub total_chunks: Option<u64>,
    /// Chunk size in bytes (`"meta"` only)
    pub chunk_size: Option<u64>,
    /// The chunk index echoed back (`"chunk"` only)
    pub chunk_index: Option<u64>,
    /// Raw chunk bytes (`"chunk"` only)
    pub data: Option<Vec<u8>>,
    /// Human-readable failure description (`"error"` only)
    pub reason: Option<String>,
}

/// CBOR codec for the file-transfer protocol
pub type FileTransferCodec =
    libp2p::request_response::cbor::codec::Codec<FileTransfer, FileTransferResponse>;

/// Device-binding exchange payload (identity lane §11; signed claims).
///
/// One wire type, two `kind`s:
///
/// * `"intro"`  — sender advertises its device (and account, once provisioned)
///   so the account holder can decide whether to mint a claim for it.
/// * `"claim"` — a signed device→user claim. `user_id`, `account_pubkey`,
///   `claim_id`, `bound_at` and `signature` are all present. The receiver
///   `save_user`s the root (upsert), verifies `signature` over
///   `user_id ‖ device_pubkey ‖ claim_id` against *that user's* stored account
///   pubkey, and only then binds (`save_device_binding`). Never auto-mints.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceBindingRequest {
    /// `"intro"` or `"claim"`
    pub kind: String,
    /// Account the claim/intro is about (`None` for an unprovisioned intro)
    pub user_id: Option<String>,
    /// Account pubkey that signs claims for `user_id` (claim; root of trust)
    pub account_pubkey: Option<Vec<u8>>,
    /// Account's self-declared username (display lane)
    pub self_username: Option<String>,
    /// Account's self-declared nickname (display lane)
    pub self_nickname: Option<String>,
    /// Peer ID of the subject device (intro: sender; claim: the device claimed)
    pub peer_id: String,
    /// Pubkey (protobuf) of the subject device (install keypair)
    pub device_pubkey: Vec<u8>,
    /// Unique claim identifier (present on claim)
    pub claim_id: Option<String>,
    /// Unix seconds when the claim was minted (present on claim; display only)
    pub bound_at: Option<f64>,
    /// Signed claim over `user_id ‖ device_pubkey ‖ claim_id` (present on claim)
    pub signature: Option<Vec<u8>>,
}

/// Ack for a device-binding request (`kind` echoes the request kind).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceBindingAck {
    /// Whether the exchange was accepted and (for claims) bound
    pub ok: bool,
    /// Human-readable reason on failure
    pub reason: Option<String>,
}

/// JSON codec for the device-binding protocol
pub type DeviceBindingCodec =
    libp2p::request_response::json::codec::Codec<DeviceBindingRequest, DeviceBindingAck>;

/// Call-signaling exchange payload (calls lane; signaling first, media later).
///
/// One wire type, four `kind`s — the caller mints the end-to-end `call_id`
/// (via `gen_msg_id`) and every side echoes it back, so each side can match
/// signals to its live session.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CallSignal {
    /// `"invite"` | `"accept"` | `"reject"` | `"end"`
    pub kind: String,
    /// Session id minted by the caller; echoed by every later signal.
    pub call_id: String,
    /// Unix seconds when the signal was sent.
    pub timestamp: f64,
    /// Reserved: media offer/answer descriptor (SDP/ICE) when media lands.
    pub descriptor: Option<String>,
}

impl CallSignal {
    /// Build a signal of the given kind for a session, stamping the current
    /// time and leaving the reserved media descriptor empty.
    #[must_use]
    pub fn new(kind: &str, call_id: String) -> Self {
        Self {
            kind: kind.to_string(),
            call_id,
            timestamp: crate::current_timestamp(),
            descriptor: None,
        }
    }
}

/// Ack for a call-signaling request (`ok` = the signal was understood).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct CallSignalAck {
    /// Whether the signal was accepted
    pub ok: bool,
    /// Human-readable reason on failure
    pub reason: Option<String>,
}

/// JSON codec for the call-signaling protocol
pub type CallSignalCodec = libp2p::request_response::json::codec::Codec<CallSignal, CallSignalAck>;

/// Combined network behavior for p2p chat
#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    /// Gossipsub protocol handler for broadcast messages
    pub gossipsub: gossipsub::Behaviour,
    /// Request-response protocol handler for direct messages
    pub request_response: request_response::Behaviour<ChatCodec>,
    /// Request-response protocol handler for signed device-binding exchanges
    /// (identity lane §11: intro + account-signed claims)
    pub device_binding: request_response::Behaviour<DeviceBindingCodec>,
    /// Request-response protocol handler for call signaling (calls lane:
    /// invite/accept/reject/end; media transport lands on a later slice)
    pub call_signaling: request_response::Behaviour<CallSignalCodec>,
    /// Request-response protocol handler for content-addressed file transfers
    /// (files lane §12): the receiver pulls metadata and chunks, the sender
    /// serves them from its on-disk store.
    pub file_transfer: request_response::Behaviour<FileTransferCodec>,
    /// mDNS protocol handler for peer discovery (when mdns feature is enabled)
    #[cfg(feature = "mdns")]
    pub mdns: libp2p::mdns::tokio::Behaviour,
    /// Kademlia DHT for internet-wide peer discovery (when kad feature is enabled)
    #[cfg(feature = "kad")]
    pub kad: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    /// Identify protocol: exchanges peer info + listen addresses (needed by kad)
    #[cfg(feature = "kad")]
    pub identify: libp2p::identify::Behaviour,
    /// Circuit-relay server (when relay feature is enabled): lets this node
    /// relay connections for peers behind NAT.
    #[cfg(feature = "relay")]
    pub relay_server: libp2p::relay::Behaviour,
    /// Circuit-relay client (when relay feature is enabled): lets this node
    /// use relay servers to become reachable from behind NAT.
    #[cfg(feature = "relay")]
    pub relay_client: libp2p::relay::client::Behaviour,
    /// `AutoNAT` (when nat feature is enabled): detects whether this node is
    /// publicly reachable.
    #[cfg(feature = "nat")]
    pub autonat: libp2p::autonat::Behaviour,
    /// `DCUtR` (when nat feature is enabled): hole punching through relays.
    #[cfg(feature = "nat")]
    pub dcutr: libp2p::dcutr::Behaviour,
    /// Ping protocol for connection liveness: detects dead peers (e.g. a node
    /// that was killed without closing the connection) and closes the
    /// connection so `ConnectionClosed` fires and the peer is marked offline.
    pub ping: ping::Behaviour,
}

// AppBehaviourEvent is auto-generated by the NetworkBehaviour derive macro

/// Build the gossipsub behaviour, tuning history/heartbeat to network size.
fn build_gossipsub(
    key: &libp2p_identity::Keypair,
    network_size: NetworkSize,
) -> Result<gossipsub::Behaviour, String> {
    // Larger networks need a longer history and a slower heartbeat to avoid
    // redundant re-broadcasts.
    let (history_gossip, history_length, heartbeat_secs) = match network_size {
        NetworkSize::Small => (3, 20, 1),
        NetworkSize::Medium => (6, 30, 2),
        NetworkSize::Large => (12, 50, 5),
    };
    let config = gossipsub::ConfigBuilder::default()
        .max_transmit_size(262_144)
        .history_gossip(history_gossip)
        .history_length(history_length)
        .heartbeat_interval(std::time::Duration::from_secs(heartbeat_secs))
        .build()
        .map_err(|e| format!("invalid gossipsub config: {e}"))?;
    gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(key.clone()), config)
        .map_err(|e| format!("Failed to create gossipsub behavior: {e}"))
}

/// Build the app behavior with appropriate protocols and configurations
///
/// # Arguments
/// * `key` - libp2p identity keypair
/// * `network_size` - Network size enum for tuning behavior parameters
///
/// # Errors
/// Returns an error if any underlying libp2p protocol behavior fails to build
/// (e.g. an invalid gossipsub configuration or mDNS setup).
pub fn build_behaviour(
    key: &libp2p_identity::Keypair,
    network_size: NetworkSize,
    #[cfg(feature = "relay")] relay_client: libp2p::relay::client::Behaviour,
) -> Result<AppBehaviour, String> {
    p2plog_debug(format!(
        "Building behavior for network size: {network_size:?}"
    ));
    // Configure Gossipsub based on network size: larger networks need a longer
    // history and a slower heartbeat to avoid redundant re-broadcasts.
    let gossipsub = build_gossipsub(key, network_size)?;

    let request_response = request_response::Behaviour::new(
        [(
            libp2p::StreamProtocol::new(DM_PROTOCOL_NAME),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    let device_binding = request_response::Behaviour::new(
        [(
            libp2p::StreamProtocol::new(DEVICE_BINDING_PROTOCOL_NAME),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    let call_signaling = request_response::Behaviour::new(
        [(
            libp2p::StreamProtocol::new(CALL_PROTOCOL_NAME),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    // Chunk payloads ride in the response (cbor cap: 10 MiB); requests are tiny.
    let file_transfer = request_response::Behaviour::new(
        [(
            libp2p::StreamProtocol::new(FILE_TRANSFER_PROTOCOL_NAME),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    // Liveness checks: ping every 10s, treat a missing pong within 10s as a
    // dead connection. This surfaces killed/disconnected peers within ~10-20s
    // instead of waiting on the (often never-elapsed) idle connection timeout.
    let ping = ping::Behaviour::new(
        ping::Config::new()
            .with_interval(std::time::Duration::from_secs(10))
            .with_timeout(std::time::Duration::from_secs(10)),
    );

    #[cfg(feature = "mdns")]
    let mdns = libp2p::mdns::tokio::Behaviour::new(
        libp2p::mdns::Config::default(),
        libp2p::PeerId::from_public_key(&key.public()),
    )
    .map_err(|e| format!("invalid mdns: {e}"))?;

    #[cfg(feature = "kad")]
    let kad = {
        let store = libp2p::kad::store::MemoryStore::new(key.public().to_peer_id());
        // Join the public (IPFS) Kademlia DHT for peer discovery, but stay in
        // client mode so we never store/serve records for other networks' peers
        // (avoids holding the global DHT's record set in memory).
        let mut kad = libp2p::kad::Behaviour::new(key.public().to_peer_id(), store);
        kad.set_mode(Some(libp2p::kad::Mode::Client));
        kad
    };
    #[cfg(feature = "kad")]
    let identify = libp2p::identify::Behaviour::new(libp2p::identify::Config::new(
        // Use the IPFS identify protocol name so public bootstrap nodes can
        // exchange addresses with us; our own nodes still identify fine.
        "/ipfs/id/1.0.0".to_string(),
        key.public(),
    ));

    #[cfg(feature = "relay")]
    let relay_server = {
        // Do not act as an open relay for the public internet by default: only
        // accept reservations when explicitly opted in via P2P_RELAY_SERVER=1.
        let mut cfg = libp2p::relay::Config::default();
        if std::env::var("P2P_RELAY_SERVER").is_err() {
            cfg.max_reservations = 0;
            cfg.max_reservations_per_peer = 0;
            cfg.max_circuits = 0;
            cfg.max_circuits_per_peer = 0;
        }
        libp2p::relay::Behaviour::new(key.public().to_peer_id(), cfg)
    };

    #[cfg(feature = "nat")]
    let autonat = libp2p::autonat::Behaviour::new(
        key.public().to_peer_id(),
        libp2p::autonat::Config::default(),
    );

    #[cfg(feature = "nat")]
    let dcutr = libp2p::dcutr::Behaviour::new(key.public().to_peer_id());

    Ok(AppBehaviour {
        gossipsub,
        request_response,
        device_binding,
        call_signaling,
        file_transfer,
        #[cfg(feature = "mdns")]
        mdns,
        #[cfg(feature = "kad")]
        kad,
        #[cfg(feature = "kad")]
        identify,
        #[cfg(feature = "relay")]
        relay_server,
        #[cfg(feature = "relay")]
        relay_client,
        #[cfg(feature = "nat")]
        autonat,
        #[cfg(feature = "nat")]
        dcutr,
        ping,
    })
}

/// Build a fully configured libp2p swarm for chat.
///
/// Sets up TCP (with nodelay), optional QUIC transport, the app behaviour
/// with adaptive network size, and a 60-second idle connection timeout.
///
/// # Errors
/// Returns an error if the libp2p identity cannot be loaded or if the swarm
/// (transport + behaviour) fails to build.
pub fn build_swarm(
    network_size: NetworkSize,
) -> color_eyre::Result<libp2p::swarm::Swarm<AppBehaviour>> {
    use std::time::Duration;

    let key = crate::get_libp2p_identity()?;

    let base = libp2p::SwarmBuilder::with_existing_identity(key)
        .with_tokio()
        .with_tcp(
            libp2p::tcp::Config::default().nodelay(true),
            libp2p::noise::Config::new,
            libp2p::yamux::Config::default,
        )?;

    #[cfg(feature = "quic")]
    let base = base.with_quic();

    #[cfg(feature = "relay")]
    let swarm = base
        .with_relay_client(libp2p::noise::Config::new, libp2p::yamux::Config::default)?
        .with_behaviour(
            |key, relay_client| -> Result<AppBehaviour, Box<dyn std::error::Error + Send + Sync>> {
                build_behaviour(key, network_size, relay_client).map_err(
                    |e| -> Box<dyn std::error::Error + Send + Sync> {
                        Box::new(std::io::Error::other(e))
                    },
                )
            },
        )?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_mins(1)))
        .build();

    #[cfg(not(feature = "relay"))]
    let swarm = base
        .with_behaviour(
            |key| -> Result<AppBehaviour, Box<dyn std::error::Error + Send + Sync>> {
                build_behaviour(key, network_size).map_err(
                    |e| -> Box<dyn std::error::Error + Send + Sync> {
                        Box::new(std::io::Error::other(e))
                    },
                )
            },
        )?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_mins(1)))
        .build();

    Ok(swarm)
}

#[cfg(test)]
#[path = "../tests/unit/unit_behavior.rs"]
mod tests;
