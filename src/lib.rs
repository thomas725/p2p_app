pub mod models_insertable;
pub mod models_queryable;
pub mod schema;
use crate::schema::identities::dsl::identities;
use crate::schema::messages::dsl::messages;
use crate::schema::peers::dsl::peers;
use crate::{
    models_insertable::{NewIdentity, NewMessage, NewPeer},
    models_queryable::{Identity, Message, Peer},
};
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
    SqliteConnection,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenvy::dotenv;
use serde::{Deserialize, Serialize};
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

pub const DM_PROTOCOL_NAME: &str = "/p2p-chat/dm/1.0.0";

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

pub fn get_libp2p_identity() -> color_eyre::Result<libp2p_identity::Keypair> {
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
    {
        #[cfg(feature = "tracing")]
        tracing::warn!("no valid identity found in database, generating and storing new one");
    }
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
        let conn = test_connection();
        let _ = save_peer("test-peer-1", &["/ip4/127.0.0.1/tcp/4001".to_string()]);
        let peers = load_peers().expect("Failed to load peers");
        assert!(peers.len() >= 1);
    }

    #[test]
    fn test_save_and_load_messages() {
        let conn = test_connection();
        let _ = save_message("Hello world", None, "test-topic", false, None);
        let msgs = load_messages("test-topic", 10).expect("Failed to load messages");
        assert!(msgs.len() >= 1);
    }
}
