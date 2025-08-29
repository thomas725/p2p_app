pub mod models_generated;
pub mod models_manual;
pub mod schema;
use crate::schema::identities::dsl::identities;
use crate::{models_generated::Identity, models_manual::NewIdentity};
use diesel::{
    Connection as _, QueryDsl, RunQueryDsl as _, SelectableHelper as _, SqliteConnection,
};
use dotenvy::dotenv;
use std::env;

pub fn sqlite_connect() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or("sqlite.db".to_owned());
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {database_url}: {e}"))
}

pub fn get_libp2p_identity() -> libp2p_identity::Keypair {
    let conn = &mut sqlite_connect();
    if let Ok(rows) = identities.select(Identity::as_select()).load(conn) {
        for row in rows {
            match libp2p_identity::Keypair::from_protobuf_encoding(&row.key) {
                Ok(i) => {
                    return i;
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
            .returning(Identity::as_returning()).get_result(conn) {
                Ok(i) => tracing::info!("inserted new identity: {i:?}"),
                Err(e) => tracing::error!("failed to insert identity {i:?}: {e}"),
            }
        },
        Err(e) => tracing::error!("failed to query sqlite for identities: {e}"),
    };
    keypair
}
