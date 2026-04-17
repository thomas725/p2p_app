pub mod models_insertable;
pub mod models_queryable;
pub mod schema;

#[cfg(feature = "tui")]
pub use tui::{DynamicTabs, TabContent};

#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{gossipsub, request_response, swarm::NetworkBehaviour};

use std::collections::VecDeque;
use std::sync::OnceLock;

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
///
/// # DEBUG level:
/// - `libp2p_swarm` - connection lifecycle, listener addresses
/// - `libp2p_gossipsub::behaviour` - mesh changes, heartbeats, peer subs
/// - `libp2p_tcp` - dial attempts, listen addresses
/// - `libp2p_quic::transport` - listen addresses
/// - `libp2p_mdns::behaviour` - peer discovery events
///
/// # Default:
/// Everything else defaults to WARN level
pub fn tracing_filter() -> tracing_subscriber::filter::Targets {
    use tracing_subscriber::filter::{LevelFilter, Targets};
    Targets::new()
        .with_target("multistream_select", LevelFilter::OFF)
        .with_target("yamux::connection", LevelFilter::OFF)
        .with_target("libp2p_core::transport::choice", LevelFilter::OFF)
        .with_target("libp2p_mdns::behaviour::iface", LevelFilter::OFF)
        .with_target("libp2p_swarm", LevelFilter::DEBUG)
        .with_target("libp2p_gossipsub::behaviour", LevelFilter::INFO)
        .with_target("libp2p_tcp", LevelFilter::DEBUG)
        .with_target("libp2p_quic::transport", LevelFilter::DEBUG)
        .with_target("libp2p_mdns::behaviour", LevelFilter::DEBUG)
        .with_default(LevelFilter::WARN)
}

/// Default broadcast topic for chat messages
pub const CHAT_TOPIC: &str = "test-net";
/// Protocol name for direct messaging via libp2p request-response
pub const DM_PROTOCOL_NAME: &str = "/p2p-chat/dm/1.0.0";

/// Type alias for the main network behavior combining gossipsub, request-response, and mDNS
pub type P2pAppBehaviour = AppBehaviour;

static LOG_TUI_CALLBACK: OnceLock<Box<dyn Fn(String) + Send + Sync>> = OnceLock::new();
static LOGS: OnceLock<std::sync::Mutex<VecDeque<String>>> = OnceLock::new();

/// Initialize the logging system by creating the global logs queue.
///
/// Must be called once at application startup before any logging occurs.
pub fn init_logging() {
    LOGS.get_or_init(|| std::sync::Mutex::new(VecDeque::new()));
}

/// Remove ANSI escape codes from a string (e.g., color/formatting codes).
///
/// Useful for cleaning terminal output before storing in logs or displaying in TUI.
///
/// # Arguments
/// * `s` - Input string that may contain ANSI escape sequences
///
/// # Returns
/// A new String with all ANSI codes stripped
///
/// # Example
/// ```
/// # use p2p_app::strip_ansi_codes;
/// let colored = "\x1b[32mHello\x1b[0m";
/// assert_eq!(strip_ansi_codes(colored), "Hello");
/// ```
pub fn strip_ansi_codes(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            result.push(c);
        }
    }
    result
}

/// Format a naive UTC datetime as a local time string with timezone.
///
/// # Arguments
/// * `time` - NaiveDateTime in UTC
///
/// # Returns
/// Formatted string in timezone-aware local time: "YYYY-MM-DD HH:MM:SS +ZZZZ"
pub fn format_peer_datetime(time: chrono::NaiveDateTime) -> String {
    let local = time.and_utc().with_timezone(&chrono::Local);
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

/// Get the current time as a formatted local time string with timezone.
///
/// # Returns
/// Formatted string: "YYYY-MM-DD HH:MM:SS +ZZZZ" in local time
pub fn now_timestamp() -> String {
    let local = chrono::Local::now();
    local.format("%Y-%m-%d %H:%M:%S %z").to_string()
}

pub fn push_log(message: impl Into<String>) {
    let ts = chrono::Local::now().format("%H:%M:%S.%3f");
    let formatted = format!("[{}] {}", ts, message.into());
    let has_callback = LOG_TUI_CALLBACK.get().is_some();
    if let Some(callback) = LOG_TUI_CALLBACK.get() {
        (callback)(formatted.clone());
    }
    if let Some(logs) = LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > 1000 {
            l.pop_front();
        }
    }
    if !has_callback {
        eprintln!("{}", formatted);
    }
}

