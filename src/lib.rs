pub mod models_insertable;
pub mod models_queryable;
pub mod schema;

#[cfg(feature = "mdns")]
use libp2p::mdns;
use libp2p::{gossipsub, request_response, swarm::NetworkBehaviour};

use std::collections::VecDeque;
use std::sync::OnceLock;

/// Build a tracing `Targets` filter that denies noisy internal modules
/// and keeps useful networking events at DEBUG level.
///
/// Denylist (set to OFF):
/// - `multistream_select` - protocol negotiation internals
/// - `yamux::connection` - stream multiplexing pings/RTT
/// - `libp2p_core::transport::choice` - unreadable type names on dial failure
/// - `libp2p_mdns::behaviour::iface` - startup-only noise
///
/// Kept at DEBUG:
/// - `libp2p_swarm` - connection lifecycle, listener addresses
/// - `libp2p_gossipsub::behaviour` - mesh changes, heartbeats, peer subs
/// - `libp2p_tcp` - dial attempts, listen addresses
/// - `libp2p_quic::transport` - listen addresses
/// - `libp2p_mdns::behaviour` - peer discovery events
///
/// Default level for everything else: WARN
pub fn tracing_filter() -> tracing_subscriber::filter::Targets {
    use tracing_subscriber::filter::{LevelFilter, Targets};
    Targets::new()
        .with_target("multistream_select", LevelFilter::OFF)
        .with_target("yamux::connection", LevelFilter::OFF)
        .with_target("libp2p_core::transport::choice", LevelFilter::OFF)
        .with_target("libp2p_mdns::behaviour::iface", LevelFilter::OFF)
        .with_target("libp2p_swarm", LevelFilter::DEBUG)
        .with_target("libp2p_gossipsub::behaviour", LevelFilter::DEBUG)
        .with_target("libp2p_tcp", LevelFilter::DEBUG)
        .with_target("libp2p_quic::transport", LevelFilter::DEBUG)
        .with_target("libp2p_mdns::behaviour", LevelFilter::DEBUG)
        .with_default(LevelFilter::WARN)
}

pub const CHAT_TOPIC: &str = "test-net";
pub const DM_PROTOCOL_NAME: &str = "/p2p-chat/dm/1.0.0";

pub type P2pAppBehaviour = AppBehaviour;

static LOG_TUI_CALLBACK: OnceLock<Box<dyn Fn(String) + Send + Sync>> = OnceLock::new();
static LOGS: OnceLock<std::sync::Mutex<VecDeque<String>>> = OnceLock::new();

pub fn init_logging() {
    LOGS.get_or_init(|| std::sync::Mutex::new(VecDeque::new()));
}

pub fn set_tui_log_callback<F>(callback: F)
where
    F: Fn(String) + Send + Sync + 'static,
{
    let _ = LOG_TUI_CALLBACK.set(Box::new(callback));
}

pub fn get_tui_logs() -> VecDeque<String> {
    LOGS.get()
        .map(|m| m.lock().unwrap().clone())
        .unwrap_or_default()
}

#[allow(dead_code)]
pub fn p2plog(level: &str, msg: String) {
    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
    let formatted = format!("[{}] [{}] {}", ts, level, msg);

    if let Some(callback) = LOG_TUI_CALLBACK.get() {
        (callback)(formatted.clone());
    }

    if let Some(logs) = LOGS.get() {
        if let Ok(mut l) = logs.lock() {
            l.push_back(formatted.clone());
            if l.len() > 1000 {
                l.pop_front();
            }
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
}

pub type ChatCodec = libp2p_request_response::json::codec::Codec<DirectMessage, DirectMessage>;

pub fn sqlite_connect() -> color_eyre::Result<SqliteConnection> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or("sqlite.db".to_owned());
    let mut conn = SqliteConnection::establish(&database_url)
        .wrap_err_with(|| format!("Error connecting to {database_url}"))?;
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| eyre!(format!("Error executing migrations on {database_url}: {e}")))?;
    Ok(conn)
}

pub fn get_database_url() -> String {
    dotenv().ok();
    env::var("DATABASE_URL").unwrap_or("sqlite.db".to_owned())
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
                let i = NewIdentity { key };
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

pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
    is_direct: bool,
    target_peer: Option<&str>,
) -> color_eyre::Result<Message> {
    let conn = &mut sqlite_connect()?;
    let new_msg = NewMessage {
        content,
        peer_id,
        topic,
        sent: 0,
        is_direct: if is_direct { 1 } else { 0 },
        target_peer,
    };
    let msg = diesel::insert_into(schema::messages::table)
        .values(&new_msg)
        .returning(Message::as_returning())
        .get_result(conn)?;
    Ok(msg)
}

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

pub fn mark_message_sent(id: i32) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::update(schema::messages::table.filter(schema::messages::id.eq(id)))
        .set(schema::messages::sent.eq(1))
        .execute(conn)?;
    Ok(())
}

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

pub fn save_peer(peer_id: &str, addresses: &[String]) -> color_eyre::Result<Peer> {
    let conn = &mut sqlite_connect()?;
    let addresses_str = addresses.join(",");
    let now = chrono::Utc::now().naive_utc();

    let new_peer = NewPeer {
        peer_id,
        addresses: &addresses_str,
        first_seen: now,
        last_seen: now,
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

pub fn load_peers() -> color_eyre::Result<Vec<Peer>> {
    let conn = &mut sqlite_connect()?;
    let peers_list = peers
        .order(schema::peers::last_seen.desc())
        .select(Peer::as_select())
        .load(conn)?;
    Ok(peers_list)
}

pub fn remove_peer(peer_id: &str) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    diesel::delete(schema::peers::table.filter(schema::peers::peer_id.eq(peer_id)))
        .execute(conn)?;
    Ok(())
}

pub fn save_peer_session(concurrent_peers: i32) -> color_eyre::Result<()> {
    let conn = &mut sqlite_connect()?;
    let new_session = NewPeerSession { concurrent_peers };
    diesel::insert_into(schema::peer_sessions::table)
        .values(&new_session)
        .execute(conn)?;
    Ok(())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSize {
    Small,
    Medium,
    Large,
}

impl NetworkSize {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_connection() -> SqliteConnection {
        let mut conn =
            SqliteConnection::establish(":memory:").expect("Failed to create in-memory database");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");
        conn
    }

    #[test]
    fn test_save_and_load_peer() {
        let _conn = test_connection();
        let _ = save_peer("test-peer-1", &["/ip4/127.0.0.1/tcp/4001".to_string()]);
        let loaded_peers = load_peers().expect("Failed to load peers");
        assert!(loaded_peers.len() >= 1);
    }

    #[ignore]
    #[test]
    fn test_save_and_load_messages() {
        let _conn = &mut test_connection();
        let _ = save_message("Hello world", None, "test-topic", false, None);
        let loaded_msgs = load_messages("test-topic", 10).expect("Failed to load messages");
        assert!(loaded_msgs.len() >= 1);
    }
}
