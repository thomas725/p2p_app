pub mod models_insertable;
pub mod models_queryable;
pub mod schema;
use crate::schema::identities::dsl::identities;
use crate::{models_queryable::Identity, models_insertable::NewIdentity};
use color_eyre::eyre::{Context, eyre};
use diesel::{
    Connection as _, QueryDsl, RunQueryDsl as _, SelectableHelper as _, SqliteConnection,
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
                Err(e) => tracing::error!("invalid identity stored: {row:?} - {e}"),
            }
        }
    }
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
                Ok(i) => tracing::info!("inserted new identity: {i:?}"),
                Err(e) => tracing::error!("failed to insert identity {i:?}: {e}"),
            }
        }
        Err(e) => tracing::error!("failed to query sqlite for identities: {e}"),
    };
    Ok(keypair)
}