pub fn set_tui_log_callback<F>(callback: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    let _ = LOG_TUI_CALLBACK.set(Box::new(callback));
}

pub fn get_tui_logs() -> VecDeque<String> {
    LOGS.get()
        .map(|m| m.lock().expect("TUI logs mutex not poisoned").clone())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn p2plog(level: &str, msg: String) {
    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
    let formatted = format!("[{}] [{}] {}", ts, level, msg);

    if let Some(callback) = LOG_TUI_CALLBACK.get() {
        (callback)(formatted.clone());
    }

    if let Some(logs) = LOGS.get()
        && let Ok(mut l) = logs.lock()
    {
        l.push_back(formatted.clone());
        if l.len() > 1000 {
            l.pop_front();
        }
    }

    // Only print to stderr if TUI is not active (no callback set)
    if LOG_TUI_CALLBACK.get().is_none() {
        eprintln!("{}", formatted);
    }
}

#[allow(dead_code)]
pub fn p2plog_debug(msg: String) {
    p2plog("DEBUG", msg);
}
#[allow(dead_code)]
pub fn p2plog_info(msg: String) {
    p2plog("INFO", msg);
}
#[allow(dead_code)]
pub fn p2plog_warn(msg: String) {
    p2plog("WARN", msg);
}
#[allow(dead_code)]
pub fn p2plog_error(msg: String) {
    p2plog("ERROR", msg);
}

#[derive(NetworkBehaviour)]
pub struct AppBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub request_response: request_response::Behaviour<ChatCodec>,
    #[cfg(feature = "mdns")]
    pub mdns: mdns::tokio::Behaviour,
}

