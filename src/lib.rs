use diesel::{Connection as _, SqliteConnection};
use dotenvy::dotenv;
use std::env;

pub fn sqlite_connect() -> SqliteConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").unwrap_or("sqlite.db".to_owned());
    SqliteConnection::establish(&database_url)
        .unwrap_or_else(|e| panic!("Error connecting to {database_url}: {e}"))
}