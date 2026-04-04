pub mod models_insertable;
pub mod models_queryable;
pub mod schema;
use crate::schema::identities::dsl::identities;
use crate::schema::messages::dsl::messages;
use crate::{
    models_insertable::{NewIdentity, NewMessage},
    models_queryable::{Identity, Message},
};
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, ExpressionMethods, QueryDsl, RunQueryDsl as _, SelectableHelper as _,
    SqliteConnection,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenvy::dotenv;
use std::env;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

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
            tracing::error!("failed to query sqlite for identities: {e}");
        }
    };
    Ok(keypair)
}

pub fn save_message(
    content: &str,
    peer_id: Option<&str>,
    topic: &str,
) -> color_eyre::Result<Message> {
    let conn = &mut sqlite_connect()?;
    let new_msg = NewMessage {
        content,
        peer_id,
        topic,
        sent: 0,
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
        .order(schema::messages::created_at.desc())
        .limit(limit as i64)
        .select(Message::as_select())
        .load(conn)?;
    Ok(msgs)
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
    fn test_migrations_run_successfully() {
        let mut conn = test_connection();
        let result = schema::identities::table
            .select(schema::identities::id)
            .load::<i32>(&mut conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_insert_and_retrieve_identity() {
        let mut conn = test_connection();
        let keypair = libp2p_identity::Keypair::generate_ed25519();
        let key = keypair.to_protobuf_encoding().expect("encode keypair");

        diesel::insert_into(schema::identities::table)
            .values(NewIdentity { key: key.clone() })
            .execute(&mut conn)
            .expect("Failed to insert identity");

        let rows = identities
            .select(Identity::as_select())
            .load::<Identity>(&mut conn)
            .expect("Failed to load identities");

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key, key);
    }

    #[test]
    fn test_get_libp2p_identity_generates_new() {
        let database_url = ":memory:";
        let mut conn =
            SqliteConnection::establish(database_url).expect("Failed to create in-memory database");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Failed to run migrations");

        let keypair = libp2p_identity::Keypair::generate_ed25519();
        let key = keypair.to_protobuf_encoding().expect("encode keypair");
        diesel::insert_into(schema::identities::table)
            .values(NewIdentity { key })
            .execute(&mut conn)
            .expect("Failed to insert identity");

        let rows = identities
            .select(Identity::as_select())
            .load::<Identity>(&mut conn)
            .expect("Failed to load identities");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn test_keypair_serialization_roundtrip() {
        let original = libp2p_identity::Keypair::generate_ed25519();
        let encoded = original.to_protobuf_encoding().expect("encode");
        let decoded = libp2p_identity::Keypair::from_protobuf_encoding(&encoded).expect("decode");
        assert_eq!(encoded, decoded.to_protobuf_encoding().expect("re-encode"));
    }
}