/// Build the libp2p NetworkBehaviour combining gossipsub, request-response, and mDNS.
///
/// Configures gossipsub mesh parameters based on network size (Small/Medium/Large)
/// to optimize for the expected scale of the network.
///
/// # Arguments
/// * `key` - The node's identity keypair
/// * `network_size` - Adaptive network classification for tuning gossipsub
///
/// # Returns
/// An AppBehaviour instance ready for use with Swarm
pub fn build_behaviour(key: &libp2p_identity::Keypair, network_size: NetworkSize) -> AppBehaviour {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::Duration;

    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    let (heartbeat_interval, gossip_lazy, mesh_n, mesh_n_low, mesh_n_high, flood_publish) =
        match network_size {
            NetworkSize::Small => (Duration::from_millis(500), 1, 1, 1, 2, true),
            NetworkSize::Medium => (Duration::from_secs(1), 3, 6, 4, 8, false),
            NetworkSize::Large => (Duration::from_secs(2), 6, 8, 6, 12, false),
        };

    let mut config_builder = gossipsub::ConfigBuilder::default();
    config_builder
        .heartbeat_interval(heartbeat_interval)
        .validation_mode(gossipsub::ValidationMode::Strict)
        .message_id_fn(message_id_fn)
        .gossip_lazy(gossip_lazy)
        .mesh_n(mesh_n)
        .mesh_n_low(mesh_n_low)
        .mesh_n_high(mesh_n_high);

    if flood_publish {
        config_builder.flood_publish(true);
    }

    let gossipsub_config = config_builder
        .build()
        .expect("gossipsub config should be valid");

    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(key.clone()),
        gossipsub_config,
    )
    .expect("gossipsub should be created");

    #[cfg(feature = "mdns")]
    let mdns = mdns::tokio::Behaviour::new(
        mdns::Config {
            query_interval: Duration::from_millis(500),
            ..Default::default()
        },
        key.public().to_peer_id(),
    )
    .expect("mDNS should be created");

    let request_response = request_response::Behaviour::<ChatCodec>::new(
        vec![(
            libp2p::StreamProtocol::new(DM_PROTOCOL_NAME),
            libp2p::request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default().with_request_timeout(Duration::from_secs(5)),
    );

    AppBehaviour {
        gossipsub,
        request_response,
        #[cfg(feature = "mdns")]
        mdns,
    }
}
use crate::schema::identities::dsl::identities;
use crate::schema::messages::dsl::messages;
use crate::schema::peers::dsl::peers;
use crate::{
    models_insertable::{NewIdentity, NewMessage, NewPeer, NewPeerSession},
    models_queryable::{Identity, Message, Peer},
};
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl as _,
    SelectableHelper as _, SqliteConnection,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenvy::dotenv;
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectMessage {
    pub content: String,
    pub timestamp: i64,
    pub sent_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub content: String,
    pub sent_at: Option<f64>,
}

pub type ChatCodec = libp2p_request_response::json::codec::Codec<DirectMessage, DirectMessage>;

/// Establish a connection to the SQLite database and run pending migrations.
///
/// Loads `DATABASE_URL` from environment or `.env` file, defaulting to "sqlite.db".
/// Automatically runs all pending Diesel migrations on first call.
///
/// # Returns
/// A new SqliteConnection with all migrations applied, or an error if connection/migration fails
///
/// # Errors
/// - If database file cannot be found or created
/// - If migrations fail to execute
pub fn sqlite_connect() -> color_eyre::Result<SqliteConnection> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or("sqlite.db".to_owned());
    let mut conn = SqliteConnection::establish(&database_url)
        .wrap_err_with(|| format!("Error connecting to {database_url}"))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| eyre!(format!("Error executing migrations on {database_url}: {e}")))?;
    Ok(conn)
}

/// Get the database URL from environment or default value.
///
/// Respects `DATABASE_URL` environment variable or `.env` file, defaulting to "sqlite.db".
#[must_use]
pub fn get_database_url() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite.db".to_owned())
}

pub fn get_libp2p_identity() -> color_eyre::Result<libp2p_identity::Keypair> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").ok();

    if let Some(_db_url) = &database_url {
        let conn = &mut sqlite_connect()?;
        if let Ok(rows) = identities.select(Identity::as_select()).load(conn) {
            for row in rows {
                match libp2p_identity::Keypair::from_protobuf_encoding(&row.key) {
                    Ok(i) => {
                        return Ok(i);
                    }
                    Err(e) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!("invalid identity stored: {row:?} - {e}");
                    }
                }
            }
        }
        #[cfg(feature = "tracing")]
        tracing::warn!("no valid identity found in database, generating and storing new one");
        let keypair = libp2p_identity::Keypair::generate_ed25519();
        match keypair.to_protobuf_encoding() {
            Ok(key) => {
                let i = NewIdentity {
                    key,
                    last_tcp_port: None,
                    last_quic_port: None,
                    self_nickname: None,
                };
                match diesel::insert_into(schema::identities::table)
                    .values(&i)
                    .returning(Identity::as_returning())
                    .get_result(conn)
                {
                    Ok(i) => {
                        #[cfg(feature = "tracing")]
                        tracing::info!("inserted new identity: {i:?}");
                    }
                    Err(e) => {
                        #[cfg(feature = "tracing")]
                        tracing::error!("failed to insert identity {i:?}: {e}");
                    }
                }
            }
            Err(e) => {
                #[cfg(feature = "tracing")]
                tracing::error!("failed to encode identity: {e}");
            }
        }
        Ok(keypair)
    } else {
        #[cfg(feature = "tracing")]
        tracing::info!("no DATABASE_URL set, generating ephemeral identity");
        Ok(libp2p_identity::Keypair::generate_ed25519())
    }
}

/// Save a message to the database.
///
/// Inserts a new message with the given content and metadata.
/// Messages are marked as unsent initially; use `mark_message_sent()` after transmission.
///
/// # Arguments
/// * `content` - Message text (can be empty string)
/// * `peer_id` - Sender's peer ID (None for local user)
/// * `topic` - Topic for broadcast messages (e.g., "test-net")
/// * `is_direct` - True for direct peer-to-peer, false for broadcast
/// * `target_peer` - Recipient peer ID (required if `is_direct` is true)
///
/// # Returns
/// The inserted Message with auto-generated id and timestamps
pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
) -> color_eyre::Result<Message> {
    let conn = &mut sqlite_connect()?;
    let new_msg = NewMessage {
        content: content.to_string(),
        peer_id: peer_id.map(|s| s.to_string()),
        topic: topic.to_string(),
        sent: 0,
        is_direct: if is_direct { 1 } else { 0 },
        target_peer: target_peer.map(|s| s.to_string()),
    };
    let msg = diesel::insert_into(schema::messages::table)
        .values(&new_msg)
        .returning(Message::as_returning())
        .get_result(conn)?;
    Ok(msg)
}

/// Get all unsent broadcast messages for a topic, ordered by creation time.
pub fn get_unsent_messages(topic: &str) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut sqlite_connect()?;
    let msgs = messages
        .filter(schema::messages::topic.eq(topic))
        .filter(schema::messages::sent.eq(0))
        .filter(schema::messages::is_direct.eq(0))
        .order(schema::messages::created_at.asc())
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Get all unsent direct messages to a specific peer, ordered by creation time.
pub fn get_unsent_direct_messages(target_peer: &str) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut sqlite_connect()?;
    let msgs = messages
        .filter(schema::messages::target_peer.eq(target_peer))
        .filter(schema::messages::sent.eq(0))
        .filter(schema::messages::is_direct.eq(1))
        .order(schema::messages::created_at.asc())
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Mark a message as sent by ID.
pub fn mark_message_sent(id: i32) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::update(schema::messages::table.filter(schema::messages::id.eq(id)))
        .set(schema::messages::sent.eq(1))
        .execute(conn)?;
    Ok(())
}

/// Load broadcast messages for a topic, newest first, limited to count.
pub fn load_messages(topic: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut sqlite_connect()?;
    let msgs = messages
        .filter(schema::messages::topic.eq(topic))
        .filter(schema::messages::is_direct.eq(0))
        .order(schema::messages::created_at.desc())
        .limit(limit as i64)
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Load direct messages with a peer, oldest first, limited to count.
pub fn load_direct_messages(target_peer: &str, limit: usize) -> color_eyre::Result<Vec<Message>> {
    let conn = &mut sqlite_connect()?;
    let msgs = messages
        .filter(schema::messages::target_peer.eq(target_peer))
        .filter(schema::messages::is_direct.eq(1))
        .order(schema::messages::created_at.asc())
        .limit(limit as i64)
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
}

/// Save or update a peer in the database.
///
/// If peer already exists (by peer_id), updates addresses and last_seen timestamp.
/// Otherwise inserts a new peer record with current timestamp.
///
/// # Arguments
/// * `peer_id` - Unique peer identifier
/// * `addresses` - List of multiaddrs where this peer can be reached
///
/// # Returns
/// The saved or updated Peer record
pub fn save_peer(peer_id: &str, addresses: &[String]) -> color_eyre::Result<Peer> {
    let conn = &mut sqlite_connect()?;
    let addresses_str = addresses.join(",");
    let now = chrono::Utc::now().naive_utc();

    let new_peer = NewPeer {
        peer_id: peer_id.to_string(),
        addresses: addresses_str.clone(),
        first_seen: now,
        last_seen: now,
        peer_local_nickname: None,
        received_nickname: None,
    };

    let peer = diesel::insert_into(schema::peers::table)
        .values(&new_peer)
        .on_conflict(schema::peers::peer_id)
        .do_update()
        .set((
            schema::peers::addresses.eq(&addresses_str),
            schema::peers::last_seen.eq(now),
        ))
        .returning(Peer::as_returning())
        .get_result(conn)?;
    Ok(peer)
}

/// Load all known peers, ordered by most recently seen first.
pub fn load_peers() -> color_eyre::Result<Vec<Peer>> {
    let conn = &mut sqlite_connect()?;
    let peers_list = peers
        .order(schema::peers::last_seen.desc())
        .select(Peer::as_select())
        .load(conn)?;
    Ok(peers_list)
}

/// Delete a peer from the database by peer_id.
pub fn remove_peer(peer_id: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::delete(schema::peers::table.filter(schema::peers::peer_id.eq(peer_id)))
        .execute(conn)?;
    Ok(())
}

pub fn save_peer_session(concurrent_peers: i32) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    let new_session = NewPeerSession {
        concurrent_peers,
        recorded_at: chrono::Utc::now().naive_utc(),
    };
    diesel::insert_into(schema::peer_sessions::table)
        .values(&new_session)
        .execute(conn)?;
    Ok(())
}

pub fn save_listen_ports(tcp_port: Option<i32>, quic_port: Option<i32>) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::update(schema::identities::table)
        .set((
            schema::identities::last_tcp_port.eq(tcp_port),
            schema::identities::last_quic_port.eq(quic_port),
        ))
        .execute(conn)?;
    Ok(())
}

pub fn load_listen_ports() -> color_eyre::Result<(Option<i32>, Option<i32>)> {
    let conn = &mut sqlite_connect()?;
    let result = schema::identities::table
        .select((
            schema::identities::last_tcp_port,
            schema::identities::last_quic_port,
        ))
        .first::<(Option<i32>, Option<i32>)>(conn)
        .optional()?;
    Ok(result.unwrap_or((None, None)))
}

pub fn get_average_peer_count() -> color_eyre::Result<f64> {
    let conn = &mut sqlite_connect()?;
    let sessions = schema::peer_sessions::table
        .select(schema::peer_sessions::concurrent_peers)
        .load::<i32>(conn)?;
    if sessions.is_empty() {
        return Ok(0.0);
    }
    let sum: i64 = sessions.iter().map(|&c| c as i64).sum();
    Ok(sum as f64 / sessions.len() as f64)
}

pub fn get_recent_peer_count() -> color_eyre::Result<i32> {
    let conn = &mut sqlite_connect()?;
    let last = schema::peer_sessions::table
        .select(schema::peer_sessions::concurrent_peers)
        .order(schema::peer_sessions::recorded_at.desc())
        .first::<i32>(conn)
        .optional()?;
    Ok(last.unwrap_or(0))
}

/// Adaptive network size classification based on average peer count.
///
/// Used to configure gossipsub mesh parameters appropriately for network conditions.
/// Smaller networks use aggressive flooding; larger networks use lazy gossip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSize {
    /// 0-3 peers: Use flood_publish, aggressive heartbeat
    Small,
    /// 4-15 peers: Balanced mesh topology
    Medium,
    /// 16+ peers: Larger mesh, lazy gossip
    Large,
}

impl NetworkSize {
    /// Classify network size based on average peer count.
    ///
    /// # Arguments
    /// * `avg` - Average number of concurrent peers from historical data
    ///
    /// # Returns
    /// NetworkSize classification for configuring gossipsub behavior
    pub fn from_peer_count(avg: f64) -> Self {
        match avg as i32 {
            0..=3 => NetworkSize::Small,
            4..=15 => NetworkSize::Medium,
            _ => NetworkSize::Large,
        }
    }
}

pub fn get_network_size() -> color_eyre::Result<NetworkSize> {
    let avg = get_average_peer_count()?;
    Ok(NetworkSize::from_peer_count(avg))
}

#[cfg(feature = "tui")]
pub mod tui {
    use std::collections::{BTreeMap, VecDeque};

    pub const TEST_MESSAGES: &[&str] = &[
        "[You] Hello world",
        "[Peer1] How are you?",
        "[You] I'm good, thanks!",
        "[Peer2] Welcome to the chat",
        "[You] Thanks!",
    ];

    #[derive(Clone, Debug)]
    pub struct TuiTestState {
        pub messages: VecDeque<String>,
        pub chat_message_peers: Vec<String>,
        pub active_tab: usize,
        pub chat_list_state_offset: usize,
        pub unread_broadcasts: u32,
        pub unread_dms: BTreeMap<String, u32>,
        pub terminal_width: usize,
    }

    impl TuiTestState {
        pub fn new() -> Self {
            Self::with_messages(TEST_MESSAGES.iter().map(|s| s.to_string()).collect())
        }

        pub fn with_messages(messages: VecDeque<String>) -> Self {
            Self::with_messages_and_width(messages, 80)
        }

        pub fn with_messages_and_width(messages: VecDeque<String>, width: usize) -> Self {
            let chat_message_peers: Vec<String> = messages
                .iter()
                .map(|m| {
                    if m.starts_with("[You]") {
                        "You".to_string()
                    } else if m.contains('[') {
                        m.split('[')
                            .nth(1)
                            .map(|s| s.split(']').next().unwrap_or("").to_string())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                })
                .collect();

            Self {
                messages,
                chat_message_peers,
                active_tab: 0,
                chat_list_state_offset: 0,
                unread_broadcasts: 0,
                unread_dms: BTreeMap::new(),
                terminal_width: width,
            }
        }

        pub fn handle_mouse_click(&self, row: u16, _col: u16) -> Option<String> {
            let first_msg_row = self.first_message_row();
            if row < first_msg_row {
                return None;
            }

            let content_width = (self.terminal_width as u16).saturating_sub(4);
            let clicked_row_in_list = row - first_msg_row;

            let mut current_row: u16 = 0;
            for msg_idx in self.chat_list_state_offset..self.messages.len() {
                let msg = &self.messages[msg_idx];
                let manual_breaks = msg.matches('\n').count() as u16;
                let wrapped_lines = ((msg.len() as u16) / content_width)
                    .saturating_add(1)
                    .max(1);
                let msg_lines = manual_breaks + wrapped_lines;

                if clicked_row_in_list >= current_row
                    && clicked_row_in_list < current_row + msg_lines
                {
                    return self.chat_message_peers.get(msg_idx).cloned();
                }

                current_row += msg_lines;
            }

            None
        }

        pub fn list_header_start_row(&self) -> u16 {
            let tabs_rows = 3;
            let notification_rows = if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() {
                1
            } else {
                0
            };
            tabs_rows + notification_rows
        }

        pub fn first_message_row(&self) -> u16 {
            self.list_header_start_row() + 2
        }

        pub fn calculate_content_start_row(&self) -> u16 {
            self.first_message_row()
        }

        pub fn handle_tab_click(&mut self, row: u16) {
            self.active_tab = match row {
                0..=2 => (row / 3) as usize,
                _ => self.active_tab,
            };
        }

        pub fn handle_notification_click(&self, col: u16) -> Option<NotificationTarget> {
            if self.unread_broadcasts > 0 || !self.unread_dms.is_empty() {
                if col < 20 {
                    Some(NotificationTarget::Broadcasts)
                } else {
                    self.unread_dms
                        .keys()
                        .next()
                        .cloned()
                        .map(NotificationTarget::Dm)
                }
            } else {
                None
            }
        }
    }

    #[derive(Clone, Debug)]
    pub enum NotificationTarget {
        Broadcasts,
        Dm(String),
    }

    impl Default for TuiTestState {
        fn default() -> Self {
            Self::new()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq, Default)]
    pub enum TabId {
        #[default]
        Chat,
        Peers,
        Direct,
        Log,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct DmTab {
        pub peer_id: String,
        pub messages: VecDeque<String>,
    }

    impl DmTab {
        pub fn new(peer_id: String) -> Self {
            Self {
                peer_id,
                messages: VecDeque::new(),
            }
        }

        pub fn with_messages(peer_id: String, messages: VecDeque<String>) -> Self {
            Self { peer_id, messages }
        }

        #[must_use]
        pub fn short_id(&self) -> String {
            self.peer_id
                .chars()
                .rev()
                .take(8)
                .collect::<String>()
                .chars()
                .rev()
                .collect()
        }
    }

    #[derive(Clone, Debug, Default)]
    pub struct DynamicTabs {
        pub dm_tabs: Vec<DmTab>,
    }

    impl DynamicTabs {
        pub fn new() -> Self {
            Self {
                dm_tabs: Vec::new(),
            }
        }

        pub fn add_dm_tab(&mut self, peer_id: String) -> usize {
            if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
                return pos + 2;
            }
            self.dm_tabs.push(DmTab::new(peer_id));
            self.dm_tabs.len() + 1
        }

        pub fn remove_dm_tab(&mut self, peer_id: &str) -> Option<usize> {
            if let Some(pos) = self.dm_tabs.iter().position(|t| t.peer_id == peer_id) {
                self.dm_tabs.remove(pos);
                return Some(pos + 2);
            }
            None
        }

        pub fn get_dm_tab(&self, peer_id: &str) -> Option<&DmTab> {
            self.dm_tabs.iter().find(|t| t.peer_id == peer_id)
        }

        pub fn get_dm_tab_mut(&mut self, peer_id: &str) -> Option<&mut DmTab> {
            self.dm_tabs.iter_mut().find(|t| t.peer_id == peer_id)
        }

        #[must_use]
        pub fn dm_tab_count(&self) -> usize {
            self.dm_tabs.len()
        }

        pub fn dm_tab_titles(&self) -> Vec<String> {
            self.dm_tabs
                .iter()
                .map(|t| format!("{} (X)", t.short_id()))
                .collect()
        }

        pub fn all_titles(&self) -> Vec<String> {
            let mut titles = vec!["Chat".to_string(), "Peers".to_string()];
            titles.extend(self.dm_tab_titles());
            titles.push("Log".to_string());
            titles
        }

        pub fn tab_index_to_content(&self, tab_idx: usize) -> TabContent {
            let log_index = 2 + self.dm_tabs.len();
            match tab_idx {
                0 => TabContent::Chat,
                1 => TabContent::Peers,
                idx if idx == log_index => TabContent::Log,
                idx if idx >= 2 && idx < log_index => {
                    let dm_idx = idx - 2;
                    if let Some(tab) = self.dm_tabs.get(dm_idx) {
                        TabContent::Direct(tab.peer_id.clone())
                    } else {
                        TabContent::Chat
                    }
                }
                _ => TabContent::Chat,
            }
        }

        #[must_use]
        pub fn total_tab_count(&self) -> usize {
            3 + self.dm_tabs.len()
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum TabContent {
        Chat,
        Peers,
        Direct(String),
        Log,
    }

    impl TabContent {
        #[must_use]
        pub fn peer_id(&self) -> Option<&str> {
            match self {
                TabContent::Direct(id) => Some(id),
                _ => None,
            }
        }

        #[must_use]
        pub fn is_input_enabled(&self) -> bool {
            matches!(self, TabContent::Chat | TabContent::Direct(_))
        }
    }

    impl TabId {
        pub fn index(&self) -> usize {
            match self {
                TabId::Chat => 0,
                TabId::Peers => 1,
                TabId::Direct => 2,
                TabId::Log => 3,
            }
        }

        pub fn from_index(idx: usize) -> Self {
            match idx {
                0 => TabId::Chat,
                1 => TabId::Peers,
                2 => TabId::Direct,
                3 => TabId::Log,
                _ => TabId::Chat,
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_peer_datetime() {
        use chrono::NaiveDateTime;
        let dt = NaiveDateTime::parse_from_str("2024-01-01 12:00:00", "%Y-%m-%d %H:%M:%S")
            .expect("Failed to parse date");
        let formatted = format_peer_datetime(dt);
        assert!(formatted.contains("2024"));
    }

    #[test]
    fn test_strip_ansi_codes() {
        let ansi_text = "\x1b[32mHello\x1b[0m";
        let clean = strip_ansi_codes(ansi_text);
        assert_eq!(clean, "Hello");
    }

    #[test]
    fn test_now_timestamp_format() {
        let ts = now_timestamp();
        assert!(ts.contains('-'));
        assert!(ts.contains(':'));
    }

    #[test]
    fn test_network_size_from_peer_count() {
        assert_eq!(NetworkSize::from_peer_count(1.0), NetworkSize::Small);
        assert_eq!(NetworkSize::from_peer_count(5.0), NetworkSize::Medium);
        assert_eq!(NetworkSize::from_peer_count(20.0), NetworkSize::Large);
    }

    #[test]
    fn test_network_size_boundary_values() {
        assert_eq!(NetworkSize::from_peer_count(0.0), NetworkSize::Small);
        assert_eq!(NetworkSize::from_peer_count(3.0), NetworkSize::Small);
        assert_eq!(NetworkSize::from_peer_count(4.0), NetworkSize::Medium);
        assert_eq!(NetworkSize::from_peer_count(15.0), NetworkSize::Medium);
        assert_eq!(NetworkSize::from_peer_count(16.0), NetworkSize::Large);
    }

    #[test]
    fn test_network_size_display() {
        assert_eq!(format!("{:?}", NetworkSize::Small), "Small");
        assert_eq!(format!("{:?}", NetworkSize::Medium), "Medium");
        assert_eq!(format!("{:?}", NetworkSize::Large), "Large");
    }

    #[test]
    fn test_network_size_from_peer_count_method() {
        let small = NetworkSize::from_peer_count(3.0);
        let medium = NetworkSize::from_peer_count(10.0);
        let large = NetworkSize::from_peer_count(100.0);
        assert_eq!(small, NetworkSize::Small);
        assert_eq!(medium, NetworkSize::Medium);
        assert_eq!(large, NetworkSize::Large);
    }

    #[test]
    fn test_strip_ansi_codes_empty() {
        let clean = strip_ansi_codes("");
        assert_eq!(clean, "");
    }

    #[test]
    fn test_strip_ansi_codes_no_ansi() {
        let clean = strip_ansi_codes("plain text");
        assert_eq!(clean, "plain text");
    }

    #[test]
    fn test_strip_ansi_codes_multiple_codes() {
        let ansi = "\x1b[1m\x1b[32mBold Green\x1b[0m Normal";
        let clean = strip_ansi_codes(ansi);
        assert_eq!(clean, "Bold Green Normal");
    }

    #[test]
    fn test_strip_ansi_codes_incomplete_sequence() {
        let ansi = "\x1b[32mGreen\x1b[0m incomplete";
        let clean = strip_ansi_codes(ansi);
        assert!(clean.contains("Green"));
        assert!(clean.contains("incomplete"));
    }

    #[test]
    fn test_direct_message_serialization() {
        use serde_json;
        let dm = DirectMessage {
            content: "Hello".to_string(),
            timestamp: 1234567890,
            sent_at: Some(1234567890.5),
        };
        let json = serde_json::to_string(&dm).unwrap();
        assert!(json.contains("Hello"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_broadcast_message_serialization() {
        use serde_json;
        let bm = BroadcastMessage {
            content: "World".to_string(),
            sent_at: Some(1234567890.5),
        };
        let json = serde_json::to_string(&bm).unwrap();
        assert!(json.contains("World"));
    }

    #[test]
    fn test_network_size_equality() {
        assert_eq!(NetworkSize::Small, NetworkSize::Small);
        assert_eq!(NetworkSize::Medium, NetworkSize::Medium);
        assert_eq!(NetworkSize::Large, NetworkSize::Large);
        assert_ne!(NetworkSize::Small, NetworkSize::Medium);
    }

    #[test]
    fn test_network_size_copy() {
        let size = NetworkSize::Medium;
        let copy = size;
        assert_eq!(size, copy);
    }
}
